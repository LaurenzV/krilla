use crate::font::KrillaGlyph;
use crate::font::{Font, GlyphUnits};
use crate::object::color::rgb;
use crate::object::color::rgb::Rgb;
use crate::paint::Paint;
use crate::path::{Fill, Stroke};
use crate::surface::Surface;
use crate::svg::util::{convert_fill, convert_stroke};
use crate::svg::{path, ProcessContext};
use skrifa::GlyphId;
use tiny_skia_path::{Point, Transform};
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
            let font = process_context.fonts.get(&glyph.font).cloned().unwrap();
            let upem = font.units_per_em();

            // The text transform contains the scale transform `font_size / upem`, we need to invert that
            // so we only get the raw transform to account for the glyph position, and the font size
            // is being taken care of by krilla.
            let transform = glyph.transform().pre_concat(Transform::from_scale(
                upem / span.font_size.get(),
                upem / span.font_size.get(),
            ));

            // We need to apply the inverse transform to fill/stroke because we don't
            // want the paint to be affected by the transform applied to the glyph. See docs
            // of `convert_paint`.
            let fill = span.fill.as_ref().map(|f| {
                convert_fill(
                    f,
                    surface.stream_builder(),
                    process_context,
                    transform.invert().unwrap(),
                )
            });

            let stroke = span.stroke.as_ref().map(|s| {
                convert_stroke(
                    s,
                    surface.stream_builder(),
                    process_context,
                    transform.invert().unwrap(),
                )
            });

            let fill_op = |sb: &mut Surface, fill: Fill<Rgb>, font: Font| {
                sb.fill_glyphs(
                    Point::from_xy(0.0, 0.0),
                    fill,
                    &[KrillaGlyph::new(
                        GlyphId::new(glyph.id.0 as u32),
                        // Don't care about those, since we render only one glyph.
                        0.0,
                        0.0,
                        0.0,
                        0..glyph.text.len(),
                    )],
                    font,
                    &glyph.text,
                    span.font_size.get(),
                    GlyphUnits::UnitsPerEm,
                    false,
                );
            };

            let stroke_op = |sb: &mut Surface, stroke: Stroke<Rgb>, font: Font| {
                sb.stroke_glyphs(
                    Point::from_xy(0.0, 0.0),
                    stroke,
                    &[KrillaGlyph::new(
                        GlyphId::new(glyph.id.0 as u32),
                        // Don't care about those, since we render only one glyph.
                        0.0,
                        0.0,
                        0.0,
                        0..glyph.text.len(),
                    )],
                    font,
                    &glyph.text,
                    span.font_size.get(),
                    GlyphUnits::UnitsPerEm,
                );
            };

            surface.push_transform(&transform);

            match (fill, stroke) {
                (Some(fill), Some(stroke)) => match span.paint_order {
                    PaintOrder::FillAndStroke => {
                        fill_op(surface, fill, font.clone());
                        stroke_op(surface, stroke, font);
                    }
                    PaintOrder::StrokeAndFill => {
                        stroke_op(surface, stroke, font.clone());
                        fill_op(surface, fill, font);
                    }
                },
                (Some(fill), None) => {
                    fill_op(surface, fill, font);
                }
                (None, Some(stroke)) => {
                    stroke_op(surface, stroke, font);
                }
                // Emulate invisible glyph by drawing it with an opacity of zero.
                (None, None) => fill_op(
                    surface,
                    Fill {
                        paint: Paint::Color(rgb::Color::new(0, 0, 0)),
                        opacity: NormalizedF32::ZERO,
                        rule: Default::default(),
                    },
                    font,
                ),
            }

            surface.pop();
        }

        if let Some(line_through) = &span.line_through {
            path::render(line_through, surface, process_context);
        }
    }
}
