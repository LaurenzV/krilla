use crate::stream::StreamBuilder;
use crate::svg::util::{convert_fill, convert_stroke, convert_transform};
use crate::svg::{path, FontContext};
use skrifa::GlyphId;
use tiny_skia_path::{FiniteF32, Transform};
use usvg::PaintOrder;

pub fn render(
    text: &usvg::Text,
    stream_builder: &mut StreamBuilder,
    font_context: &mut FontContext,
) {
    for span in text.layouted() {
        if !span.visible {
            continue;
        }

        if let Some(overline) = &span.overline {
            path::render(overline, stream_builder, font_context);
        }

        if let Some(underline) = &span.underline {
            path::render(underline, stream_builder, font_context);
        }

        for glyph in &span.positioned_glyphs {
            let font = font_context.fonts.get(&glyph.font).unwrap().font.clone();
            let fill = span.fill.as_ref().map(|f| {
                convert_fill(
                    &f,
                    stream_builder.serializer_context().clone(),
                    font_context,
                )
            });
            let stroke = span.stroke.as_ref().map(|s| {
                convert_stroke(
                    &s,
                    stream_builder.serializer_context().clone(),
                    font_context,
                )
            });

            let transform = glyph.transform().pre_concat(Transform::from_scale(
                font.units_per_em() as f32 / span.font_size.get(),
                font.units_per_em() as f32 / span.font_size.get(),
            ));

            match (fill, stroke) {
                (Some(fill), Some(stroke)) => match span.paint_order {
                    PaintOrder::FillAndStroke => {
                        stream_builder.fill_glyph(
                            GlyphId::new(glyph.id.0 as u32),
                            font.clone(),
                            FiniteF32::new(span.font_size.get()).unwrap(),
                            &convert_transform(&transform),
                            &fill,
                        );
                        stream_builder.stroke_glyph(
                            GlyphId::new(glyph.id.0 as u32),
                            font,
                            FiniteF32::new(span.font_size.get()).unwrap(),
                            &convert_transform(&transform),
                            &stroke,
                        );
                    }
                    PaintOrder::StrokeAndFill => {
                        stream_builder.stroke_glyph(
                            GlyphId::new(glyph.id.0 as u32),
                            font.clone(),
                            FiniteF32::new(span.font_size.get()).unwrap(),
                            &convert_transform(&transform),
                            &stroke,
                        );
                        stream_builder.fill_glyph(
                            GlyphId::new(glyph.id.0 as u32),
                            font,
                            FiniteF32::new(span.font_size.get()).unwrap(),
                            &convert_transform(&transform),
                            &fill,
                        );
                    }
                },
                (Some(fill), None) => {
                    stream_builder.fill_glyph(
                        GlyphId::new(glyph.id.0 as u32),
                        font,
                        FiniteF32::new(span.font_size.get()).unwrap(),
                        &convert_transform(&transform),
                        &fill,
                    );
                }
                (None, Some(stroke)) => {
                    stream_builder.stroke_glyph(
                        GlyphId::new(glyph.id.0 as u32),
                        font,
                        FiniteF32::new(span.font_size.get()).unwrap(),
                        &convert_transform(&transform),
                        &stroke,
                    );
                }
                (None, None) => stream_builder.invisible_glyph(
                    GlyphId::new(glyph.id.0 as u32),
                    font,
                    FiniteF32::new(span.font_size.get()).unwrap(),
                    &convert_transform(&transform),
                ),
            }
        }

        if let Some(line_through) = &span.line_through {
            path::render(line_through, stream_builder, font_context);
        }
    }
}
