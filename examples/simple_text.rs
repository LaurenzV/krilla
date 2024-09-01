//! This example shows how to use `simple_text` API to draw simple text in a single line.
//!
//! Note that simple text in this case does not mean that complex scripts aren't supported
//! (they are, including RTL text!), but the text itself must not contain mixed scripts. And
//! the font must contain all necessary glyphs, otherwise the `.notdef` glyph will be emitted
//! instead of font fallback.

use krilla::color::rgb;
use krilla::color::rgb::Rgb;
use krilla::font::Font;
use krilla::paint::Paint;
use krilla::path::{Fill, Stroke};
use krilla::{Document, PageSettings};
use std::sync::Arc;
use tiny_skia_path::Point;
use usvg::NormalizedF32;

fn main() {
    let noto_font = Font::new(
        Arc::new(std::fs::read("assets/fonts/NotoSans-Regular.ttf").unwrap()),
        0,
        vec![],
    )
    .unwrap();
    let noto_arabic_font = Font::new(
        Arc::new(std::fs::read("assets/fonts/NotoSansArabic-Regular.ttf").unwrap()),
        0,
        vec![],
    )
    .unwrap();

    // The usual page setup.
    let mut document = Document::new();
    let mut page = document.start_page_with(PageSettings::with_size(600.0, 300.0));
    let mut surface = page.surface();

    surface.fill_text(
        Point::from_xy(0.0, 25.0),
        Fill {
            paint: Paint::<Rgb>::Color(rgb::Color::new(255, 0, 0)),
            opacity: NormalizedF32::new(0.5).unwrap(),
            rule: Default::default(),
        },
        noto_font.clone(),
        25.0,
        &[],
        "This text is filled red and has opacity.",
    );

    surface.stroke_text(
        Point::from_xy(0.0, 50.0),
        Stroke {
            paint: Paint::<Rgb>::Color(rgb::Color::new(0, 255, 0)),
            width: 0.0,
            ..Default::default()
        },
        noto_font.clone(),
        25.0,
        &[],
        "This text is stroked green!",
    );

    surface.fill_text(
        Point::from_xy(0.0, 75.0),
        Fill::<Rgb>::default(),
        noto_arabic_font.clone(),
        25.0,
        &[],
        "هذا هو السطر الثاني من النص.",
    );

    // Finish up.
    surface.finish();
    page.finish();
    let pdf = document.finish().unwrap();

    // Write the resulting PDF!
    std::fs::write("target/simple_text.pdf", &pdf).unwrap();
}
