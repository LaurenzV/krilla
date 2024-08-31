//! This example shows how to use cosmic-text to create advanced layouted text.
//!
//! It is unfortunately still somewhat hard if you are not familiar with text
//! shaping/layouting, but using this code as a template should hopefully help
//! you get started.
//!
//! Another important point to mention is that you need to ensure that the
//! version of `cosmic-text` you use uses the same `fontdb` version as krilla.
//! For this example, we are using [my fork](https://github.com/LaurenzV/cosmic-text)
//! that I try to keep in sync.

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use krilla::color::rgb;
use krilla::color::rgb::Rgb;
use krilla::font::{Font, Glyph};
use krilla::paint::Paint;
use krilla::path::Fill;
use krilla::{Document, PageSettings};
use parley::layout::Alignment;
use parley::style::{FontFamily, FontStack, FontWeight, StyleProperty};
use parley::{FontContext, LayoutContext};
use skrifa::instance::Location;
use skrifa::GlyphId;
use std::alloc::Layout;
use std::collections::HashMap;
use skrifa::raw::collections::int_set::Domain;
use tiny_skia_path::Point;
use usvg::NormalizedF32;

fn main() {
    let text = String::from(
        "This is a long text. We want it to not be wider than 200pt, \
    so that it fits on the page. Let's intersperse some emojis 💩👻💀emojis🦩🌚😁😆\
    as well as complex scripts: हैलो वर्ल्ड and مرحبا بالعالم",
    );

    // The width for line wrapping
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

    // Set the first 4 characters to bold
    let bold = FontWeight::new(600.0);
    let bold_style = StyleProperty::FontWeight(bold);
    builder.push(&bold_style, 0..4);

    // Set next 4 characters to red.
    let color_style = StyleProperty::Brush(rgb::Color::new(255, 0, 0));
    builder.push(&color_style, 5..12);

    let mut layout = builder.build();
    layout.break_all_lines(max_advance);
    layout.align(max_advance, Alignment::Start);

    let mut font_cache = HashMap::new();
    let mut document = Document::new();
    let mut page = document.start_page_with(PageSettings::with_size(200.0, 300.0));
    let mut surface = page.surface();

    for line in layout.lines() {
        let y = line.metrics().baseline;
        let mut x = 0.0;
        for run in line.runs() {
            let font = run.font().clone();
            let (font_data, id) = font.data.into_raw_parts();
            let krilla_font = font_cache
                .entry(id)
                .or_insert_with(|| Font::new(font_data, font.index, Location::default()).unwrap());
            let font_size = run.font_size();

            let mut glyphs = vec![];

            for cluster in run.visual_clusters() {
                for glyph in cluster.glyphs() {
                    glyphs.push(Glyph::new(GlyphId::new(glyph.id.to_u32()), glyph.advance, glyph.x, glyph.y, cluster.text_range(), font_size))
                }
            }

            surface.fill_glyphs(
                Point::from_xy(x, y),
                Fill::<Rgb>::default(),
                &glyphs,
                krilla_font.clone(),
                &text
            );

            x += run.advance()
        }
    }

    surface.finish();
    page.finish();

    let pdf = document.finish().unwrap();
    // Write the resulting PDF!
    std::fs::write("target/parley.pdf", &pdf).unwrap();
}
