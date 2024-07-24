use crate::canvas::Surface;
use crate::svg::util::{convert_fill, convert_stroke, convert_transform};
use usvg::PaintOrder;

pub fn render<T>(path: &usvg::Path, transform: &usvg::Transform, surface: &mut T)
where
    T: Surface,
{
    if !path.is_visible() {
        return;
    }

    match path.paint_order() {
        PaintOrder::FillAndStroke => {
            fill_path(path, transform, surface);
            stroke_path(path, transform, surface);
        }
        PaintOrder::StrokeAndFill => {
            stroke_path(path, transform, surface);
            fill_path(path, transform, surface);
        }
    }
}

pub fn fill_path<T>(path: &usvg::Path, transform: &usvg::Transform, surface: &mut T)
where
    T: Surface,
{
    if let Some(fill) = path.fill() {
        surface.fill_path(
            path.data().clone(),
            convert_transform(&transform),
            convert_fill(fill),
        );
    }
}

pub fn stroke_path<T>(path: &usvg::Path, transform: &usvg::Transform, surface: &mut T)
where
    T: Surface,
{
    if let Some(stroke) = path.stroke() {
        surface.stroke_path(
            path.data().clone(),
            convert_transform(&transform),
            convert_stroke(stroke),
        );
    }
}
