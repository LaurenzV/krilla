//! This example shows how to use parley to create advanced layouted text.
//!
//! Similarly to the `cosmic-text` example, it might be a bit hard to grasph if you
//! are not familiar with text shaping/layouting, but using this code as a template
//! should hopefully help you get started. It should be noted that this example also
//! only supports basic glyphs with certain properties (like colors, font weight, etc.)
//! but not other parley features like underline, strike-through, etc.

use krilla::color::rgb;
use krilla::font::{Font, GlyphId, GlyphUnits, KrillaGlyph};
use krilla::path::Fill;
use krilla::{Document, PageSettings};
use parley::layout::Alignment;
use parley::style::{FontFamily, FontStack, FontWeight, StyleProperty};
use parley::{FontContext, LayoutContext};
use std::collections::HashMap;
use krilla::geom::{NormalizedF32, Point};

fn main() {
    // The text that we want to insert into the PDF.
    let text = String::from(
        "This is a long text. We want it to not be wider than 200pt, \
    so that it fits on the page. Let's intersperse some emojis 💩👻💀emojis🦩🌚😁😆\
    as well as complex scripts: हैलो वर्ल्ड and مرحبا بالعالم",
    );

    // Set up the properties of the text. See the documentation of parley for more information
    // on how you can configure its behavior.
    let max_advance = Some(200.0);
    let text_color = rgb::Color::new(0, 0, 0);
    let mut font_cx = FontContext::default();
    let mut layout_cx = LayoutContext::new();
    let mut builder = layout_cx.ranged_builder(&mut font_cx, &text, 1.0);
    let brush_style = StyleProperty::Brush(text_color);
    builder.push_default(&brush_style);

    let font_stack = FontStack::List(&[
        FontFamily::Named("Noto Sans"),
        FontFamily::Named("Noto Sans Arabic"),
        FontFamily::Named("Noto Sans Devanagari"),
        FontFamily::Named("Noto Color Emoji"),
    ]);
    let font_stack_style = StyleProperty::FontStack(font_stack);
    builder.push_default(&font_stack_style);
    builder.push_default(&StyleProperty::LineHeight(1.3));
    builder.push_default(&StyleProperty::FontSize(16.0));

    // In our case, we set the first four characters to bold and also make some
    // part of the text red.
    let bold = FontWeight::new(600.0);
    let bold_style = StyleProperty::FontWeight(bold);
    builder.push(&bold_style, 0..4);

    let color_style = StyleProperty::Brush(rgb::Color::new(255, 0, 0));
    builder.push(&color_style, 2..12);

    let mut layout = builder.build();
    layout.break_all_lines(max_advance);
    layout.align(max_advance, Alignment::Start);

    // After setting up everything, now starts the actual part where we use krilla to write
    // the text to a PDF.
    // We need to set up a font cache that converts from parley fonts to krilla fonts.
    let mut font_cache = HashMap::new();

    // The usual page setup.
    let mut document = Document::new();
    let mut page = document.start_page_with(PageSettings::new(200.0, 300.0));
    let mut surface = page.surface();

    for line in layout.lines() {
        let y = line.metrics().baseline;
        let mut x = 0.0;
        for run in line.runs() {
            let mut cur_x = x;
            let font = run.font().clone();
            let (font_data, id) = font.data.into_raw_parts();
            // Get the krilla font.
            let krilla_font = font_cache
                .entry(id)
                .or_insert_with(|| Font::new(font_data, font.index, vec![]).unwrap());
            let font_size = run.font_size();

            // This is part is somewhat convoluted, the reason being that each glyph might
            // have a different style than the previous one. So we always need to keep track
            // of the current style, and whenever we encounter a new style we "flush" all
            // current glyphs into the PDF, and build the next sequence of consecutive
            // glyphs.
            let mut cur_style = None;
            let mut glyphs = vec![];

            for cluster in run.visual_clusters() {
                for glyph in cluster.glyphs() {
                    let glyph_style = glyph.style_index;

                    if let Some(style) = cur_style {
                        if style != glyph_style {
                            // If style doesn't match, flush all glyphs up to now.
                            cur_style = Some(glyph_style);
                            let style = layout.styles()[style as usize].brush;
                            surface.fill_glyphs(
                                Point::from_xy(cur_x, y),
                                Fill {
                                    paint: style.into(),
                                    opacity: NormalizedF32::ONE,
                                    rule: Default::default(),
                                },
                                &glyphs,
                                krilla_font.clone(),
                                &text,
                                font_size,
                                GlyphUnits::UserSpace,
                                false,
                            );
                            glyphs.clear();
                            cur_x = x;
                        }
                    } else {
                        cur_style = Some(glyph_style);
                    }

                    // Add the current glyph to our buffer of glyphs.
                    glyphs.push(KrillaGlyph::new(
                        GlyphId::new(glyph.id as u32),
                        glyph.advance,
                        glyph.x,
                        glyph.y,
                        0.0,
                        cluster.text_range(),
                    ));
                    // And make sure keep track of the current x position.
                    x += glyph.advance;
                }
            }

            // Flush all remaining glyphs, if existing.
            if !glyphs.is_empty() {
                surface.fill_glyphs(
                    Point::from_xy(cur_x, y),
                    Fill {
                        paint: layout.styles()[cur_style.unwrap() as usize].brush.into(),
                        opacity: NormalizedF32::ONE,
                        rule: Default::default(),
                    },
                    &glyphs,
                    krilla_font.clone(),
                    &text,
                    font_size,
                    GlyphUnits::UserSpace,
                    false,
                );
            }
        }
    }

    surface.finish();
    page.finish();

    let pdf = document.finish().unwrap();
    std::fs::write("../../target/parley.pdf", &pdf).unwrap();
}
