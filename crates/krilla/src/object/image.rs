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
use crate::resource::{RegisterableResource, Resource};
use crate::serialize::SerializerContext;
use crate::stream::FilterStream;
use crate::util::{Deferred, NameExt, SizeWrapper};
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tiny_skia_path::Size;
use zune_jpeg::zune_core::result::DecodingResult;
use zune_jpeg::JpegDecoder;
use zune_png::zune_core::colorspace::ColorSpace;
use zune_png::PngDecoder;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct ImageIdentifier(pub(crate) Ref);

impl From<ImageIdentifier> for Resource {
    fn from(value: ImageIdentifier) -> Self {
        Self::ImageIdentifier(value)
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

/// The colorspace of an image.
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

/// Representation of a raw, decode image.
#[derive(Debug, Hash, Eq, PartialEq)]
struct SampledRepr {
    color_channel: Vec<u8>,
    alpha_channel: Option<Vec<u8>>,
    icc: Option<Arc<Vec<u8>>>,
    size: SizeWrapper,
    image_color_space: ImageColorspace,
}

/// Representation of an encoded jpg image.
#[derive(Debug, Hash, Eq, PartialEq)]
struct JpegRepr {
    data: Arc<Vec<u8>>,
    icc: Option<Arc<Vec<u8>>>,
    size: SizeWrapper,
    bits_per_component: BitsPerComponent,
    image_color_space: ImageColorspace,
}

#[derive(Debug, Hash, Eq, PartialEq)]
enum Repr {
    Sampled(SampledRepr),
    Jpeg(JpegRepr),
}

impl Repr {
    fn size(&self) -> Size {
        match self {
            Repr::Sampled(s) => s.size.0,
            Repr::Jpeg(j) => j.size.0,
        }
    }

    fn icc(&self) -> Option<Arc<Vec<u8>>> {
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

pub trait Image: Hash + Clone + Eq + Send + Sync + 'static {
    fn is_jpeg(&self) -> bool;
    fn raw_image(&self) -> &[u8];
    fn color_channel(&self) -> &[u8];
    fn size(&self) -> (u32, u32);
    fn alpha_channel(&self) -> Option<&[u8]>;
    fn color_space(&self) -> ImageColorspace;
    fn icc_profile(&self) -> Option<Arc<Vec<u8>>>;
}

/// A bitmap image.
///
/// This type is cheap to hash and clone, but expensive to create.
#[derive(Hash, Clone, PartialEq, Eq)]
pub struct KrillaImage(Arc<Repr>);

fn get_icc_profile_type(data: Vec<u8>, color_space: ImageColorspace) -> Option<ICCProfileWrapper> {
    let wrapper = match color_space {
        ImageColorspace::Rgb => ICCProfileWrapper::Rgb(ICCProfile::new(Arc::new(data))?),
        ImageColorspace::Luma => ICCProfileWrapper::Luma(ICCProfile::new(Arc::new(data))?),
        ImageColorspace::Cmyk => ICCProfileWrapper::Cmyk(ICCProfile::new(Arc::new(data))?),
    };

    Some(wrapper)
}

impl KrillaImage {
    /// Create a new bitmap image from a `.png` file.
    ///
    /// Returns `None` if krilla was unable to parse the file.
    pub fn from_png(data: &[u8]) -> Option<Self> {
        let mut decoder = PngDecoder::new(data);
        decoder.decode_headers().ok()?;

        let color_space = decoder.get_colorspace()?;
        let image_color_space = color_space.try_into().ok()?;

        let size = {
            let info = decoder.get_info()?;
            Size::from_wh(info.width as f32, info.height as f32)?
        };
        let decoded = decoder.decode().ok()?;

        let (color_channel, alpha_channel) = match decoded {
            DecodingResult::U8(u8) => handle_u8_image(u8, color_space),
            DecodingResult::U16(_) => return None,
            _ => return None,
        };

        let icc = decoder.get_info()?.icc_profile.clone().map(Arc::new);

        Some(Self(Arc::new(Repr::Sampled(SampledRepr {
            color_channel,
            alpha_channel,
            icc,
            image_color_space,
            size: SizeWrapper(size),
        }))))
    }

    /// Create a new bitmap image from a `.jpg` file.
    ///
    /// Returns `None` if krilla was unable to parse the file.
    pub fn from_jpeg(data: Arc<Vec<u8>>) -> Option<Self> {
        let mut decoder = JpegDecoder::new(data.as_slice());
        decoder.decode_headers().ok()?;
        let size = {
            let dimensions = decoder.dimensions()?;
            Size::from_wh(dimensions.0 as f32, dimensions.1 as f32)?
        };

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
            let icc = decoder.icc_profile().map(|d| Arc::new(d));

            Some(Self(Arc::new(Repr::Jpeg(JpegRepr {
                data: data.clone(),
                icc,
                size: SizeWrapper(size),
                bits_per_component: BitsPerComponent::Eight,
                image_color_space,
            }))))
        } else {
            // Unknown color space, fall back to decoding the JPEG into RGB8 and saving
            // it in the sampled representation.
            let output_color_space = decoder.get_output_colorspace()?;
            let image_color_space = output_color_space.try_into().ok()?;

            let decoded = decoder.decode().ok()?;
            let (color_channel, alpha_channel) = handle_u8_image(decoded, output_color_space);

            Some(Self(Arc::new(Repr::Sampled(SampledRepr {
                // Cannot embed ICC profile in this case, since we are converting to RGB.
                icc: None,
                color_channel,
                alpha_channel,
                image_color_space,
                size: SizeWrapper(size),
            }))))
        }
    }

    /// Create a new bitmap image from a `.gif` file.
    ///
    /// Returns `None` if krilla was unable to parse the file.
    pub fn from_gif(data: &[u8]) -> Option<Self> {
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = decoder.read_info(data).ok()?;
        let first_frame = decoder.read_next_frame().ok()??;

        let size = Size::from_wh(first_frame.width as f32, first_frame.height as f32)?;

        let (color_channel, alpha_channel) =
            handle_u8_image(first_frame.buffer.to_vec(), ColorSpace::RGBA);

        Some(Self(Arc::new(Repr::Sampled(SampledRepr {
            icc: None,
            color_channel,
            alpha_channel,
            image_color_space: ImageColorspace::Rgb,
            size: SizeWrapper(size),
        }))))
    }

    /// Create a new bitmap image from a `.webp` file.
    ///
    /// Returns `None` if krilla was unable to parse the file.
    pub fn from_webp(data: &[u8]) -> Option<Self> {
        let mut decoder = image_webp::WebPDecoder::new(std::io::Cursor::new(data)).ok()?;
        let mut first_frame = vec![0; decoder.output_buffer_size()?];
        decoder.read_image(&mut first_frame).ok()?;

        let size = {
            let (w, h) = decoder.dimensions();
            Size::from_wh(w as f32, h as f32)?
        };

        let color_space = if decoder.has_alpha() {
            ColorSpace::RGBA
        } else {
            ColorSpace::RGB
        };
        let image_color_space = color_space.try_into().ok()?;

        let icc = decoder.icc_profile().ok()?.map(|d| Arc::new(d));

        let (color_channel, alpha_channel) = handle_u8_image(first_frame, color_space);

        Some(Self(Arc::new(Repr::Sampled(SampledRepr {
            icc,
            color_channel,
            alpha_channel,
            image_color_space,
            size: SizeWrapper(size),
        }))))
    }

    /// Returns the dimensions of the image.
    pub fn size(&self) -> Size {
        self.0.size()
    }
}

impl Image for KrillaImage {
    fn is_jpeg(&self) -> bool {
        match self.0.deref() {
            Repr::Sampled(_) => false,
            Repr::Jpeg(_) => true,
        }
    }

    fn raw_image(&self) -> &[u8] {
        match self.0.deref() {
            Repr::Sampled(_) => unreachable!(),
            Repr::Jpeg(j) => j.data.as_ref(),
        }
    }

    fn color_channel(&self) -> &[u8] {
        match self.0.deref() {
            Repr::Sampled(s) => &s.color_channel,
            Repr::Jpeg(_) => unreachable!(),
        }
    }

    fn size(&self) -> (u32, u32) {
        match self.0.deref() {
            Repr::Sampled(s) => (s.size.0.width() as u32, s.size.0.height() as u32),
            Repr::Jpeg(j) => (j.size.0.width() as u32, j.size.0.height() as u32),
        }
    }

    fn alpha_channel(&self) -> Option<&[u8]> {
        match self.0.deref() {
            Repr::Sampled(s) => s.alpha_channel.as_deref(),
            Repr::Jpeg(j) => None,
        }
    }

    fn color_space(&self) -> ImageColorspace {
        match self.0.deref() {
            Repr::Sampled(s) => s.image_color_space,
            Repr::Jpeg(j) => j.image_color_space,
        }
    }

    fn icc_profile(&self) -> Option<Arc<Vec<u8>>> {
        match self.0.deref() {
            Repr::Sampled(s) => s.icc.clone(),
            Repr::Jpeg(j) => j.icc.clone(),
        }
    }
}

pub(crate) fn serialize_image(
    image: impl Image,
    sc: &mut SerializerContext,
    root_ref: Ref,
) -> Deferred<Chunk> {
    let soft_mask_id = if image.alpha_channel().is_some() {
        Some(sc.new_ref())
    } else {
        None
    };

    let icc_profile = image.icc_profile().map(|p| match image.color_space() {
        ImageColorspace::Rgb => ICCProfileWrapper::Rgb(ICCProfile::new(p.clone()).unwrap()),
        ImageColorspace::Luma => ICCProfileWrapper::Luma(ICCProfile::new(p.clone()).unwrap()),
        ImageColorspace::Cmyk => ICCProfileWrapper::Cmyk(ICCProfile::new(p.clone()).unwrap()),
    });

    let icc_ref = icc_profile.and_then(|ic| {
        if sc
            .serialize_settings()
            .pdf_version
            .supports_icc(ic.metadata())
        {
            let ref_ = match ic {
                ICCProfileWrapper::Luma(l) => sc.add_object(ICCBasedColorSpace(l)),
                ICCProfileWrapper::Rgb(r) => sc.add_object(ICCBasedColorSpace(r)),
                ICCProfileWrapper::Cmyk(c) => sc.add_object(ICCBasedColorSpace(c)),
            };

            Some(ref_)
        } else {
            // Don't embed ICC profiles from images if the current
            // PDF version does not support it.
            None
        }
    });

    let serialize_settings = sc.serialize_settings().clone();

    Deferred::new(move || {
        // TODO: Validate image
        let mut chunk = Chunk::new();

        let alpha_mask = image.alpha_channel().map(|data| {
            let soft_mask_id = soft_mask_id.unwrap();
            let mask_stream = FilterStream::new_from_binary_data(data, &serialize_settings);
            let mut s_mask = chunk.image_xobject(soft_mask_id, mask_stream.encoded_data());
            mask_stream.write_filters(s_mask.deref_mut().deref_mut());
            s_mask.width(image.size().0 as i32);
            s_mask.height(image.size().1 as i32);
            s_mask.pair(
                Name(b"ColorSpace"),
                // Mask color space must be device gray -- see Table 145.
                DEVICE_GRAY.to_pdf_name(),
            );
            s_mask.bits_per_component(8);
            soft_mask_id
        });

        let image_stream = if image.is_jpeg() {
            FilterStream::new_from_jpeg_data(&image.raw_image(), &serialize_settings)
        } else {
            FilterStream::new_from_binary_data(&image.color_channel(), &serialize_settings)
        };

        let mut image_x_object = chunk.image_xobject(root_ref, image_stream.encoded_data());
        image_stream.write_filters(image_x_object.deref_mut().deref_mut());
        image_x_object.width(image.size().0 as i32);
        image_x_object.height(image.size().1 as i32);

        if let Some(icc_ref) = icc_ref {
            image_x_object.pair(Name(b"ColorSpace"), icc_ref);
        } else {
            let name = match image.color_space() {
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
        if image.is_jpeg() && image.color_space() == ImageColorspace::Cmyk {
            image_x_object.decode([1.0, 0.0].repeat(image.color_space().num_components() as usize));
        }

        image_x_object.bits_per_component(8);
        if let Some(soft_mask_id) = alpha_mask {
            image_x_object.s_mask(soft_mask_id);
        }
        image_x_object.finish();

        chunk
    })
}

impl RegisterableResource<crate::resource::XObject> for ImageIdentifier {}

fn handle_u8_image(data: Vec<u8>, cs: ColorSpace) -> (Vec<u8>, Option<Vec<u8>>) {
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

    (encoded_image, encoded_mask)
}

#[cfg(test)]
mod tests {
    use crate::image::KrillaImage;
    use crate::serialize::SerializerContext;
    use crate::surface::Surface;
    use crate::tests::{load_gif_image, load_jpg_image, load_png_image, load_webp_image};
    use crate::Document;
    use krilla_macros::{snapshot, visreg};
    use tiny_skia_path::Size;

    #[snapshot]
    fn image_luma8_png(sc: &mut SerializerContext) {
        sc.add_image(load_png_image("luma8.png"));
    }

    #[snapshot]
    fn image_luma16_png(sc: &mut SerializerContext) {
        sc.add_image(load_png_image("luma16.png"));
    }

    #[snapshot]
    fn image_rgb8_png(sc: &mut SerializerContext) {
        sc.add_image(load_png_image("rgb8.png"));
    }

    #[snapshot]
    fn image_rgb16_png(sc: &mut SerializerContext) {
        sc.add_image(load_png_image("rgb16.png"));
    }

    #[snapshot]
    fn image_rgba8_png(sc: &mut SerializerContext) {
        sc.add_image(load_png_image("rgba8.png"));
    }

    #[snapshot]
    fn image_rgba16_png(sc: &mut SerializerContext) {
        sc.add_image(load_png_image("rgba16.png"));
    }

    #[snapshot]
    fn image_luma8_jpg(sc: &mut SerializerContext) {
        sc.add_image(load_jpg_image("luma8.jpg"));
    }

    #[snapshot]
    fn image_rgb8_jpg(sc: &mut SerializerContext) {
        sc.add_image(load_jpg_image("rgb8.jpg"));
    }

    // Currently gets converted into RGB.
    #[snapshot]
    fn image_cmyk_jpg(sc: &mut SerializerContext) {
        sc.add_image(load_jpg_image("cmyk.jpg"));
    }

    // Currently gets converted into RGBA.
    #[snapshot]
    fn image_rgb8_gif(sc: &mut SerializerContext) {
        sc.add_image(load_gif_image("rgb8.gif"));
    }

    #[snapshot]
    fn image_rgba8_gif(sc: &mut SerializerContext) {
        sc.add_image(load_gif_image("rgba8.gif"));
    }
    #[snapshot]
    fn image_rgba8_webp(sc: &mut SerializerContext) {
        sc.add_image(load_webp_image("rgba8.webp"));
    }

    fn image_visreg_impl(surface: &mut Surface, name: &str, load_fn: fn(&str) -> KrillaImage) {
        let image = load_fn(name);
        let size = image.size();
        surface.draw_image(image, size);
    }

    #[visreg(all)]
    fn image_luma8_png(surface: &mut Surface) {
        image_visreg_impl(surface, "luma8.png", load_png_image);
    }

    #[visreg(all)]
    fn image_luma16_png(surface: &mut Surface) {
        image_visreg_impl(surface, "luma16.png", load_png_image);
    }

    #[visreg(all)]
    fn image_rgb8_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb8.png", load_png_image);
    }

    #[visreg(all)]
    fn image_rgb16_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb16.png", load_png_image);
    }

    #[visreg(all)]
    fn image_rgba8_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgba8.png", load_png_image);
    }

    #[visreg(all)]
    fn image_rgba16_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgba16.png", load_png_image);
    }

    #[visreg(pdfium, mupdf, pdfbox, pdfjs, poppler, quartz)]
    fn image_luma8_jpg(surface: &mut Surface) {
        image_visreg_impl(surface, "luma8.jpg", load_jpg_image);
    }

    #[visreg(pdfium, mupdf, pdfbox, pdfjs, poppler, quartz)]
    fn image_rgb8_jpg(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb8.jpg", load_jpg_image);
    }

    #[visreg(pdfium, mupdf, pdfbox, pdfjs, poppler, quartz)]
    fn image_cmyk_jpg(surface: &mut Surface) {
        image_visreg_impl(surface, "cmyk.jpg", load_jpg_image);
    }

    #[visreg(all)]
    fn image_rgb8_gif(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb8.gif", load_gif_image);
    }

    #[visreg(all)]
    fn image_rgba8_gif(surface: &mut Surface) {
        image_visreg_impl(surface, "rgba8.gif", load_gif_image);
    }

    #[visreg(all)]
    fn image_rgba8_webp(surface: &mut Surface) {
        image_visreg_impl(surface, "rgba8.webp", load_webp_image);
    }

    #[visreg]
    fn image_cmyk_icc_jpg(surface: &mut Surface) {
        image_visreg_impl(surface, "cmyk_icc.jpg", load_jpg_image);
    }

    #[visreg]
    fn image_rgb8_icc_jpg(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb8_icc.jpg", load_jpg_image);
    }

    #[visreg]
    fn image_luma8_icc_png(surface: &mut Surface) {
        image_visreg_impl(surface, "luma8_icc.png", load_png_image);
    }

    #[visreg]
    fn image_rgba8_icc_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgba8_icc.png", load_png_image);
    }

    #[visreg]
    fn image_rgb8_icc_png(surface: &mut Surface) {
        image_visreg_impl(surface, "rgb8_icc.png", load_png_image);
    }

    #[visreg]
    fn image_resized(surface: &mut Surface) {
        let image = load_png_image("rgba8.png");
        surface.draw_image(image, Size::from_wh(100.0, 80.0).unwrap());
    }

    #[snapshot(document)]
    fn image_deduplication(document: &mut Document) {
        let size = load_png_image("luma8.png").size();
        let mut page = document.start_page();
        let mut surface = page.surface();
        surface.draw_image(load_png_image("luma8.png"), size);
        surface.draw_image(load_png_image("luma8.png"), size);
        surface.finish();

        page.finish();

        let mut page = document.start_page();
        let mut surface = page.surface();
        surface.draw_image(load_png_image("luma8.png"), size);
    }
}
