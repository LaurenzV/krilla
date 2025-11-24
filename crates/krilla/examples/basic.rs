//! A basic example of a PDF file created with krilla.

use std::path;
use std::path::PathBuf;

use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::Fill;
use krilla::paint::{FillRule, LinearGradient, SpreadMethod, Stop};
use krilla::text::Font;
use krilla::text::TextDirection;
use krilla::Document;

fn main() {
    // Create a new document.
    let mut document = Document::new();
    // Load a font.
    let font = {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/fonts/NotoSans-Regular.ttf");
        let data = std::fs::read(&path).unwrap();
        Font::new(data.into(), 0).unwrap()
    };

    // Add a new page with dimensions 200x200.
    let mut page = document.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());
    // Get the surface of the page.
    let mut surface = page.surface();
    // Draw some text.
    surface.draw_text(
        Point::from_xy(0.0, 25.0),
        font.clone(),
        14.0,
        "This text has font size 14!",
        false,
        TextDirection::Auto,
    );

    surface.set_fill(Some(Fill {
        paint: rgb::Color::new(255, 0, 0).into(),
        opacity: NormalizedF32::new(0.5).unwrap(),
        rule: Default::default(),
    }));
    // Draw some more text, in a different color with an opacity and bigger font size.
    surface.draw_text(
        Point::from_xy(0.0, 50.0),
        font.clone(),
        16.0,
        "This text has font size 16!",
        false,
        TextDirection::Auto,
    );

    // Finish the page.
    surface.finish();
    page.finish();

    // Start a new page.
    let mut page = document.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());
    // Create the triangle.
    let triangle = {
        let mut pb = PathBuilder::new();
        pb.move_to(100.0, 20.0);
        pb.line_to(160.0, 160.0);
        pb.line_to(40.0, 160.0);
        pb.close();

        pb.finish().unwrap()
    };

    // Create the linear gradient.
    let lg = LinearGradient {
        x1: 60.0,
        y1: 0.0,
        x2: 140.0,
        y2: 0.0,
        transform: Default::default(),
        spread_method: SpreadMethod::Repeat,
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
        anti_alias: false,
    };
    let mut surface = page.surface();

    // Set the fill.
    surface.set_fill(Some(Fill {
        paint: lg.into(),
        rule: FillRule::EvenOdd,
        opacity: NormalizedF32::ONE,
    }));

    // Fill the path.
    surface.draw_path(&triangle);

    // Finish up and write the resulting PDF.
    surface.finish();
    page.finish();

    let pdf = document.finish().unwrap();
    let path = path::absolute("basic.pdf").unwrap();
    eprintln!("Saved PDF to '{}'", path.display());

    // Write the PDF to a file.
    std::fs::write(path, &pdf).unwrap();
}
