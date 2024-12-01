//! Path conversion

use usvg::PaintOrder;

use crate::surface::Surface;
use crate::svg::util::{convert_fill, convert_stroke};
use crate::svg::ProcessContext;

/// Render a path into a surface.
pub(crate) fn render(path: &usvg::Path, surface: &mut Surface, process_context: &mut ProcessContext) {
    if !path.is_visible() {
        return;
    }

    match path.paint_order() {
        PaintOrder::FillAndStroke => {
            fill_path(path, surface, process_context);
            stroke_path(path, surface, process_context);
        }
        PaintOrder::StrokeAndFill => {
            stroke_path(path, surface, process_context);
            fill_path(path, surface, process_context);
        }
    }
}

/// Render a filled path into a surface.
pub(crate) fn fill_path(path: &usvg::Path, surface: &mut Surface, process_context: &mut ProcessContext) {
    if let Some(fill) = path.fill() {
        let fill = convert_fill(
            fill,
            surface.stream_builder(),
            process_context,
            tiny_skia_path::Transform::identity(),
        );
        surface.fill_path(path.data(), fill);
    }
}

/// Render a stroked path into a surface.
pub(crate) fn stroke_path(path: &usvg::Path, surface: &mut Surface, process_context: &mut ProcessContext) {
    if let Some(stroke) = path.stroke() {
        let stroke = convert_stroke(
            stroke,
            surface.stream_builder(),
            process_context,
            tiny_skia_path::Transform::identity(),
        );
        surface.stroke_path(path.data(), stroke);
    }
}
