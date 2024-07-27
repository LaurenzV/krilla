use crate::canvas::Surface;
use crate::svg::util::{convert_fill, convert_stroke};
use tiny_skia_path::Transform;
use usvg::PaintOrder;

pub fn render(path: &usvg::Path, surface: &mut dyn Surface) {
    if !path.is_visible() {
        return;
    }

    match path.paint_order() {
        PaintOrder::FillAndStroke => {
            fill_path(path, surface);
            stroke_path(path, surface);
        }
        PaintOrder::StrokeAndFill => {
            stroke_path(path, surface);
            fill_path(path, surface);
        }
    }
}

pub fn fill_path(path: &usvg::Path, surface: &mut dyn Surface) {
    if let Some(fill) = path.fill() {
        surface.fill_path(
            path.data().clone(),
            Transform::identity(),
            convert_fill(fill),
        );
    }
}

pub fn stroke_path(path: &usvg::Path, surface: &mut dyn Surface) {
    if let Some(stroke) = path.stroke() {
        surface.stroke_path(
            path.data().clone(),
            Transform::identity(),
            convert_stroke(stroke),
        );
    }
}
