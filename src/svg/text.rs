use crate::canvas::CanvasBuilder;
use crate::font::Glyph;
use crate::svg::util::{convert_fill, convert_stroke, convert_transform};
use crate::svg::{path, FontContext};
use crate::{Fill, Stroke};
use skrifa::GlyphId;
use tiny_skia_path::{FiniteF32, Transform};
use usvg::PaintOrder;

pub fn render(
    text: &usvg::Text,
    canvas_builder: &mut CanvasBuilder,
    font_context: &mut FontContext,
) {
    for span in text.layouted() {
        if !span.visible {
            continue;
        }

        if let Some(overline) = &span.overline {
            path::render(overline, canvas_builder, font_context);
        }

        if let Some(underline) = &span.underline {
            path::render(underline, canvas_builder, font_context);
        }

        for glyph in &span.positioned_glyphs {
            let font = font_context.fonts.get(&glyph.font).unwrap().font.clone();
            let fill = span
                .fill
                .as_ref()
                .map(|f| convert_fill(&f, canvas_builder.sub_canvas(), font_context));
            let stroke = span
                .stroke
                .as_ref()
                .map(|s| convert_stroke(&s, canvas_builder.sub_canvas(), font_context));

            let transform = glyph.transform().pre_concat(Transform::from_scale(
                font.units_per_em() as f32 / span.font_size.get(),
                font.units_per_em() as f32 / span.font_size.get(),
            ));

            let fill_op = |sb: &mut CanvasBuilder, fill: &Fill| {
                sb.fill_glyph(
                    Glyph::new(GlyphId::new(glyph.id.0 as u32), glyph.text.clone()),
                    font.clone(),
                    FiniteF32::new(span.font_size.get()).unwrap(),
                    &convert_transform(&transform),
                    &fill,
                );
            };

            let stroke_op = |sb: &mut CanvasBuilder, stroke: &Stroke| {
                sb.stroke_glyph(
                    Glyph::new(GlyphId::new(glyph.id.0 as u32), glyph.text.clone()),
                    font.clone(),
                    FiniteF32::new(span.font_size.get()).unwrap(),
                    &convert_transform(&transform),
                    &stroke,
                );
            };

            match (fill, stroke) {
                (Some(fill), Some(stroke)) => match span.paint_order {
                    PaintOrder::FillAndStroke => {
                        fill_op(canvas_builder, &fill);
                        stroke_op(canvas_builder, &stroke);
                    }
                    PaintOrder::StrokeAndFill => {
                        stroke_op(canvas_builder, &stroke);
                        fill_op(canvas_builder, &fill);
                    }
                },
                (Some(fill), None) => {
                    fill_op(canvas_builder, &fill);
                }
                (None, Some(stroke)) => {
                    stroke_op(canvas_builder, &stroke);
                }
                (None, None) => canvas_builder.invisible_glyph(
                    Glyph::new(GlyphId::new(glyph.id.0 as u32), glyph.text.clone()),
                    font,
                    FiniteF32::new(span.font_size.get()).unwrap(),
                    &convert_transform(&transform),
                ),
            }
        }

        if let Some(line_through) = &span.line_through {
            path::render(line_through, canvas_builder, font_context);
        }
    }
}
