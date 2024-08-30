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
    Cmyk,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Repr {
    image_data: Vec<u8>,
    size: SizeWrapper,
    mask_data: Option<Vec<u8>>,
    bits_per_component: BitsPerComponent,
    image_color_space: ImageColorspace,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Image(Arc<Prehashed<Repr>>);

// TODO: Improve this so:
// 1) Users are not forced to pass a dynamic image
// 2) Use the DCT decoder for JPEG images.
impl Image {
    pub fn from_png(data: &[u8]) -> Option<Self> {
        let mut decoder = PngDecoder::new(data);
        decoder.decode_headers().ok()?;

        let color_space = decoder.get_colorspace()?;
        let image_color_space = match color_space {
            ColorSpace::RGB => ImageColorspace::Rgb,
            ColorSpace::RGBA => ImageColorspace::Rgb,
            ColorSpace::Luma => ImageColorspace::Luma,
            ColorSpace::LumaA => ImageColorspace::Luma,
            _ => return None,
        };

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
            bits_per_component,
            image_color_space,
            size: SizeWrapper(size),
        }))))
    }

    pub fn from_jpeg(data: &[u8]) {
        let mut decoder = JpegDecoder::new(data);
        let size = {
            let dimensions = decoder.dimensions().unwrap();
            Size::from_wh(dimensions.0 as f32, dimensions.1 as f32).unwrap()
        };
        let color_space = decoder.get_output_colorspace().unwrap();
        let headers = decoder.decode().unwrap();
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

        let image_stream =
            FilterStream::new_from_binary_data(&self.0.image_data, &sc.serialize_settings);

        let mut image_x_object = chunk.image_xobject(root_ref, &image_stream.encoded_data());
        image_stream.write_filters(image_x_object.deref_mut().deref_mut());
        image_x_object.width(self.0.size.width() as i32);
        image_x_object.height(self.0.size.height() as i32);

        match self.0.image_color_space {
            ImageColorspace::Rgb => image_x_object.pair(Name(b"ColorSpace"), sc.rgb()),
            ImageColorspace::Luma => image_x_object.pair(Name(b"ColorSpace"), sc.gray()),
            ImageColorspace::Cmyk => unimplemented!(),
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
