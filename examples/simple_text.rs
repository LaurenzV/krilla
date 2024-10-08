//! This example shows how to use `simple_text` API to draw simple text in a single line. It
//! also demonstrates how you can use non-default variation coordinates.
//!
//! Note that simple text in this case does not mean that complex scripts aren't supported
//! (they are, including RTL text!), but the text itself must not contain mixed scripts. And
//! the font must contain all necessary glyphs, otherwise the `.notdef` glyph will be emitted
//! instead of font fallback.

use krilla::color::rgb;
use krilla::font::Font;
use krilla::paint::{LinearGradient, SpreadMethod, Stop};
use krilla::path::Fill;
use krilla::surface::TextDirection;
use krilla::{Document, PageSettings};
use std::sync::Arc;
use tiny_skia_path::Point;
use usvg::NormalizedF32;

fn main() {
    // The usual page setup.
    let mut document = Document::new();
    let mut page = document.start_page_with(PageSettings::new(600.0, 280.0));
    let mut surface = page.surface();

    let noto_font = Font::new(
        Arc::new(std::fs::read("assets/fonts/NotoSans-Regular.ttf").unwrap()),
        0,
        vec![],
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
                color: rgb::Color::new(255, 0, 0),
                opacity: NormalizedF32::ONE,
            },
            Stop {
                offset: NormalizedF32::new(0.8).unwrap(),
                color: rgb::Color::new(255, 255, 0),
                opacity: NormalizedF32::ONE,
            },
        ]
        .into(),
    };

    // Let's first write some red-colored text with some English text.
    surface.fill_text(
        Point::from_xy(0.0, 25.0),
        Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::new(0.5).unwrap(),
            rule: Default::default(),
        },
        noto_font.clone(),
        25.0,
        &[],
        "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔͆̏̓͢",
        false,
        TextDirection::Auto,
    );
    //
    // // Instead of applying fills, we can also apply strokes!
    // surface.stroke_text(
    //     Point::from_xy(0.0, 50.0),
    //     Stroke {
    //         paint: rgb::Color::new(0, 255, 0).into(),
    //         ..Default::default()
    //     },
    //     noto_font.clone(),
    //     25.0,
    //     &[],
    //     "This text is stroked green!",
    //     false,
    //     TextDirection::Auto,
    // );
    //
    // let noto_arabic_font = Font::new(
    //     Arc::new(std::fs::read("assets/fonts/NotoSansArabic-Regular.ttf").unwrap()),
    //     0,
    //     vec![],
    // )
    // .unwrap();
    //
    // // As mentioned above, complex scripts are supported, you just can't mix them in
    // // one run.
    // surface.fill_text(
    //     Point::from_xy(0.0, 75.0),
    //     Fill::default(),
    //     noto_arabic_font.clone(),
    //     25.0,
    //     &[],
    //     "هذا هو السطر الثاني من النص.",
    //     false,
    //     TextDirection::Auto,
    // );
    //
    // let mut y = 125.0;
    //
    // let axes = vec![
    //     vec![("wght".to_string(), 100.0), ("wdth".to_string(), 62.5)],
    //     vec![("wght".to_string(), 233.33), ("wdth".to_string(), 68.75)],
    //     vec![("wght".to_string(), 366.67), ("wdth".to_string(), 75.0)],
    //     vec![("wght".to_string(), 500.0), ("wdth".to_string(), 81.25)],
    //     vec![("wght".to_string(), 633.33), ("wdth".to_string(), 87.5)],
    //     vec![("wght".to_string(), 766.67), ("wdth".to_string(), 93.75)],
    //     vec![("wght".to_string(), 900.0), ("wdth".to_string(), 100.0)],
    // ];
    //
    // // krilla also supports variable fonts!
    // for coordinates in axes {
    //     let noto_font_variable = Font::new(
    //         Arc::new(std::fs::read("assets/fonts/NotoSans_variable.ttf").unwrap()),
    //         0,
    //         coordinates,
    //     )
    //     .unwrap();
    //
    //     surface.fill_text(
    //         Point::from_xy(0.0, y),
    //         Fill {
    //             paint: rgb::Color::new(0, 0, 255).into(),
    //             ..Default::default()
    //         },
    //         noto_font_variable.clone(),
    //         25.0,
    //         &[],
    //         "Variable fonts rock!",
    //         false,
    //         TextDirection::Auto,
    //     );
    //
    //     y += 25.0;
    // }

    surface.finish();
    page.finish();
    let pdf = document.finish().unwrap();

    std::fs::write("target/simple_text.pdf", &pdf).unwrap();
}
