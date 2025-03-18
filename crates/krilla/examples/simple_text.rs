//! This example shows how to use `simple_text` API to draw simple text in a single line. It
//! also demonstrates how you can use non-default variation coordinates.
//!
//! Note that simple text in this case does not mean that complex scripts aren't supported
//! (they are, including RTL text!), but the text itself must not contain mixed scripts. And
//! the font must contain all necessary glyphs, otherwise the `.notdef` glyph will be emitted
//! instead of font fallback.

use std::path;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use krilla::color::rgb;
use krilla::geom::Point;
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, LinearGradient, SpreadMethod, Stop, Stroke};
use krilla::text::{Font, TextDirection};
use krilla::Document;
use once_cell::sync::Lazy;

pub(crate) static WORKSPACE_PATH: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../"));

pub(crate) static ASSETS_PATH: LazyLock<PathBuf> = LazyLock::new(|| WORKSPACE_PATH.join("assets"));

fn main() {
    // The usual page setup.
    let mut document = Document::new();
    let mut page = document.start_page_with(PageSettings::new(600.0, 280.0));
    let mut surface = page.surface();

    let noto_font = Font::new(
        Arc::new(std::fs::read(ASSETS_PATH.join("fonts/NotoSans-Regular.ttf")).unwrap()).into(),
        0,
        true,
    )
    .unwrap();

    let gradient = LinearGradient {
        x1: 30.0,
        y1: 0.0,
        x2: 50.0,
        y2: 0.0,
        transform: Default::default(),
        spread_method: SpreadMethod::Reflect,
        stops: vec![
            Stop {
                offset: NormalizedF32::new(0.2).unwrap(),
                color: rgb::Color::new(255, 0, 0).into(),
                opacity: NormalizedF32::ONE,
            },
            Stop {
                offset: NormalizedF32::new(0.8).unwrap(),
                color: rgb::Color::new(255, 255, 0).into(),
                opacity: NormalizedF32::ONE,
            },
        ],
        anti_alias: true,
    };

    surface.set_fill(Fill {
        paint: gradient.into(),
        opacity: NormalizedF32::new(0.5).unwrap(),
        rule: Default::default(),
    });

    // Let's first write some red-colored text with some English text.
    surface.fill_text(
        Point::from_xy(0.0, 25.0),
        noto_font.clone(),
        25.0,
        "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔͆̏̓͢",
        false,
        TextDirection::Auto,
    );

    surface.set_stroke(Stroke {
        paint: rgb::Color::new(0, 255, 0).into(),
        ..Default::default()
    });
    // Instead of applying fills, we can also apply strokes!
    surface.stroke_text(
        Point::from_xy(0.0, 50.0),
        noto_font.clone(),
        25.0,
        "This text is stroked green!",
        false,
        TextDirection::Auto,
    );

    let noto_arabic_font = Font::new(
        Arc::new(std::fs::read(ASSETS_PATH.join("fonts/NotoSansArabic-Regular.ttf")).unwrap())
            .into(),
        0,
        true,
    )
    .unwrap();

    surface.set_fill(Fill::default());
    // As mentioned above, complex scripts are supported, you just can't mix them in
    // one run.
    surface.fill_text(
        Point::from_xy(0.0, 75.0),
        noto_arabic_font.clone(),
        25.0,
        "هذا هو السطر الثاني من النص.",
        false,
        TextDirection::Auto,
    );

    surface.finish();
    page.finish();
    let pdf = document.finish().unwrap();

    let path = path::absolute("simple_text.pdf").unwrap();
    eprintln!("Saved PDF to '{}'", path.display());

    // Write the PDF to a file.
    std::fs::write(path, &pdf).unwrap();
}
