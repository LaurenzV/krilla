use crate::canvas::CanvasBuilder;
use crate::svg::util::{convert_fill, convert_stroke};
use crate::svg::FontContext;
use usvg::PaintOrder;

pub fn render(
    path: &usvg::Path,
    canvas_builder: &mut CanvasBuilder,
    font_context: &mut FontContext,
) {
    if !path.is_visible() {
        return;
    }

    match path.paint_order() {
        PaintOrder::FillAndStroke => {
            fill_path(path, canvas_builder, font_context);
            stroke_path(path, canvas_builder, font_context);
        }
        PaintOrder::StrokeAndFill => {
            stroke_path(path, canvas_builder, font_context);
            fill_path(path, canvas_builder, font_context);
        }
    }
}

pub fn fill_path(
    path: &usvg::Path,
    canvas_builder: &mut CanvasBuilder,
    font_context: &mut FontContext,
) {
    if let Some(fill) = path.fill() {
        let fill = convert_fill(fill, canvas_builder.sub_canvas(), font_context);
        canvas_builder.fill_path(&path.data(), &fill);
    }
}

pub fn stroke_path(
    path: &usvg::Path,
    canvas_builder: &mut CanvasBuilder,
    font_context: &mut FontContext,
) {
    if let Some(stroke) = path.stroke() {
        let stroke = convert_stroke(stroke, canvas_builder.sub_canvas(), font_context);
        canvas_builder.stroke_path(&path.data(), &stroke);
    }
}
