use crate::canvas::Surface;
use crate::object::image::Image;
use crate::svg::group;
use crate::util::RectExt;
use crate::FillRule;
use image::ImageFormat;
use tiny_skia_path::{Rect, Transform};
use usvg::ImageKind;

pub fn render(image: &usvg::Image, surface: &mut dyn Surface) {
    if !image.is_visible() {
        return;
    }

    match image.kind() {
        ImageKind::JPEG(d) => {
            let dynamic_image =
                image::load_from_memory_with_format(d.as_slice(), ImageFormat::Jpeg).unwrap();
            let d_image = Image::new(&dynamic_image);
            surface.draw_image(d_image, image.size(), Transform::default());
        }
        ImageKind::PNG(d) => {
            let dynamic_image =
                image::load_from_memory_with_format(d.as_slice(), ImageFormat::Png).unwrap();
            let d_image = Image::new(&dynamic_image);
            surface.draw_image(d_image, image.size(), Transform::default());
        }
        ImageKind::GIF(d) => {
            let dynamic_image =
                image::load_from_memory_with_format(d.as_slice(), ImageFormat::Gif).unwrap();
            let d_image = Image::new(&dynamic_image);
            surface.draw_image(d_image, image.size(), Transform::default());
        }
        ImageKind::SVG(t) => {
            let mut clipped = surface.clipped(
                Rect::from_xywh(0.0, 0.0, t.size().width(), t.size().height())
                    .unwrap()
                    .to_clip_path(),
                FillRule::NonZero,
            );
            group::render(t.root(), &mut clipped);
        }
    }
}
