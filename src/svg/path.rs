use crate::stream::StreamBuilder;
use crate::svg::util::{convert_fill, convert_stroke};
use crate::svg::FontContext;
use usvg::PaintOrder;

pub fn render(
    path: &usvg::Path,
    stream_builder: &mut StreamBuilder,
    font_context: &mut FontContext,
) {
    if !path.is_visible() {
        return;
    }

    match path.paint_order() {
        PaintOrder::FillAndStroke => {
            fill_path(path, stream_builder, font_context);
            stroke_path(path, stream_builder, font_context);
        }
        PaintOrder::StrokeAndFill => {
            stroke_path(path, stream_builder, font_context);
            fill_path(path, stream_builder, font_context);
        }
    }
}

pub fn fill_path(
    path: &usvg::Path,
    stream_builder: &mut StreamBuilder,
    font_context: &mut FontContext,
) {
    if let Some(fill) = path.fill() {
        let fill = convert_fill(fill, stream_builder.serializer_context(), font_context);
        stream_builder.fill_path(&path.data(), &fill);
    }
}

pub fn stroke_path(
    path: &usvg::Path,
    stream_builder: &mut StreamBuilder,
    font_context: &mut FontContext,
) {
    if let Some(stroke) = path.stroke() {
        let stroke = convert_stroke(stroke, stream_builder.serializer_context(), font_context);
        stream_builder.stroke_path(&path.data(), &stroke);
    }
}
