use crate::object::image::Image;
use crate::stream::StreamBuilder;
use crate::svg::{group, FontContext};
use crate::util::RectExt;
use crate::FillRule;
use image::ImageFormat;
use tiny_skia_path::Rect;
use usvg::ImageKind;

pub fn render(
    image: &usvg::Image,
    stream_builder: &mut StreamBuilder,
    font_context: &mut FontContext,
) {
    if !image.is_visible() {
        return;
    }

    match image.kind() {
        ImageKind::JPEG(d) => {
            let dynamic_image =
                image::load_from_memory_with_format(d.as_slice(), ImageFormat::Jpeg).unwrap();
            let d_image = Image::new(&dynamic_image);
            stream_builder.draw_image(d_image, image.size());
        }
        ImageKind::PNG(d) => {
            let dynamic_image =
                image::load_from_memory_with_format(d.as_slice(), ImageFormat::Png).unwrap();
            let d_image = Image::new(&dynamic_image);
            stream_builder.draw_image(d_image, image.size());
        }
        ImageKind::GIF(d) => {
            let dynamic_image =
                image::load_from_memory_with_format(d.as_slice(), ImageFormat::Gif).unwrap();
            let d_image = Image::new(&dynamic_image);
            stream_builder.draw_image(d_image, image.size());
        }
        ImageKind::SVG(t) => {
            stream_builder.push_clip_path(
                &Rect::from_xywh(0.0, 0.0, t.size().width(), t.size().height())
                    .unwrap()
                    .to_clip_path(),
                &FillRule::NonZero,
            );
            group::render(t.root(), stream_builder, font_context);
            stream_builder.pop_clip_path();
        }
        _ => unimplemented!(),
    }
}
