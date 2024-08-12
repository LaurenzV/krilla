use crate::surface::Surface;
use crate::svg::util::{convert_fill, convert_stroke};
use crate::svg::ProcessContext;
use usvg::PaintOrder;

pub fn render(
    path: &usvg::Path,
    canvas_builder: &mut Surface,
    process_context: &mut ProcessContext,
) {
    if !path.is_visible() {
        return;
    }

    match path.paint_order() {
        PaintOrder::FillAndStroke => {
            fill_path(path, canvas_builder, process_context);
            stroke_path(path, canvas_builder, process_context);
        }
        PaintOrder::StrokeAndFill => {
            stroke_path(path, canvas_builder, process_context);
            fill_path(path, canvas_builder, process_context);
        }
    }
}

pub fn fill_path(
    path: &usvg::Path,
    canvas_builder: &mut Surface,
    process_context: &mut ProcessContext,
) {
    if let Some(fill) = path.fill() {
        let fill = convert_fill(
            fill,
            canvas_builder.stream_surface(),
            process_context,
            tiny_skia_path::Transform::identity(),
        );
        canvas_builder.fill_path(&path.data(), fill);
    }
}

pub fn stroke_path(
    path: &usvg::Path,
    canvas_builder: &mut Surface,
    process_context: &mut ProcessContext,
) {
    if let Some(stroke) = path.stroke() {
        let stroke = convert_stroke(
            stroke,
            canvas_builder.stream_surface(),
            process_context,
            tiny_skia_path::Transform::identity(),
        );
        canvas_builder.stroke_path(&path.data(), stroke);
    }
}
