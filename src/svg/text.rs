use crate::object::color_space::srgb::Srgb;
use crate::stream::TestGlyph;
use crate::surface::Surface;
use crate::svg::util::{convert_fill, convert_stroke};
use crate::svg::{path, FontContext};
use crate::{Fill, Stroke};
use skrifa::GlyphId;
use tiny_skia_path::Transform;
use usvg::PaintOrder;

pub fn render(text: &usvg::Text, canvas_builder: &mut Surface, font_context: &mut FontContext) {
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
            let (font, upem) = font_context.fonts.get(&glyph.font).copied().unwrap();

            let transform = glyph.transform().pre_concat(Transform::from_scale(
                upem as f32 / span.font_size.get(),
                upem as f32 / span.font_size.get(),
            ));

            // We need to apply the inverse transform to fill/stroke because we don't
            // want the paint to be affected by the transform applied to the glyph.
            let fill = span.fill.as_ref().map(|f| {
                convert_fill(
                    &f,
                    canvas_builder.stream_surface(),
                    font_context,
                    transform.invert().unwrap(),
                )
            });
            let stroke = span.stroke.as_ref().map(|s| {
                convert_stroke(
                    &s,
                    canvas_builder.stream_surface(),
                    font_context,
                    transform.invert().unwrap(),
                )
            });

            let fill_op = |sb: &mut Surface, fill: &Fill<Srgb>, font_context: &mut FontContext| {
                sb.fill_glyph_run(
                    0.0,
                    0.0,
                    font_context.fontdb,
                    &fill,
                    [TestGlyph::new(
                        font,
                        GlyphId::new(glyph.id.0 as u32),
                        0.0,
                        0.0,
                        span.font_size.get(),
                        glyph.text.clone(),
                    )]
                    .into_iter()
                    .peekable(),
                );
            };

            let stroke_op =
                |sb: &mut Surface, stroke: &Stroke<Srgb>, font_context: &mut FontContext| {
                    sb.stroke_glyph_run(
                        0.0,
                        0.0,
                        font_context.fontdb,
                        &stroke,
                        [TestGlyph::new(
                            font,
                            GlyphId::new(glyph.id.0 as u32),
                            0.0,
                            0.0,
                            span.font_size.get(),
                            glyph.text.clone(),
                        )]
                        .into_iter()
                        .peekable(),
                    );
                };

            canvas_builder.push_transform(&transform);

            match (fill, stroke) {
                (Some(fill), Some(stroke)) => match span.paint_order {
                    PaintOrder::FillAndStroke => {
                        fill_op(canvas_builder, &fill, font_context);
                        stroke_op(canvas_builder, &stroke, font_context);
                    }
                    PaintOrder::StrokeAndFill => {
                        stroke_op(canvas_builder, &stroke, font_context);
                        fill_op(canvas_builder, &fill, font_context);
                    }
                },
                (Some(fill), None) => {
                    fill_op(canvas_builder, &fill, font_context);
                }
                (None, Some(stroke)) => {
                    stroke_op(canvas_builder, &stroke, font_context);
                }
                (None, None) => canvas_builder.invisible_glyph_run(
                    0.0,
                    0.0,
                    font_context.fontdb,
                    [TestGlyph::new(
                        font,
                        GlyphId::new(glyph.id.0 as u32),
                        0.0,
                        0.0,
                        span.font_size.get(),
                        glyph.text.clone(),
                    )]
                    .into_iter()
                    .peekable(),
                ),
            }

            canvas_builder.pop_transform();
        }

        if let Some(line_through) = &span.line_through {
            path::render(line_through, canvas_builder, font_context);
        }
    }
}
