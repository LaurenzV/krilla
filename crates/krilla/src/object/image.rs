//! Creating and using bitmap images.
//!
//! krilla allows you to add bitmap images to your PDF very easily.
//! The currently supported formats include
//! - PNG
//! - JPG
//! - GIF
//! - WEBP

// TODO: CLean up and update docs
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::DerefMut;
use std::sync::Arc;

use pdf_writer::{Chunk, Finish, Name, Ref};
use zune_jpeg::zune_core::result::DecodingResult;
use zune_jpeg::JpegDecoder;
use zune_png::zune_core::colorspace::ColorSpace;
use zune_png::PngDecoder;

use crate::color::{GenericICCProfile, ICCBasedColorSpace, ICCProfile, DEVICE_CMYK, DEVICE_RGB};
use crate::configure::ValidationError;
use crate::error::{KrillaError, KrillaResult};
use crate::object::color::DEVICE_GRAY;
use crate::serialize::SerializeContext;
use crate::stream::{deflate_encode, FilterStreamBuilder};
use crate::util::{Deferred, NameExt, SipHashable};
use crate::Data;

/// The number of buits per color component.
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum BitsPerComponent {
    /// Eight bits per component.
    Eight,
    /// Sixteen bits per component.
    Sixteen,
}

impl BitsPerComponent {
    fn as_u8(&self) -> u8 {
        match self {
            BitsPerComponent::Eight => 8,
            BitsPerComponent::Sixteen => 16,
        }
    }
}

/// The color space of the image.
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum ImageColorspace {
    /// The RGB color space.
    Rgb,
    /// The luma color space.
    Luma,
    /// The CMYK color space.
    Cmyk,
}

impl ImageColorspace {
    fn num_components(&self) -> u8 {
        match self {
            ImageColorspace::Luma => 1,
            ImageColorspace::Rgb => 3,
            ImageColorspace::Cmyk => 4,
        }
    }
}

impl TryFrom<ColorSpace> for ImageColorspace {
    type Error = ();

    fn try_from(value: ColorSpace) -> Result<Self, Self::Error> {
        match value {
            ColorSpace::RGB => Ok(ImageColorspace::Rgb),
            ColorSpace::RGBA => Ok(ImageColorspace::Rgb),
            ColorSpace::YCbCr => Ok(ImageColorspace::Rgb),
            ColorSpace::Luma => Ok(ImageColorspace::Luma),
            ColorSpace::LumaA => Ok(ImageColorspace::Luma),
            ColorSpace::YCCK => Ok(ImageColorspace::Cmyk),
            ColorSpace::CMYK => Ok(ImageColorspace::Cmyk),
            _ => Err(()),
        }
    }
}

struct SampledRepr {
    color_channel: Vec<u8>,
    alpha_channel: Option<Vec<u8>>,
    bits_per_component: BitsPerComponent,
}

struct JpegRepr {
    data: Data,
    bits_per_component: BitsPerComponent,
    invert_cmyk: bool,
}

enum Repr {
    Sampled(SampledRepr),
    Jpeg(JpegRepr),
}

impl Repr {
    fn bits_per_component(&self) -> BitsPerComponent {
        match self {
            Repr::Sampled(s) => s.bits_per_component,
            Repr::Jpeg(j) => j.bits_per_component,
        }
    }
}

/// A trait for custom images, which you can use if the
/// current methods provided by krilla (JPEG/PNG/WEBP/GIF) images
/// are not suitable for your own purpose.
///
/// Note that a struct implementing this trait should be cheap to
/// hash and clone, otherwise performance might be bad!
pub trait CustomImage: Hash + Clone + Send + Sync + 'static {
    /// Return the raw bytes of the color channel.
    fn color_channel(&self) -> &[u8];
    /// Return the raw bytes of the alpha channel, if available.
    fn alpha_channel(&self) -> Option<&[u8]>;
    /// Return the bits per component of the image.
    fn bits_per_component(&self) -> BitsPerComponent;
    /// Return the dimensions of the image.
    fn size(&self) -> (u32, u32);
    /// Return the ICC profile of the image, is available.
    fn icc_profile(&self) -> Option<&[u8]>;
    /// Return the color space of the image.
    fn color_space(&self) -> ImageColorspace;
}

struct ImageMetadata {
    size: (u32, u32),
    color_space: ImageColorspace,
    has_alpha: bool,
    icc: Option<GenericICCProfile>,
}

struct ImageRepr {
    inner: Deferred<Option<Repr>>,
    metadata: ImageMetadata,
    sip: u128,
    interpolate: bool,
}

impl ImageRepr {
    fn size(&self) -> (u32, u32) {
        self.metadata.size
    }

    fn icc(&self) -> Option<GenericICCProfile> {
        self.metadata.icc.clone()
    }

    fn color_space(&self) -> ImageColorspace {
        self.metadata.color_space
    }
}

impl Debug for ImageRepr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ImageRepr {{..}}")
    }
}

impl Hash for ImageRepr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.sip.hash(state);
    }
}

impl PartialEq for ImageRepr {
    fn eq(&self, other: &Self) -> bool {
        self.sip == other.sip
    }
}

impl Eq for ImageRepr {}

/// A bitmap image.
///
/// This type is cheap to hash and clone, but expensive to create.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Image(Arc<ImageRepr>);

fn get_icc_profile_type(data: &[u8], color_space: ImageColorspace) -> Option<GenericICCProfile> {
    let wrapper = match color_space {
        ImageColorspace::Rgb => GenericICCProfile::Rgb(ICCProfile::new(data)?),
        ImageColorspace::Luma => GenericICCProfile::Luma(ICCProfile::new(data)?),
        ImageColorspace::Cmyk => GenericICCProfile::Cmyk(ICCProfile::new(data)?),
    };

    Some(wrapper)
}

impl Image {
    /// Create a new bitmap image from a `.png` file.
    ///
    /// Returns `None` if krilla was unable to parse the file.
    pub fn from_png(data: Data, interpolate: bool) -> Option<Image> {
        let hash = data.as_ref().sip_hash();
        let metadata = png_metadata(data.as_ref())?;

        Some(Self(Arc::new(ImageRepr {
            inner: Deferred::new(move || decode_png(data.as_ref())),
            metadata,
            sip: hash,
            interpolate,
        })))
    }

    /// Create a new bitmap image from a `.jpg` file.
    ///
    /// Returns `None` if krilla was unable to parse the file.
    pub fn from_jpeg(data: Data, interpolate: bool) -> Option<Image> {
        let hash = data.as_ref().sip_hash();
        let metadata = jpeg_metadata(data.as_ref())?;

        Some(Self(Arc::new(ImageRepr {
            inner: Deferred::new(move || decode_jpeg(data)),
            metadata,
            sip: hash,
            interpolate,
        })))
    }

    /// Create a new bitmap image from a `.gif` file.
    ///
    /// Returns `None` if krilla was unable to parse the file.
    pub fn from_gif(data: Data, interpolate: bool) -> Option<Image> {
        let hash = data.as_ref().sip_hash();
        let metadata = gif_metadata(data.as_ref())?;

        Some(Self(Arc::new(ImageRepr {
            inner: Deferred::new(move || decode_gif(data)),
            metadata,
            sip: hash,
            interpolate,
        })))
    }

    /// Create a new bitmap image from a `.webp` file.
    ///
    /// Returns `None` if krilla was unable to parse the file.
    pub fn from_webp(data: Data, interpolate: bool) -> Option<Image> {
        let hash = data.as_ref().sip_hash();
        let metadata = webp_metadata(data.as_ref())?;

        Some(Self(Arc::new(ImageRepr {
            inner: Deferred::new(move || decode_webp(data)),
            metadata,
            sip: hash,
            interpolate,
        })))
    }

    /// Create a new image from a custom image.
    ///
    /// Panics if the dimensions of the image and the length of the
    /// data doesn't match.
    pub fn from_custom<T: CustomImage>(image: T, interpolate: bool) -> Option<Image> {
        let hash = image.sip_hash();
        let metadata = ImageMetadata {
            size: image.size(),
            color_space: image.color_space(),
            has_alpha: image.alpha_channel().is_some(),
            icc: image
                .icc_profile()
                .and_then(|d| get_icc_profile_type(d, image.color_space())),
        };

        Some(Self(Arc::new(ImageRepr {
            inner: Deferred::new(move || {
                let bytes_per_component = (image.bits_per_component().as_u8() / 8) as u32;
                let color_channel_len = bytes_per_component
                    * image.color_space().num_components() as u32
                    * metadata.size.0
                    * metadata.size.1;
                let color_channel = image.color_channel();
                assert_eq!(color_channel.len(), color_channel_len as usize);

                let alpha_channel_len = bytes_per_component * metadata.size.0 * metadata.size.1;
                let alpha_channel = image.alpha_channel();
                if let Some(alpha_channel) = alpha_channel {
                    assert_eq!(alpha_channel.len(), alpha_channel_len as usize);
                }

                Some(Repr::Sampled(SampledRepr {
                    color_channel: deflate_encode(color_channel),
                    alpha_channel: image.alpha_channel().map(deflate_encode),
                    bits_per_component: image.bits_per_component(),
                }))
            }),
            metadata,
            sip: hash,
            interpolate,
        })))
    }

    // Used for SVG filters
    pub(crate) fn from_rgba8(data: Vec<u8>, width: u32, height: u32) -> Self {
        let hash = data.sip_hash();
        let metadata = ImageMetadata {
            has_alpha: true,
            size: (width, height),
            color_space: ImageColorspace::Rgb,
            icc: None,
        };

        Self(Arc::new(ImageRepr {
            inner: Deferred::new(move || {
                let (color_channel, alpha_channel, bits_per_component) =
                    handle_u8_image(&data, ColorSpace::RGBA);

                Some(Repr::Sampled(SampledRepr {
                    color_channel,
                    alpha_channel,
                    bits_per_component,
                }))
            }),
            metadata,
            sip: hash,
            interpolate: false,
        }))
    }

    /// Return the size of the image.
    pub fn size(&self) -> (u32, u32) {
        self.0.size()
    }

    fn icc(&self) -> Option<GenericICCProfile> {
        self.0.icc()
    }

    fn color_space(&self) -> ImageColorspace {
        self.0.color_space()
    }

    pub(crate) fn serialize(
        self,
        sc: &mut SerializeContext,
        root_ref: Ref,
    ) -> Deferred<KrillaResult<Chunk>> {
        let soft_mask_id = self.0.metadata.has_alpha.then(|| {
            sc.register_validation_error(ValidationError::Transparency(sc.location));
            sc.new_ref()
        });
        let icc_ref = self.icc().and_then(|ic| {
            if sc
                .serialize_settings()
                .pdf_version()
                .supports_icc(ic.metadata())
            {
                let ref_ = match ic {
                    GenericICCProfile::Luma(l) => sc.register_cacheable(ICCBasedColorSpace(l)),
                    GenericICCProfile::Rgb(r) => sc.register_cacheable(ICCBasedColorSpace(r)),
                    GenericICCProfile::Cmyk(c) => sc.register_cacheable(ICCBasedColorSpace(c)),
                };

                Some(ref_)
            } else {
                // Don't embed ICC profiles from images if the current
                // PDF version does not support it.
                None
            }
        });

        if self.0.interpolate {
            sc.register_validation_error(ValidationError::ImageInterpolation(sc.location));
        }

        let serialize_settings = sc.serialize_settings().clone();

        Deferred::new(move || {
            let mut chunk = Chunk::new();

            let repr = self
                .0
                .inner
                .wait()
                .as_ref()
                .ok_or(KrillaError::ImageError(self.clone()))?;

            let alpha_mask = match repr {
                Repr::Sampled(sampled) => sampled.alpha_channel.as_ref().map(|mask_data| {
                    let soft_mask_id = soft_mask_id.unwrap();
                    let mask_stream = FilterStreamBuilder::new_from_deflated(mask_data)
                        .finish(&serialize_settings);
                    let mut s_mask = chunk.image_xobject(soft_mask_id, mask_stream.encoded_data());
                    mask_stream.write_filters(s_mask.deref_mut().deref_mut());
                    s_mask.width(self.size().0 as i32);
                    s_mask.height(self.size().1 as i32);
                    s_mask.pair(
                        Name(b"ColorSpace"),
                        // Mask color space must be device gray -- see Table 145.
                        DEVICE_GRAY.to_pdf_name(),
                    );

                    if self.0.interpolate {
                        s_mask.interpolate(true);
                    }

                    s_mask.bits_per_component(repr.bits_per_component().as_u8() as i32);
                    soft_mask_id
                }),
                Repr::Jpeg(_) => None,
            };

            let filter_stream = match repr {
                Repr::Sampled(s) => FilterStreamBuilder::new_from_deflated(&s.color_channel)
                    .finish(&serialize_settings),
                Repr::Jpeg(j) => FilterStreamBuilder::new_from_jpeg_data(j.data.as_ref())
                    .finish(&serialize_settings),
            };

            let mut image_x_object = chunk.image_xobject(root_ref, filter_stream.encoded_data());
            filter_stream.write_filters(image_x_object.deref_mut().deref_mut());
            image_x_object.width(self.size().0 as i32);
            image_x_object.height(self.size().1 as i32);

            if let Some(icc_ref) = icc_ref {
                image_x_object.pair(Name(b"ColorSpace"), icc_ref);
            } else {
                let name = match self.color_space() {
                    ImageColorspace::Rgb => DEVICE_RGB.to_pdf_name(),
                    ImageColorspace::Luma => DEVICE_GRAY.to_pdf_name(),
                    ImageColorspace::Cmyk => DEVICE_CMYK.to_pdf_name(),
                };

                image_x_object.pair(Name(b"ColorSpace"), name);
            }

            if self.0.interpolate {
                image_x_object.interpolate(true);
            }

            // Photoshop CMYK images need to be inverted, see
            // https://github.com/sile-typesetter/libtexpdf/blob/1891bee5e0b73165e4a259f910d3ea3fe1df0b42/jpegimage.c#L25-L51
            // I'm not sure if this applies to all JPEG CMYK images out there, but for now we just
            // always do it. In libtexpdf, they only seem to do it if they can find the Adobe APP
            // marker.
            if let Repr::Jpeg(j) = repr {
                if j.invert_cmyk {
                    image_x_object
                        .decode([1.0, 0.0].repeat(self.color_space().num_components() as usize));
                }
            }

            image_x_object.bits_per_component(repr.bits_per_component().as_u8() as i32);
            if let Some(soft_mask_id) = alpha_mask {
                image_x_object.s_mask(soft_mask_id);
            }
            image_x_object.finish();

            Ok(chunk)
        })
    }
}

fn png_metadata(data: &[u8]) -> Option<ImageMetadata> {
    let mut decoder = PngDecoder::new(data);
    decoder.decode_headers().ok()?;

    let size = {
        let info = decoder.get_info()?;
        (info.width as u32, info.height as u32)
    };
    let color_space = decoder.get_colorspace()?;
    let image_color_space = color_space.try_into().ok()?;
    let icc = decoder
        .get_info()?
        .icc_profile
        .as_ref()
        .and_then(|d| get_icc_profile_type(d, image_color_space));

    Some(ImageMetadata {
        has_alpha: color_space.has_alpha(),
        size,
        color_space: image_color_space,
        icc,
    })
}

fn decode_png(data: &[u8]) -> Option<Repr> {
    let mut decoder = PngDecoder::new(data);
    decoder.decode_headers().ok()?;

    let color_space = decoder.get_colorspace()?;

    let decoded = decoder.decode().ok()?;

    let (color_channel, alpha_channel, bits_per_component) = match decoded {
        DecodingResult::U8(u8) => handle_u8_image(&u8, color_space),
        DecodingResult::U16(u16) => handle_u16_image(&u16, color_space),
        _ => return None,
    };

    Some(Repr::Sampled(SampledRepr {
        color_channel,
        alpha_channel,
        bits_per_component,
    }))
}

fn jpeg_metadata(data: &[u8]) -> Option<ImageMetadata> {
    let mut decoder = JpegDecoder::new(data);
    decoder.decode_headers().ok()?;

    let size = {
        let dimensions = decoder.dimensions()?;
        (dimensions.0 as u32, dimensions.1 as u32)
    };

    let input_color_space = decoder.get_input_colorspace()?;
    let image_color_space = input_color_space.try_into().ok()?;

    let icc = decoder
        .icc_profile()
        .and_then(|d| get_icc_profile_type(&d, image_color_space));

    Some(ImageMetadata {
        has_alpha: false,
        size,
        color_space: image_color_space,
        icc,
    })
}

fn decode_jpeg(data: Data) -> Option<Repr> {
    let mut decoder = JpegDecoder::new(data.as_ref());
    decoder.decode_headers().ok()?;

    let input_color_space = decoder.get_input_colorspace()?;

    if matches!(
        input_color_space,
        ColorSpace::Luma
            | ColorSpace::YCbCr
            | ColorSpace::RGB
            | ColorSpace::CMYK
            | ColorSpace::YCCK
    ) {
        Some(Repr::Jpeg(JpegRepr {
            data,
            bits_per_component: BitsPerComponent::Eight,
            invert_cmyk: matches!(input_color_space, ColorSpace::YCCK | ColorSpace::CMYK),
        }))
    } else {
        // JPEGs shouldn't be able to have a different color space?
        None
    }
}

fn decode_gif(data: Data) -> Option<Repr> {
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = decoder.read_info(data.as_ref()).ok()?;
    let first_frame = decoder.read_next_frame().ok()??;

    let (color_channel, alpha_channel, bits_per_component) =
        handle_u8_image(first_frame.buffer.as_ref(), ColorSpace::RGBA);

    Some(Repr::Sampled(SampledRepr {
        color_channel,
        alpha_channel,
        bits_per_component,
    }))
}

fn gif_metadata(data: &[u8]) -> Option<ImageMetadata> {
    let size = imagesize::blob_size(data).ok()?;

    Some(ImageMetadata {
        // We always decode GIFs using RGBA, see `decode_gif`
        has_alpha: true,
        size: (size.width as u32, size.height as u32),
        color_space: ImageColorspace::Rgb,
        icc: None,
    })
}

fn webp_metadata(data: &[u8]) -> Option<ImageMetadata> {
    let mut decoder = image_webp::WebPDecoder::new(std::io::Cursor::new(data)).ok()?;
    let size = decoder.dimensions();
    let color_space = ImageColorspace::Rgb;
    let icc = decoder
        .icc_profile()
        .ok()?
        .and_then(|d| get_icc_profile_type(&d, color_space));

    Some(ImageMetadata {
        has_alpha: decoder.has_alpha(),
        size,
        color_space,
        icc,
    })
}

fn decode_webp(data: Data) -> Option<Repr> {
    let mut decoder = image_webp::WebPDecoder::new(std::io::Cursor::new(data.as_ref())).ok()?;
    let mut first_frame = vec![0; decoder.output_buffer_size()?];
    decoder.read_image(&mut first_frame).ok()?;

    let color_space = if decoder.has_alpha() {
        ColorSpace::RGBA
    } else {
        ColorSpace::RGB
    };

    let (color_channel, alpha_channel, bits_per_component) =
        handle_u8_image(&first_frame, color_space);

    Some(Repr::Sampled(SampledRepr {
        color_channel,
        alpha_channel,
        bits_per_component,
    }))
}

fn handle_u8_image(data: &[u8], cs: ColorSpace) -> (Vec<u8>, Option<Vec<u8>>, BitsPerComponent) {
    let mut alphas = if cs.has_alpha() {
        Vec::with_capacity(data.len() / cs.num_components())
    } else {
        Vec::new()
    };

    let color_channel = match cs {
        ColorSpace::RGB => deflate_encode(data),
        ColorSpace::RGBA => {
            let data = data
                .iter()
                .enumerate()
                .flat_map(|(index, val)| {
                    if index % 4 == 3 {
                        alphas.push(*val);
                        None
                    } else {
                        Some(*val)
                    }
                })
                .collect::<Vec<_>>();
            deflate_encode(&data)
        }
        ColorSpace::Luma => deflate_encode(data),
        ColorSpace::LumaA => {
            let data = data
                .iter()
                .enumerate()
                .flat_map(|(index, val)| {
                    if index % 2 == 1 {
                        alphas.push(*val);
                        None
                    } else {
                        Some(*val)
                    }
                })
                .collect::<Vec<_>>();
            deflate_encode(&data)
        }
        // PNG/WEBP/GIF only support those three, so should be enough?
        _ => unimplemented!(),
    };

    let alpha_channel = if !alphas.is_empty() && alphas.iter().any(|v| *v != 255) {
        Some(deflate_encode(&alphas))
    } else {
        None
    };

    (color_channel, alpha_channel, BitsPerComponent::Eight)
}

fn handle_u16_image(data: &[u16], cs: ColorSpace) -> (Vec<u8>, Option<Vec<u8>>, BitsPerComponent) {
    let mut alphas = if cs.has_alpha() {
        // * 2 because we are going from u16 to u8
        Vec::with_capacity(2 * data.len() / cs.num_components())
    } else {
        Vec::new()
    };

    let encoded_image = match cs {
        ColorSpace::RGB => {
            let data = data
                .iter()
                .flat_map(|b| b.to_be_bytes())
                .collect::<Vec<_>>();
            deflate_encode(&data)
        }
        ColorSpace::RGBA => {
            let data = data
                .iter()
                .enumerate()
                .flat_map(|(index, val)| {
                    if index % 4 == 3 {
                        alphas.extend(val.to_be_bytes());
                        None
                    } else {
                        Some(*val)
                    }
                })
                .flat_map(|n| n.to_be_bytes())
                .collect::<Vec<_>>();
            deflate_encode(&data)
        }
        ColorSpace::Luma => {
            let data = data
                .iter()
                .flat_map(|b| b.to_be_bytes())
                .collect::<Vec<_>>();
            deflate_encode(&data)
        }
        ColorSpace::LumaA => {
            let data = data
                .iter()
                .enumerate()
                .flat_map(|(index, val)| {
                    if index % 2 == 1 {
                        alphas.extend(val.to_be_bytes());
                        None
                    } else {
                        Some(*val)
                    }
                })
                .flat_map(|n| n.to_be_bytes())
                .collect::<Vec<_>>();
            deflate_encode(&data)
        }
        // PNG/WEBP/GIF only support those three, so should be enough?
        _ => unimplemented!(),
    };

    let encoded_mask = if !alphas.is_empty() {
        Some(deflate_encode(&alphas))
    } else {
        None
    };

    (encoded_image, encoded_mask, BitsPerComponent::Sixteen)
}

#[cfg(test)]
mod tests {
    use krilla_macros::{snapshot, visreg};
    use tiny_skia_path::Size;

    use crate::image::Image;
    use crate::page::Page;
    use crate::serialize::SerializeContext;
    use crate::surface::Surface;
    use crate::tests::{
        load_custom_image, load_custom_image_with_icc, load_gif_image, load_jpg_image,
        load_png_image, load_webp_image, ASSETS_PATH,
    };
    use crate::Document;

    #[snapshot]
    fn image_luma8_png(sc: &mut SerializeContext) {
        sc.register_image(load_png_image("luma8.png"));
    }

    #[snapshot]
    fn image_custom_luma8_png(sc: &mut SerializeContext) {
        sc.register_image(load_custom_image("luma8.png"));
    }

    #[snapshot]
    fn image_luma16_png(sc: &mut SerializeContext) {
        sc.register_image(load_png_image("luma16.png"));
    }

    #[snapshot]
    fn image_rgb8_png(sc: &mut SerializeContext) {
        sc.register_image(load_png_image("rgb8.png"));
    }

    #[snapshot]
    fn image_custom_rgb8_png(sc: &mut SerializeContext) {
        sc.register_image(load_custom_image("rgb8.png"));
    }

    // ICC profile should be ignored.
    #[snapshot]
    fn image_custom_rgb8_png_invalid_icc(sc: &mut SerializeContext) {
        sc.register_image(load_custom_image_with_icc(
            "rgb8.png",
            std::fs::read(crate::tests::ASSETS_PATH.join("icc/eciCMYK_v2.icc")).unwrap(),
        ));
    }

    #[snapshot]
    fn image_rgb16_png(sc: &mut SerializeContext) {
        sc.register_image(load_png_image("rgb16.png"));
    }

    #[snapshot]
    fn image_rgba8_png(sc: &mut SerializeContext) {
        sc.register_image(load_png_image("rgba8.png"));
    }

    #[snapshot]
    fn image_custom_rgba8_png(sc: &mut SerializeContext) {
        sc.register_image(load_custom_image("rgba8.png"));
    }

    #[snapshot]
    fn image_rgba16_png(sc: &mut SerializeContext) {
        sc.register_image(load_png_image("rgba16.png"));
    }

    #[snapshot]
    fn image_luma8_jpg(sc: &mut SerializeContext) {
        sc.register_image(load_jpg_image("luma8.jpg"));
    }

    #[snapshot]
    fn image_rgb8_jpg(sc: &mut SerializeContext) {
        sc.register_image(load_jpg_image("rgb8.jpg"));
    }

    #[snapshot]
    fn image_cmyk_jpg(sc: &mut SerializeContext) {
        sc.register_image(load_jpg_image("cmyk.jpg"));
    }

    // Currently gets converted into RGBA.
    #[snapshot]
    fn image_rgb8_gif(sc: &mut SerializeContext) {
        sc.register_image(load_gif_image("rgb8.gif"));
    }

    #[snapshot]
    fn image_rgba8_gif(sc: &mut SerializeContext) {
        sc.register_image(load_gif_image("rgba8.gif"));
    }
    #[snapshot]
    fn image_rgba8_webp(sc: &mut SerializeContext) {
        sc.register_image(load_webp_image("rgba8.webp"));
    }
}
