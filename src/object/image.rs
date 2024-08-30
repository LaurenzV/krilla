use crate::chunk_container::ChunkContainer;
use crate::error::KrillaResult;
use crate::object::color_space::DEVICE_GRAY;
use crate::serialize::{FilterStream, Object, SerializerContext};
use crate::util::{NameExt, Prehashed, SizeWrapper};
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::ops::DerefMut;
use std::sync::Arc;
use tiny_skia_path::Size;
use zune_jpeg::zune_core::result::DecodingResult;
use zune_jpeg::JpegDecoder;
use zune_png::zune_core::colorspace::ColorSpace;
use zune_png::PngDecoder;

#[derive(Debug, Hash, Eq, PartialEq)]
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

#[derive(Debug, Hash, Eq, PartialEq)]
enum ImageColorspace {
    Rgb,
    Luma,
}

impl TryFrom<ColorSpace> for ImageColorspace {
    type Error = ();

    fn try_from(value: ColorSpace) -> Result<Self, Self::Error> {
        match value {
            ColorSpace::RGB => Ok(ImageColorspace::Rgb),
            ColorSpace::RGBA => Ok(ImageColorspace::Rgb),
            ColorSpace::Luma => Ok(ImageColorspace::Luma),
            ColorSpace::LumaA => Ok(ImageColorspace::Luma),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Repr {
    image_data: Vec<u8>,
    is_dct_encoded: bool,
    size: SizeWrapper,
    mask_data: Option<Vec<u8>>,
    bits_per_component: BitsPerComponent,
    image_color_space: ImageColorspace,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Image(Arc<Prehashed<Repr>>);

impl Image {
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

        let (image_data, mask_data, bits_per_component) = match decoded {
            DecodingResult::U8(u8) => handle_u8_image(u8, color_space),
            DecodingResult::U16(u16) => handle_u16_image(u16, color_space),
            _ => return None,
        };

        Some(Self(Arc::new(Prehashed::new(Repr {
            image_data,
            mask_data,
            is_dct_encoded: false,
            bits_per_component,
            image_color_space,
            size: SizeWrapper(size),
        }))))
    }

    pub fn from_jpeg(data: &[u8]) -> Option<Self> {
        let mut decoder = JpegDecoder::new(data);
        decoder.decode_headers().ok()?;
        let size = {
            let dimensions = decoder.dimensions()?;
            Size::from_wh(dimensions.0 as f32, dimensions.1 as f32)?
        };

        let color_space = decoder.get_output_colorspace()?;
        let image_color_space = color_space.try_into().ok()?;

        let decoded = decoder.decode().ok()?;
        let (image_data, _, bits_per_component) = handle_u8_image(decoded, color_space);

        Some(Self(Arc::new(Prehashed::new(Repr {
            image_data,
            mask_data: None,
            is_dct_encoded: false,
            bits_per_component,
            image_color_space,
            size: SizeWrapper(size),
        }))))
    }

    pub fn from_gif(data: &[u8]) -> Option<Self> {
        let mut decoder = gif::DecodeOptions::new();
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = decoder.read_info(data).ok()?;
        let first_frame = decoder.read_next_frame().ok()??;

        let size = Size::from_wh(first_frame.width as f32, first_frame.height as f32)?;

        let (image_data, mask_data, bits_per_component) =
            handle_u8_image(first_frame.buffer.to_vec(), ColorSpace::RGBA);

        Some(Self(Arc::new(Prehashed::new(Repr {
            image_data,
            mask_data,
            is_dct_encoded: false,
            bits_per_component,
            image_color_space: ImageColorspace::Rgb,
            size: SizeWrapper(size),
        }))))
    }

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

        let (image_data, mask_data, bits_per_component) = handle_u8_image(first_frame, color_space);

        Some(Self(Arc::new(Prehashed::new(Repr {
            image_data,
            mask_data,
            is_dct_encoded: false,
            bits_per_component,
            image_color_space,
            size: SizeWrapper(size),
        }))))
    }

    pub fn size(&self) -> Size {
        self.0.size.0
    }
}

impl Object for Image {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.images
    }

    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let alpha_mask = self.0.mask_data.as_ref().map(|mask_data| {
            let soft_mask_id = sc.new_ref();
            let mask_stream = FilterStream::new_from_binary_data(mask_data, &sc.serialize_settings);
            let mut s_mask = chunk.image_xobject(soft_mask_id, &mask_stream.encoded_data());
            mask_stream.write_filters(s_mask.deref_mut().deref_mut());
            s_mask.width(self.0.size.width() as i32);
            s_mask.height(self.0.size.height() as i32);
            s_mask.pair(
                Name(b"ColorSpace"),
                // Mask color space must be device gray -- see Table 145.
                DEVICE_GRAY.to_pdf_name(),
            );
            s_mask.bits_per_component(self.0.bits_per_component.as_u8() as i32);
            soft_mask_id
        });

        let image_stream = if self.0.is_dct_encoded {
            FilterStream::new_from_dct_encoded(&self.0.image_data, &sc.serialize_settings)
        } else {
            FilterStream::new_from_binary_data(&self.0.image_data, &sc.serialize_settings)
        };

        let mut image_x_object = chunk.image_xobject(root_ref, &image_stream.encoded_data());
        image_stream.write_filters(image_x_object.deref_mut().deref_mut());
        image_x_object.width(self.0.size.width() as i32);
        image_x_object.height(self.0.size.height() as i32);

        match self.0.image_color_space {
            ImageColorspace::Rgb => {
                image_x_object.pair(Name(b"ColorSpace"), sc.rgb());
            }
            ImageColorspace::Luma => {
                image_x_object.pair(Name(b"ColorSpace"), sc.gray());
            }
        };

        image_x_object.bits_per_component(self.0.bits_per_component.as_u8() as i32);
        if let Some(soft_mask_id) = alpha_mask {
            image_x_object.s_mask(soft_mask_id);
        }
        image_x_object.finish();

        Ok(chunk)
    }
}

fn handle_u8_image<'a>(
    data: Vec<u8>,
    cs: ColorSpace,
) -> (Vec<u8>, Option<Vec<u8>>, BitsPerComponent) {
    let mut alphas = Vec::new();

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

fn handle_u16_image<'a>(
    data: Vec<u16>,
    cs: ColorSpace,
) -> (Vec<u8>, Option<Vec<u8>>, BitsPerComponent) {
    let mut alphas = Vec::new();

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
    mod snapshot {
        use crate::serialize::SerializerContext;
        use crate::tests::{load_gif_image, load_jpg_image, load_png_image, load_webp_image};
        use krilla_macros::snapshot;

        #[snapshot]
        fn image_luma8_png(sc: &mut SerializerContext) {
            sc.add_object(load_png_image("luma8.png")).unwrap();
        }

        #[snapshot]
        fn image_luma16_png(sc: &mut SerializerContext) {
            sc.add_object(load_png_image("luma16.png")).unwrap();
        }

        #[snapshot]
        fn image_rgb8_png(sc: &mut SerializerContext) {
            sc.add_object(load_png_image("rgb8.png")).unwrap();
        }

        #[snapshot]
        fn image_rgb16_png(sc: &mut SerializerContext) {
            sc.add_object(load_png_image("rgb16.png")).unwrap();
        }

        #[snapshot]
        fn image_rgba8_png(sc: &mut SerializerContext) {
            sc.add_object(load_png_image("rgba8.png")).unwrap();
        }

        #[snapshot]
        fn image_rgba16_png(sc: &mut SerializerContext) {
            sc.add_object(load_png_image("rgba16.png")).unwrap();
        }

        #[snapshot]
        fn image_luma8_jpg(sc: &mut SerializerContext) {
            sc.add_object(load_jpg_image("luma8.jpg")).unwrap();
        }

        #[snapshot]
        fn image_rgb8_jpg(sc: &mut SerializerContext) {
            sc.add_object(load_jpg_image("rgb8.jpg")).unwrap();
        }

        // Currently gets converted into RGB.
        #[snapshot]
        fn image_cmyk_jpg(sc: &mut SerializerContext) {
            sc.add_object(load_jpg_image("cmyk.jpg")).unwrap();
        }

        // Currently gets converted into RGBA.
        #[snapshot]
        fn image_rgb8_gif(sc: &mut SerializerContext) {
            sc.add_object(load_gif_image("rgb8.gif")).unwrap();
        }

        #[snapshot]
        fn image_rgba8_gif(sc: &mut SerializerContext) {
            sc.add_object(load_gif_image("rgba8.gif")).unwrap();
        }
        #[snapshot]
        fn image_rgba8_webp(sc: &mut SerializerContext) {
            sc.add_object(load_webp_image("rgba8.webp")).unwrap();
        }
    }

    mod visreg {
        use crate::image::Image;
        use crate::surface::Surface;
        use crate::tests::{load_gif_image, load_jpg_image, load_png_image, load_webp_image};
        use krilla_macros::visreg;

        fn image_impl(surface: &mut Surface, name: &str, load_fn: fn(&str) -> Image) {
            let image = load_fn(name);
            let size = image.size();
            surface.draw_image(image, size);
        }

        #[visreg(all)]
        fn image_luma8_png(surface: &mut Surface) {
            image_impl(surface, "luma8.png", load_png_image);
        }

        #[visreg(all)]
        fn image_luma16_png(surface: &mut Surface) {
            image_impl(surface, "luma16.png", load_png_image);
        }

        #[visreg(all)]
        fn image_rgb8_png(surface: &mut Surface) {
            image_impl(surface, "rgb8.png", load_png_image);
        }

        #[visreg(all)]
        fn image_rgb16_png(surface: &mut Surface) {
            image_impl(surface, "rgb16.png", load_png_image);
        }

        #[visreg(all)]
        fn image_rgba8_png(surface: &mut Surface) {
            image_impl(surface, "rgba8.png", load_png_image);
        }

        #[visreg(all)]
        fn image_rgba16_png(surface: &mut Surface) {
            image_impl(surface, "rgba16.png", load_png_image);
        }

        #[visreg(pdfium, mupdf, pdfbox, pdfjs, poppler, quartz)]
        fn image_luma8_jpg(surface: &mut Surface) {
            image_impl(surface, "luma8.jpg", load_jpg_image);
        }

        #[visreg(pdfium, mupdf, pdfbox, pdfjs, poppler, quartz)]
        fn image_rgb8_jpg(surface: &mut Surface) {
            image_impl(surface, "rgb8.jpg", load_jpg_image);
        }

        #[visreg(pdfium, mupdf, pdfbox, pdfjs, poppler, quartz)]
        fn image_cmyk_jpg(surface: &mut Surface) {
            image_impl(surface, "cmyk.jpg", load_jpg_image);
        }

        #[visreg(all)]
        fn image_rgb8_gif(surface: &mut Surface) {
            image_impl(surface, "rgb8.gif", load_gif_image);
        }

        #[visreg(all)]
        fn image_rgba8_gif(surface: &mut Surface) {
            image_impl(surface, "rgba8.gif", load_gif_image);
        }
        #[visreg(all)]
        fn image_rgba8_webp(surface: &mut Surface) {
            image_impl(surface, "rgba8.webp", load_webp_image);
        }
    }
}
