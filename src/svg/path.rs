use crate::stream::StreamBuilder;
use crate::svg::util::{convert_fill, convert_stroke};
use usvg::PaintOrder;

pub fn render(path: &usvg::Path, stream_builder: &mut StreamBuilder) {
    if !path.is_visible() {
        return;
    }

    match path.paint_order() {
        PaintOrder::FillAndStroke => {
            fill_path(path, stream_builder);
            stroke_path(path, stream_builder);
        }
        PaintOrder::StrokeAndFill => {
            stroke_path(path, stream_builder);
            fill_path(path, stream_builder);
        }
    }
}

pub fn fill_path(path: &usvg::Path, stream_builder: &mut StreamBuilder) {
    if let Some(fill) = path.fill() {
        let fill = convert_fill(fill, stream_builder.serializer_context());
        stream_builder.draw_fill_path(&path.data(), &fill);
    }
}

pub fn stroke_path(path: &usvg::Path, stream_builder: &mut StreamBuilder) {
    if let Some(stroke) = path.stroke() {
        let stroke = convert_stroke(stroke, stream_builder.serializer_context());
        stream_builder.draw_stroke_path(&path.data(), &stroke);
    }
}
