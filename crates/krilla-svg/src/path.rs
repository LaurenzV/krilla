use krilla::surface::Surface;
use usvg::PaintOrder;

use crate::util::{convert_fill, convert_stroke, PathExt};
use crate::ProcessContext;

/// Render a path into a surface.
pub(crate) fn render(
    path: &usvg::Path,
    surface: &mut Surface,
    process_context: &mut ProcessContext,
) {
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
pub(crate) fn fill_path(
    path: &usvg::Path,
    surface: &mut Surface,
    process_context: &mut ProcessContext,
) {
    if let Some(fill) = path.fill() {
        let fill = convert_fill(
            fill,
            surface.stream_builder(),
            process_context,
            usvg::tiny_skia_path::Transform::identity(),
        );
        surface.set_fill(fill);
        surface.fill_path(&path.to_krilla());
    }
}

/// Render a stroked path into a surface.
pub(crate) fn stroke_path(
    path: &usvg::Path,
    surface: &mut Surface,
    process_context: &mut ProcessContext,
) {
    if let Some(stroke) = path.stroke() {
        let stroke = convert_stroke(
            stroke,
            surface.stream_builder(),
            process_context,
            usvg::tiny_skia_path::Transform::identity(),
        );
        surface.set_stroke(stroke);
        surface.stroke_path(&path.to_krilla());
    }
}
