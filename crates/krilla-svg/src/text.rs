//! Text conversion

use krilla::color::rgb;
use krilla::font::{GlyphId, GlyphUnits, KrillaGlyph};
use krilla::path::{Fill, Stroke};
use krilla::surface::Surface;
use krilla::{Font, NormalizedF32, Point};
use usvg::tiny_skia_path::Transform;
use usvg::PaintOrder;

use crate::util::{convert_fill, convert_stroke, UsvgTransformExt};
use crate::{path, ProcessContext};

/// Render a text into a surface.
pub(crate) fn render(
    text: &usvg::Text,
    surface: &mut Surface,
    process_context: &mut ProcessContext,
) {
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
            // Ignore glyph if font can't be fetched.
            let Some(font) = process_context.fonts.get(&glyph.font).cloned() else {
                continue;
            };

            let upem = font.units_per_em();

            // The text transform contains the scale transform `font_size / upem`, we need to invert that
            // so we only get the raw transform to account for the glyph position, and the font size
            // is being taken care of by krilla.
            let transform = glyph.transform().pre_concat(Transform::from_scale(
                upem / span.font_size.get(),
                upem / span.font_size.get(),
            ));

            let Some(inverted) = transform.invert() else {
                continue;
            };

            // We need to apply the inverse transform to fill/stroke because we don't
            // want the paint to be affected by the transform applied to the glyph. See docs
            // of `convert_paint`.
            let fill = span
                .fill
                .as_ref()
                .map(|f| convert_fill(f, surface.stream_builder(), process_context, inverted));

            let stroke = span
                .stroke
                .as_ref()
                .map(|s| convert_stroke(s, surface.stream_builder(), process_context, inverted));

            let fill_op = |sb: &mut Surface, fill: Fill, font: Font, embed_text: bool| {
                sb.fill_glyphs(
                    Point::from_xy(0.0, 0.0),
                    fill,
                    &[KrillaGlyph::new(
                        GlyphId::new(glyph.id.0 as u32),
                        // Don't care about those, since we render only one glyph.
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        0..glyph.text.len(),
                        None,
                    )],
                    font,
                    &glyph.text,
                    span.font_size.get(),
                    GlyphUnits::UnitsPerEm,
                    !embed_text,
                );
            };

            let stroke_op = |sb: &mut Surface, stroke: Stroke, font: Font, embed_text: bool| {
                sb.stroke_glyphs(
                    Point::from_xy(0.0, 0.0),
                    stroke,
                    &[KrillaGlyph::new(
                        GlyphId::new(glyph.id.0 as u32),
                        // Don't care about those, since we render only one glyph.
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        0..glyph.text.len(),
                        None,
                    )],
                    font,
                    &glyph.text,
                    span.font_size.get(),
                    GlyphUnits::UnitsPerEm,
                    !embed_text,
                );
            };

            surface.push_transform(&transform.to_krilla());

            match (fill, stroke) {
                (Some(fill), Some(stroke)) => match span.paint_order {
                    // We always outline strokes in this case,
                    // so that text won't be selected two times.
                    PaintOrder::FillAndStroke => {
                        fill_op(
                            surface,
                            fill,
                            font.clone(),
                            process_context.svg_settings.embed_text,
                        );
                        stroke_op(surface, stroke, font, false);
                    }
                    PaintOrder::StrokeAndFill => {
                        stroke_op(surface, stroke, font.clone(), false);
                        fill_op(surface, fill, font, process_context.svg_settings.embed_text);
                    }
                },
                (Some(fill), None) => {
                    fill_op(surface, fill, font, process_context.svg_settings.embed_text);
                }
                (None, Some(stroke)) => {
                    stroke_op(
                        surface,
                        stroke,
                        font,
                        process_context.svg_settings.embed_text,
                    );
                }
                // Emulate invisible glyph by drawing it with an opacity of zero.
                (None, None) => fill_op(
                    surface,
                    Fill {
                        paint: rgb::Color::new(0, 0, 0).into(),
                        opacity: NormalizedF32::ZERO,
                        rule: Default::default(),
                    },
                    font,
                    process_context.svg_settings.embed_text,
                ),
            }

            surface.pop();
        }

        if let Some(line_through) = &span.line_through {
            path::render(line_through, surface, process_context);
        }
    }
}
