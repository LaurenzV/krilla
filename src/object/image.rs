use crate::object::color_space::DEVICE_GRAY;
use crate::serialize::{Object, RegisterableObject, SerializerContext};
use crate::util::{NameExt, Prehashed};
use image::{ColorType, DynamicImage, Luma, Rgb, Rgba};
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::sync::Arc;
use tiny_skia_path::Size;

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Repr {
    image_data: Vec<u8>,
    size: Size,
    mask_data: Option<Vec<u8>>,
    color_type: ColorType,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Image(Arc<Prehashed<Repr>>);

// TODO: Improve this so:
// 1) Users are not forced to pass a dynamic image
// 2) Use the DCT decoder for JPEG images.
impl Image {
    pub fn new(dynamic_image: &DynamicImage) -> Self {
        let (image_data, mask_data) = handle_transparent_image(&dynamic_image);
        let color_type = dynamic_image.color();
        let size =
            Size::from_wh(dynamic_image.width() as f32, dynamic_image.height() as f32).unwrap();
        Self(Arc::new(Prehashed::new(Repr {
            image_data,
            mask_data,
            color_type,
            size,
        })))
    }

    pub fn size(&self) -> Size {
        self.0.size
    }
}

impl Object for Image {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        let alpha_mask = self.0.mask_data.as_ref().map(|mask_data| {
            let soft_mask_id = sc.new_ref();
            let (mask_data, mask_filter) = sc.get_binary_stream(mask_data);
            let mut s_mask = chunk.image_xobject(soft_mask_id, &mask_data);
            s_mask.filter(mask_filter);
            s_mask.width(self.0.size.width() as i32);
            s_mask.height(self.0.size.height() as i32);
            s_mask.pair(
                Name(b"ColorSpace"),
                // Mask color space must be device gray -- see Table 145.
                DEVICE_GRAY.to_pdf_name(),
            );
            s_mask.bits_per_component(calculate_bits_per_component(self.0.color_type));
            soft_mask_id
        });

        let (image_data, image_filter) = sc.get_binary_stream(&self.0.image_data);

        let mut image_x_object = chunk.image_xobject(root_ref, &image_data);
        image_x_object.filter(image_filter);
        image_x_object.width(self.0.size.width() as i32);
        image_x_object.height(self.0.size.height() as i32);

        if self.0.color_type.has_color() {
            image_x_object.pair(Name(b"ColorSpace"), sc.rgb());
        } else {
            image_x_object.pair(Name(b"ColorSpace"), sc.gray());
        }

        image_x_object.bits_per_component(calculate_bits_per_component(self.0.color_type));
        if let Some(soft_mask_id) = alpha_mask {
            image_x_object.s_mask(soft_mask_id);
        }
        image_x_object.finish();

        chunk
    }
}

impl RegisterableObject for Image {}

fn calculate_bits_per_component(color_type: ColorType) -> i32 {
    (color_type.bits_per_pixel() / color_type.channel_count() as u16) as i32
}

fn handle_transparent_image(image: &DynamicImage) -> (Vec<u8>, Option<Vec<u8>>) {
    let color = image.color();
    let bits = color.bits_per_pixel();
    let channels = color.channel_count() as u16;

    let encoded_image: Vec<u8> = match (channels, bits / channels > 8) {
        (1 | 2, false) => image.to_luma8().pixels().flat_map(|&Luma(c)| c).collect(),
        (1 | 2, true) => image
            .to_luma16()
            .pixels()
            .flat_map(|&Luma(x)| x)
            .flat_map(|x| x.to_be_bytes())
            .collect(),
        (3 | 4, false) => image.to_rgb8().pixels().flat_map(|&Rgb(c)| c).collect(),
        (3 | 4, true) => image
            .to_rgb16()
            .pixels()
            .flat_map(|&Rgb(c)| c)
            .flat_map(|x| x.to_be_bytes())
            .collect(),
        _ => panic!("unknown number of channels={channels}"),
    };

    let encoded_mask: Option<Vec<u8>> = if color.has_alpha() {
        if bits / channels > 8 {
            let image = image.to_rgba16();

            if image.pixels().any(|&Rgba([.., a])| a != u16::MAX) {
                Some(
                    image
                        .pixels()
                        .flat_map(|&Rgba([.., a])| a.to_be_bytes())
                        .collect(),
                )
            } else {
                None
            }
        } else {
            let image = image.to_rgba8();

            if image.pixels().any(|&Rgba([.., a])| a != u8::MAX) {
                Some(image.pixels().map(|&Rgba([.., a])| a).collect())
            } else {
                None
            }
        }
    } else {
        None
    };

    (encoded_image, encoded_mask)
}
