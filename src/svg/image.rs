use crate::image::Image;
use crate::path::FillRule;
use crate::surface::Surface;
use crate::svg::{group, ProcessContext};
use crate::util::RectExt;
use tiny_skia_path::Rect;
use usvg::ImageKind;

/// Render an image into a surface.
pub fn render(image: &usvg::Image, surface: &mut Surface, process_context: &mut ProcessContext) {
    if !image.is_visible() {
        return;
    }

    match image.kind() {
        ImageKind::JPEG(d) => {
            // TODO: Remove unwraps
            let d_image = Image::from_jpeg(d).unwrap();
            surface.draw_image(d_image, image.size());
        }
        ImageKind::PNG(d) => {
            let d_image = Image::from_png(d).unwrap();
            surface.draw_image(d_image, image.size());
        }
        ImageKind::GIF(d) => {
            let d_image = Image::from_gif(d).unwrap();
            surface.draw_image(d_image, image.size());
        }
        ImageKind::WEBP(d) => {
            let d_image = Image::from_webp(d).unwrap();
            surface.draw_image(d_image, image.size());
        }
        ImageKind::SVG(t) => {
            surface.push_clip_path(
                &Rect::from_xywh(0.0, 0.0, t.size().width(), t.size().height())
                    .unwrap()
                    .to_clip_path(),
                &FillRule::NonZero,
            );
            group::render(t.root(), surface, process_context);
            surface.pop();
        }
    }
}
