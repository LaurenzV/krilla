use crate::object::color_space::rgb;
use crate::object::color_space::rgb::Rgb;
use crate::stream::TestGlyph;
use crate::surface::Surface;
use crate::svg::util::{convert_fill, convert_stroke};
use crate::svg::{path, ProcessContext};
use crate::{Fill, Paint, Stroke};
use skrifa::GlyphId;
use tiny_skia_path::Transform;
use usvg::{NormalizedF32, PaintOrder};

/// Render a text into a surface.
pub fn render(text: &usvg::Text, surface: &mut Surface, process_context: &mut ProcessContext) {
    // TODO: Add possibility to render as paths instead
    for span in text.layouted() {
        if !span.visible {
            continue;
        }

        if let Some(overline) = &span.overline {
            path::render(overline, surface, process_context);
        }

        if let Some(underline) = &span.underline {
            path::render(underline, surface, process_context);
        }

        for glyph in &span.positioned_glyphs {
            let (font, upem) = process_context.fonts.get(&glyph.font).copied().unwrap();

            // The text transform contains the scale transform `font_size / upem`, we need to invert that
            // so we only get the raw transform to account for the glyph position, and the font size
            // is being taken care of by krilla.
            let transform = glyph.transform().pre_concat(Transform::from_scale(
                upem as f32 / span.font_size.get(),
                upem as f32 / span.font_size.get(),
            ));

            // We need to apply the inverse transform to fill/stroke because we don't
            // want the paint to be affected by the transform applied to the glyph. See docs
            // of `convert_paint`.
            let fill = span.fill.as_ref().map(|f| {
                convert_fill(
                    &f,
                    surface.stream_surface(),
                    process_context,
                    transform.invert().unwrap(),
                )
            });

            let stroke = span.stroke.as_ref().map(|s| {
                convert_stroke(
                    &s,
                    surface.stream_surface(),
                    process_context,
                    transform.invert().unwrap(),
                )
            });

            let fill_op =
                |sb: &mut Surface, fill: Fill<Rgb>, process_context: &mut ProcessContext| {
                    sb.fill_glyph_run(
                        0.0,
                        0.0,
                        process_context.krilla_fontdb,
                        fill,
                        [TestGlyph::new(
                            font,
                            GlyphId::new(glyph.id.0 as u32),
                            // Don't care about those, since we render only one glyph.
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
                |sb: &mut Surface, stroke: Stroke<Rgb>, process_context: &mut ProcessContext| {
                    sb.stroke_glyph_run(
                        0.0,
                        0.0,
                        process_context.krilla_fontdb,
                        stroke,
                        [TestGlyph::new(
                            font,
                            GlyphId::new(glyph.id.0 as u32),
                            // Don't care about those, since we render only one glyph.
                            0.0,
                            0.0,
                            span.font_size.get(),
                            glyph.text.clone(),
                        )]
                        .into_iter()
                        .peekable(),
                    );
                };

            surface.push_transform(&transform);

            match (fill, stroke) {
                (Some(fill), Some(stroke)) => match span.paint_order {
                    PaintOrder::FillAndStroke => {
                        fill_op(surface, fill, process_context);
                        stroke_op(surface, stroke, process_context);
                    }
                    PaintOrder::StrokeAndFill => {
                        stroke_op(surface, stroke, process_context);
                        fill_op(surface, fill, process_context);
                    }
                },
                (Some(fill), None) => {
                    fill_op(surface, fill, process_context);
                }
                (None, Some(stroke)) => {
                    stroke_op(surface, stroke, process_context);
                }
                // Emulate invisible glyph by drawing it with an opacity of zero.
                (None, None) => fill_op(
                    surface,
                    Fill {
                        paint: Paint::Color(rgb::Color::new(0, 0, 0)),
                        opacity: NormalizedF32::ZERO,
                        rule: Default::default(),
                    },
                    process_context,
                ),
            }

            surface.pop();
        }

        if let Some(line_through) = &span.line_through {
            path::render(line_through, surface, process_context);
        }
    }
}
