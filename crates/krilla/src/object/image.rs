//! Creating and using bitmap images.
//!
//! krilla allows you to add bitmap images to your PDF very easily.
//! The currently supported formats include
//! - PNG
//! - JPG
//! - GIF
//! - WEBP

use crate::color::{ICCBasedColorSpace, ICCProfile, ICCProfileWrapper, DEVICE_CMYK, DEVICE_RGB};
use crate::object::color::DEVICE_GRAY;
use crate::resource::RegisterableResource;
use crate::serialize::SerializerContext;
use crate::stream::FilterStream;
use crate::util::{Deferred, NameExt, OptionDeferred, Prehashed, SizeWrapper};
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tiny_skia_path::{IntSize, Size};
use zune_jpeg::JpegDecoder;
use zune_png::zune_core::colorspace::ColorSpace;
use zune_png::zune_core::result::DecodingResult;
use zune_png::PngDecoder;

/// A bitmap image.
///
/// This type is cheap to hash and clone, but expensive to create.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Image(Arc<Prehashed<ImageRepr>>);

/// The size of an image.
#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub struct ImageSize(u32, u32);

impl ImageSize {
    /// Create a new image size.
    pub fn new(width: u32, height: u32) -> Self {
        Self(width, height)
    }

    /// Return the width.
    pub fn width(&self) -> u32 {
        self.0
    }

    /// Return the height.
    pub fn height(&self) -> u32 {
        self.1
    }

    pub(crate) fn to_tiny_skia(&self) -> tiny_skia_path::Size {
        Size::from_wh(self.0 as f32, self.1 as f32).unwrap()
    }
}

/// The format of an image.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum ImageType {
    /// The PNG format.
    Png,
    /// The JPG format.
    Jpeg,
    /// The WEBP format.
    Webp,
    /// The GIF format.
    Gif,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
struct ImageRepr {
    data: Arc<Vec<u8>>,
    image_type: ImageType,
    size: ImageSize,
}

impl Image {
    /// Create a new bitmap image. Currently, only PNG, JPG, GIF and WEBP are supported.
    ///
    /// Returns `None` if the image is not supported or could not be read.
    pub fn new(data: Arc<Vec<u8>>) -> Option<Self> {
        let image_type = match imagesize::image_type(&data).ok()? {
            imagesize::ImageType::Gif => ImageType::Gif,
            imagesize::ImageType::Jpeg => ImageType::Jpeg,
            imagesize::ImageType::Png => ImageType::Png,
            imagesize::ImageType::Webp => ImageType::Webp,
            _ => return None,
        };
        let raw_size = imagesize::blob_size(&data).ok()?;
        let size = ImageSize::new(raw_size.width as u32, raw_size.height as u32);
        Some(Self(Arc::new(Prehashed::new(ImageRepr {
            data,
            image_type,
            size,
        }))))
    }

    fn png_repr(&self) -> Option<Repr> {
        let mut decoder = PngDecoder::new(self.0.data.as_ref());
        decoder.decode_headers().ok()?;

        let color_space = decoder.get_colorspace()?;
        let image_color_space = color_space.try_into().ok()?;
        let decoded = decoder.decode().ok()?;

        let (image_data, mask_data, bits_per_component) = match decoded {
            DecodingResult::U8(u8) => handle_u8_image(u8, color_space),
            DecodingResult::U16(u16) => handle_u16_image(u16, color_space),
            _ => return None,
        };

        let icc = decoder
            .get_info()?
            .icc_profile
            .as_ref()
            .and_then(|d| get_icc_profile_type(d.clone(), image_color_space));

        Some(Repr::Sampled(SampledRepr {
            image_data,
            icc,
            mask_data,
            bits_per_component,
            image_color_space,
            size: self.size(),
        }))
    }

    fn jpeg_repr(&self) -> Option<Repr> {
        let mut decoder = JpegDecoder::new(self.0.data.as_ref());
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
            // Don't decode the image and save it with the existing DCT encoding.
            let image_color_space = input_color_space.try_into().ok()?;
            let icc = decoder
                .icc_profile()
                .and_then(|d| get_icc_profile_type(d.clone(), image_color_space));

            Some(Repr::Jpeg(JpegRepr {
                icc,
                data: self.0.data.clone(),
                size: self.size(),
                bits_per_component: BitsPerComponent::Eight,
                image_color_space,
                invert_cmyk: matches!(input_color_space, ColorSpace::YCCK | ColorSpace::CMYK),
            }))
        } else {
            // Unknown color space, fall back to decoding the JPEG into RGB8 and saving
            // it in the sampled representation.
            let output_color_space = decoder.get_output_colorspace()?;
            let image_color_space = output_color_space.try_into().ok()?;

            let decoded = decoder.decode().ok()?;
            let (image_data, _, bits_per_component) = handle_u8_image(decoded, output_color_space);

            Some(Repr::Sampled(SampledRepr {
                // Cannot embed ICC profile in this case, since we are converting to RGB.
                icc: None,
                image_data,
                mask_data: None,
                bits_per_component,
                image_color_space,
                size: self.size(),
            }))
        }
    }

    fn gif_repr(&self) -> Option<Repr> {
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = decoder.read_info(self.0.data.as_slice()).ok()?;
        let first_frame = decoder.read_next_frame().ok()??;

        let (image_data, mask_data, bits_per_component) =
            handle_u8_image(first_frame.buffer.to_vec(), ColorSpace::RGBA);

        Some(Repr::Sampled(SampledRepr {
            icc: None,
            image_data,
            mask_data,
            bits_per_component,
            image_color_space: ImageColorspace::Rgb,
            size: self.size(),
        }))
    }

    /// Create a new bitmap image from a `.webp` file.
    ///
    /// Returns `None` if krilla was unable to parse the file.
    fn webp_repr(&self) -> Option<Repr> {
        let mut decoder =
            image_webp::WebPDecoder::new(std::io::Cursor::new(self.0.data.as_slice())).ok()?;
        let mut first_frame = vec![0; decoder.output_buffer_size()?];
        decoder.read_image(&mut first_frame).ok()?;
        let color_space = if decoder.has_alpha() {
            ColorSpace::RGBA
        } else {
            ColorSpace::RGB
        };
        let image_color_space = color_space.try_into().ok()?;

        let icc = decoder
            .icc_profile()
            .ok()?
            .and_then(|d| get_icc_profile_type(d.clone(), image_color_space));

        let (image_data, mask_data, bits_per_component) = handle_u8_image(first_frame, color_space);

        Some(Repr::Sampled(SampledRepr {
            icc,
            image_data,
            mask_data,
            bits_per_component,
            image_color_space,
            size: self.size(),
        }))
    }

    /// Returns the dimensions of the image.
    pub fn size(&self) -> ImageSize {
        self.0.size
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
enum BitsPerComponent {
    Eight,
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

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
enum ImageColorspace {
    Rgb,
    Luma,
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

/// Representation of a raw, decoded image.
#[derive(Debug, Hash, Eq, PartialEq)]
struct SampledRepr {
    image_data: Vec<u8>,
    icc: Option<ICCProfileWrapper>,
    size: ImageSize,
    mask_data: Option<Vec<u8>>,
    bits_per_component: BitsPerComponent,
    image_color_space: ImageColorspace,
}

/// Representation of an encoded jpg image.
#[derive(Debug, Hash, Eq, PartialEq)]
struct JpegRepr {
    data: Arc<Vec<u8>>,
    icc: Option<ICCProfileWrapper>,
    size: ImageSize,
    bits_per_component: BitsPerComponent,
    image_color_space: ImageColorspace,
    invert_cmyk: bool,
}

#[derive(Debug, Hash, Eq, PartialEq)]
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

    fn icc(&self) -> Option<ICCProfileWrapper> {
        match self {
            Repr::Sampled(s) => s.icc.clone(),
            Repr::Jpeg(j) => j.icc.clone(),
        }
    }

    fn color_space(&self) -> ImageColorspace {
        match self {
            Repr::Sampled(s) => s.image_color_space,
            Repr::Jpeg(j) => j.image_color_space,
        }
    }
}

impl Image {
    // TODO: Replace Option with error
    pub(crate) fn serialize(
        self,
        sc: &mut SerializerContext,
        root_ref: Ref,
    ) -> OptionDeferred<Chunk> {
        // We eagerly allocate new Refs, because we cannot pass the Serializer into the deferred
        // closure. It's not issue if they don't end up being used, because the chunk container
        // will renumber everything, anyway.
        let soft_mask_id = sc.new_ref();
        let icc_ref = sc.new_ref();
        let cs_ref = sc.new_ref();
        let serialize_settings = sc.serialize_settings().clone();

        OptionDeferred::new(move || {
            let mut chunk = Chunk::new();
            let repr = match self.0.image_type {
                ImageType::Png => self.png_repr(),
                ImageType::Jpeg => self.jpeg_repr(),
                ImageType::Webp => self.webp_repr(),
                ImageType::Gif => self.gif_repr(),
            }?;

            let alpha_mask = match &repr {
                Repr::Sampled(sampled) => sampled.mask_data.as_ref().map(|mask_data| {
                    let soft_mask_id = soft_mask_id;
                    let mask_stream =
                        FilterStream::new_from_binary_data(mask_data, &serialize_settings);
                    let mut s_mask = chunk.image_xobject(soft_mask_id, mask_stream.encoded_data());
                    mask_stream.write_filters(s_mask.deref_mut().deref_mut());
                    s_mask.width(self.size().width() as i32);
                    s_mask.height(self.size().height() as i32);
                    s_mask.pair(
                        Name(b"ColorSpace"),
                        // Mask color space must be device gray -- see Table 145.
                        DEVICE_GRAY.to_pdf_name(),
                    );
                    s_mask.bits_per_component(sampled.bits_per_component.as_u8() as i32);
                    soft_mask_id
                }),
                Repr::Jpeg(_) => None,
            };

            let image_stream = match &repr {
                Repr::Sampled(s) => {
                    FilterStream::new_from_binary_data(&s.image_data, &serialize_settings)
                }
                Repr::Jpeg(j) => FilterStream::new_from_jpeg_data(&j.data, &serialize_settings),
            };

            let mut image_x_object = chunk.image_xobject(root_ref, image_stream.encoded_data());
            image_stream.write_filters(image_x_object.deref_mut().deref_mut());
            image_x_object.width(self.size().width() as i32);
            image_x_object.height(self.size().height() as i32);

            let mut use_icc = false;

            // TODO: Unfortunately we cannot add the ICC profile via the serializer context
            // (and thus ensure that they are deduplicated, because we cannot access
            // it in a deferred closure. Because of this, we have to manually append
            // the ICC profile to the chunk. Maybe there is some way of avoiding it.
            let icc_chunk = repr.icc().and_then(|ic| {
                if serialize_settings.pdf_version.supports_icc(ic.metadata()) {
                    use_icc = true;
                    let mut chunk = ic.serialize_with_settings(serialize_settings.clone(), icc_ref);
                    let mut array = chunk.indirect(cs_ref).array();
                    array.item(Name(b"ICCBased"));
                    array.item(icc_ref);
                    array.finish();
                    Some(chunk)
                } else {
                    // Don't embed ICC profiles from images if the current
                    // PDF version does not support it.
                    None
                }
            });

            if use_icc {
                image_x_object.pair(Name(b"ColorSpace"), cs_ref);
            }   else {
                let name = match repr.color_space() {
                    ImageColorspace::Rgb => DEVICE_RGB.to_pdf_name(),
                    ImageColorspace::Luma => DEVICE_GRAY.to_pdf_name(),
                    ImageColorspace::Cmyk => DEVICE_CMYK.to_pdf_name(),
                };

                image_x_object.pair(Name(b"ColorSpace"), name);
            }

            // Photoshop CMYK images need to be inverted, see
            // https://github.com/sile-typesetter/libtexpdf/blob/1891bee5e0b73165e4a259f910d3ea3fe1df0b42/jpegimage.c#L25-L51
            // I'm not sure if this applies to all JPEG CMYK images out there, but for now we just
            // always do it. In libtexpdf, they only seem to do it if they can find the Adobe APP
            // marker.
            if let Repr::Jpeg(j) = &repr {
                if j.invert_cmyk {
                    image_x_object
                        .decode([1.0, 0.0].repeat(j.image_color_space.num_components() as usize));
                }
            }

            image_x_object.bits_per_component(repr.bits_per_component().as_u8() as i32);
            if let Some(soft_mask_id) = alpha_mask {
                image_x_object.s_mask(soft_mask_id);
            }
            image_x_object.finish();

            if let Some(icc_chunk) = icc_chunk {
                chunk.extend(&icc_chunk);
            }

            Some(chunk)
        })
    }
}

fn get_icc_profile_type(data: Vec<u8>, color_space: ImageColorspace) -> Option<ICCProfileWrapper> {
    let wrapper = match color_space {
        ImageColorspace::Rgb => ICCProfileWrapper::Rgb(ICCProfile::new(Arc::new(data))?),
        ImageColorspace::Luma => ICCProfileWrapper::Luma(ICCProfile::new(Arc::new(data))?),
        ImageColorspace::Cmyk => ICCProfileWrapper::Cmyk(ICCProfile::new(Arc::new(data))?),
    };

    Some(wrapper)
}

impl RegisterableResource<crate::resource::XObject> for Image {}

fn handle_u8_image(data: Vec<u8>, cs: ColorSpace) -> (Vec<u8>, Option<Vec<u8>>, BitsPerComponent) {
    let mut alphas = if cs.has_alpha() {
        Vec::with_capacity(data.len() / cs.num_components())
    } else {
        Vec::new()
    };

    let encoded_image = match cs {
        ColorSpace::RGB => data,
        ColorSpace::RGBA => data
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
            .collect::<Vec<_>>(),
        ColorSpace::Luma => data,
        ColorSpace::LumaA => data
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
            .collect::<Vec<_>>(),
        _ => unimplemented!(),
    };

    let encoded_mask = if !alphas.is_empty() {
        Some(alphas)
    } else {
        None
    };

    (encoded_image, encoded_mask, BitsPerComponent::Eight)
}

fn handle_u16_image(
    data: Vec<u16>,
    cs: ColorSpace,
) -> (Vec<u8>, Option<Vec<u8>>, BitsPerComponent) {
    let mut alphas = if cs.has_alpha() {
        // * 2 because we are going from u16 to u8
        Vec::with_capacity(2 * data.len() / cs.num_components())
    } else {
        Vec::new()
    };

    let encoded_image = match cs {
        ColorSpace::RGB => data
            .iter()
            .flat_map(|b| b.to_be_bytes())
            .collect::<Vec<_>>(),
        ColorSpace::RGBA => data
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
            .collect::<Vec<_>>(),
        ColorSpace::Luma => data
            .iter()
            .flat_map(|b| b.to_be_bytes())
            .collect::<Vec<_>>(),
        ColorSpace::LumaA => data
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
            .collect::<Vec<_>>(),
        _ => unimplemented!(),
    };

    let encoded_mask = if !alphas.is_empty() {
        Some(alphas)
    } else {
        None
    };

    (encoded_image, encoded_mask, BitsPerComponent::Sixteen)
}

#[cfg(test)]
mod tests {
    use crate::image::{Image, ImageSize};
    use crate::serialize::SerializerContext;
    use crate::surface::Surface;
    use crate::tests::load_image;
    use crate::Document;
    use krilla_macros::{snapshot, visreg};
    use tiny_skia_path::Size;

    #[snapshot]
    fn image_luma8_png(sc: &mut SerializerContext) {
        sc.add_image(load_image("luma8.png"));
    }

    #[snapshot]
    fn image_luma16_png(sc: &mut SerializerContext) {
        sc.add_image(load_image("luma16.png"));
    }

    #[snapshot]
    fn image_rgb8_png(sc: &mut SerializerContext) {
        sc.add_image(load_image("rgb8.png"));
    }

    #[snapshot]
    fn image_rgb16_png(sc: &mut SerializerContext) {
        sc.add_image(load_image("rgb16.png"));
    }

    #[snapshot]
    fn image_rgba8_png(sc: &mut SerializerContext) {
        sc.add_image(load_image("rgba8.png"));
    }

    #[snapshot]
    fn image_rgba16_png(sc: &mut SerializerContext) {
        sc.add_image(load_image("rgba16.png"));
    }

    #[snapshot]
    fn image_luma8_jpg(sc: &mut SerializerContext) {
        sc.add_image(load_image("luma8.jpg"));
    }

    #[snapshot]
    fn image_rgb8_jpg(sc: &mut SerializerContext) {
        sc.add_image(load_image("rgb8.jpg"));
    }

    // Currently gets converted into RGB.
    #[snapshot]
    fn image_cmyk_jpg(sc: &mut SerializerContext) {
        sc.add_image(load_image("cmyk.jpg"));
    }

    // Currently gets converted into RGBA.
    #[snapshot]
    fn image_rgb8_gif(sc: &mut SerializerContext) {
        sc.add_image(load_image("rgb8.gif"));
    }

    #[snapshot]
    fn image_rgba8_gif(sc: &mut SerializerContext) {
        sc.add_image(load_image("rgba8.gif"));
    }
    #[snapshot]
    fn image_rgba8_webp(sc: &mut SerializerContext) {
        sc.add_image(load_image("rgba8.webp"));
    }

    fn image_visreg_impl(surface: &mut Surface, name: &str, load_fn: fn(&str) -> Image) {
        let image = load_fn(name);
        let size = image.size().to_tiny_skia();
        surface.draw_image(image, size);
    }

    #[visreg(all)]
    fn image_luma8_png(surface: &mut Surface) {
        image_visreg_impl(surface, "luma8.png", load_image);
    }

    #[visreg(all)]
    fn image_luma16_png(surface: &mut Surface) {
        image_visreg_impl(surface, "luma16.png", load_image);
    }

    #[visreg(all)]
    fn image_rgb8_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb8.png", load_image);
    }

    #[visreg(all)]
    fn image_rgb16_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb16.png", load_image);
    }

    #[visreg(all)]
    fn image_rgba8_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgba8.png", load_image);
    }

    #[visreg(all)]
    fn image_rgba16_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgba16.png", load_image);
    }

    #[visreg(pdfium, mupdf, pdfbox, pdfjs, poppler, quartz)]
    fn image_luma8_jpg(surface: &mut Surface) {
        image_visreg_impl(surface, "luma8.jpg", load_image);
    }

    #[visreg(pdfium, mupdf, pdfbox, pdfjs, poppler, quartz)]
    fn image_rgb8_jpg(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb8.jpg", load_image);
    }

    #[visreg(pdfium, mupdf, pdfbox, pdfjs, poppler, quartz)]
    fn image_cmyk_jpg(surface: &mut Surface) {
        image_visreg_impl(surface, "cmyk.jpg", load_image);
    }

    #[visreg(all)]
    fn image_rgb8_gif(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb8.gif", load_image);
    }

    #[visreg(all)]
    fn image_rgba8_gif(surface: &mut Surface) {
        image_visreg_impl(surface, "rgba8.gif", load_image);
    }

    #[visreg(all)]
    fn image_rgba8_webp(surface: &mut Surface) {
        image_visreg_impl(surface, "rgba8.webp", load_image);
    }

    #[visreg]
    fn image_cmyk_icc_jpg(surface: &mut Surface) {
        image_visreg_impl(surface, "cmyk_icc.jpg", load_image);
    }

    #[visreg]
    fn image_rgb8_icc_jpg(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb8_icc.jpg", load_image);
    }

    #[visreg]
    fn image_luma8_icc_png(surface: &mut Surface) {
        image_visreg_impl(surface, "luma8_icc.png", load_image);
    }

    #[visreg]
    fn image_rgba8_icc_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgba8_icc.png", load_image);
    }

    #[visreg]
    fn image_rgb8_icc_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb8_icc.png", load_image);
    }

    #[visreg]
    fn image_resized(surface: &mut Surface) {
        let image = load_image("rgba8.png");
        surface.draw_image(image, ImageSize::new(100, 80).to_tiny_skia());
    }

    #[snapshot(document)]
    fn image_deduplication(document: &mut Document) {
        let size = load_image("luma8.png").size().to_tiny_skia();
        let mut page = document.start_page();
        let mut surface = page.surface();
        surface.draw_image(load_image("luma8.png"), size);
        surface.draw_image(load_image("luma8.png"), size);
        surface.finish();

        page.finish();

        let mut page = document.start_page();
        let mut surface = page.surface();
        surface.draw_image(load_image("luma8.png"), size);
    }
}
