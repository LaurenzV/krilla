//! This example introduces the concept of a `StreamBuilder`, which allows you to
//! define graphics content on a separate context than the main page surface. This is necessary
//! when defining patterns or mask.

use krilla::color::rgb;
use krilla::color::rgb::Rgb;
use krilla::geom::Transform;
use krilla::paint::{Paint, Pattern};
use krilla::path::{Fill, Stroke};
use krilla::{Document, PageSettings};
use tiny_skia_path::{Path, PathBuilder, Rect};

fn main() {
    let mut document = Document::new();
    let mut page = document.start_page_with(PageSettings::with_size(200.0, 200.0));
    let mut surface = page.surface();

    // We want to define a pattern with a red rectangle on the top-left and a
    // blue rectangle on the bottom-right. Then we want to apply this pattern to
    // a rectangle that is rotated by 45 degrees in the center.

    // First, let's define the pattern by creating a new stream builder.
    let mut stream_builder = surface.stream_builder();
    let mut pattern_surface = stream_builder.surface();

    // Draw the top-left rectangle.
    pattern_surface.fill_path(
        &rect_to_path(0.0, 0.0, 10.0, 10.0),
        Fill {
            paint: Paint::<Rgb>::Color(rgb::Color::new(255, 0, 0)),
            ..Default::default()
        },
    );

    // Draw the bottom-right rectangle.
    pattern_surface.fill_path(
        &rect_to_path(10.0, 10.0, 20.0, 20.0),
        Fill {
            paint: Paint::<Rgb>::Color(rgb::Color::new(0, 0, 255)),
            ..Default::default()
        },
    );
    pattern_surface.finish();

    // Get the pattern stream
    let pattern_stream = stream_builder.finish();

    // Define the actual pattern
    let pattern = Pattern {
        stream: pattern_stream,
        transform: Default::default(),
        width: 20.0,
        height: 20.0,
    };

    // Now we draw the actual transformed rectangle.
    // First, push a transform so that the rectangle will be rotated.
    surface.push_transform(&Transform::from_rotate_at(45.0, 100.0, 100.0));

    let rect_path = rect_to_path(30.0, 30.0, 170.0, 170.0);

    // Draw the rectangle.
    surface.fill_path(
        &rect_path,
        Fill {
            // Note: The actual color space you use here doesn't matter, it has no
            // effect on the output. It's just a current limitation in the API of
            // krilla that you need to define something.
            paint: Paint::<Rgb>::Pattern(pattern),
            ..Default::default()
        },
    );

    // Let's also add a stroke, makes it look a bit nicer.
    surface.stroke_path(
        &rect_path,
        Stroke {
            // Note: The actual color space you use here doesn't matter, it has no
            // effect on the output. It's just a current limitation in the API of
            // krilla that you need to define something.
            paint: Paint::<Rgb>::Color(rgb::Color::black()),
            ..Default::default()
        },
    );

    // Don't forget to pop! Each `push_` method must have a corresponding pop.
    surface.pop();

    surface.finish();
    page.finish();

    let pdf = document.finish().unwrap();
    std::fs::write("target/stream_builder.pdf", &pdf).unwrap();
}

// A simple convenience function that allow us to generate rectangle paths.
pub fn rect_to_path(x1: f32, y1: f32, x2: f32, y2: f32) -> Path {
    let mut builder = PathBuilder::new();
    builder.push_rect(Rect::from_ltrb(x1, y1, x2, y2).unwrap());
    builder.finish().unwrap()
}
