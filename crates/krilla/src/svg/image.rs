use crate::image::Image;
use crate::path::FillRule;
use crate::surface::Surface;
use crate::svg::{group, ProcessContext};
use crate::util::RectExt;
use tiny_skia_path::Rect;
use usvg::ImageKind;

/// Render an image into a surface.
///
/// Returns `None` if the image could not be rendered.
pub fn render(
    image: &usvg::Image,
    surface: &mut Surface,
    process_context: &mut ProcessContext,
) -> Option<()> {
    if !image.is_visible() {
        return Some(());
    }

    match image.kind() {
        ImageKind::JPEG(d) => {
            let image = Image::from_jpeg(d)?;
            let size = image.size();
            surface.draw_image(image, size);
        }
        ImageKind::PNG(d) => {
            let image = Image::from_png(d)?;
            let size = image.size();
            surface.draw_image(image, size);
        }
        ImageKind::GIF(d) => {
            let image = Image::from_gif(d)?;
            let size = image.size();
            surface.draw_image(image, size);
        }
        ImageKind::WEBP(d) => {
            let image = Image::from_webp(d)?;
            let size = image.size();
            surface.draw_image(image, size);
        }
        ImageKind::SVG(t) => {
            let clip_path =
                Rect::from_xywh(0.0, 0.0, t.size().width(), t.size().height())?.to_clip_path();
            surface.push_clip_path(&clip_path, &FillRule::NonZero);
            group::render(t.root(), surface, process_context);
            surface.pop();
        }
    }

    Some(())
}
