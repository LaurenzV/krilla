/*!
A high-level, ergonomic Rust library creating PDF documents.

[krilla] is a high-level Rust crate that allows for the creation of PDF files. It builds
on top of the [pdf-writer] crate, but abstracts away all complexities that are
involved in creating a PDF file, instead providing an interface with high-level primitives, such
as fills, strokes, gradient, glyphs and images which can be used and combined easily
without having to worry about low-level details.

To get started, take a look at the [`document`] module that explains how you can create
a document using krilla.

# Example

The following example shows some of the features of krilla in action.

The example creates a PDF file with two pages. On the first page,
we add two small pieces of text, and on the second page we embed a full-page SVG.
Consult the documentation to see all features that are available in krilla.

For more examples, feel free to take a look at the [examples] directory of the GitHub repository.

```
# use krilla::color::rgb;
# use krilla::color::rgb::Rgb;
# use krilla::font::Font;
# use krilla::geom::Point;
# use krilla::paint::Paint;
# use krilla::path::Fill;
# use krilla::{Document, PageSettings};
# use std::path::PathBuf;
# use std::sync::Arc;
# use usvg::NormalizedF32;
# fn main() {
// Create a new document.
let mut document = Document::new();
// Load a font.
let mut font = {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts/NotoSans-Regular.ttf");
    let data = std::fs::read(&path).unwrap();
    Font::new(Arc::new(data), 0, vec![]).unwrap()
};

// Add a new page with dimensions 200x200.
let mut page = document.start_page_with(PageSettings::new(200.0, 200.0));
// Get the surface of the page.
let mut surface = page.surface();
// Draw some text.
surface.fill_text(
    Point::from_xy(0.0, 25.0),
    Fill::<Rgb>::default(),
    font.clone(),
    14.0,
    &[],
    "This text has font size 14!",
);
// Draw some more text, in a different color with an opacity and bigger font size.
surface.fill_text(
    Point::from_xy(0.0, 50.0),
    Fill {
        paint: Paint::<Rgb>::Color(rgb::Color::new(255, 0, 0)),
        opacity: NormalizedF32::new(0.5).unwrap(),
        rule: Default::default(),
    },
    font.clone(),
    16.0,
    &[],
    "This text has font size 16!",
);

// Finish the page.
surface.finish();
page.finish();

// Load an SVG.
let svg_tree = {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/svgs/custom_integration_wikimedia_coat_of_the_arms_of_edinburgh_city_council.svg");
    let data = std::fs::read(&path).unwrap();
    usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap()
};

// Start a new page, with the same dimensions as the SVG.
let svg_size = svg_tree.size();
let mut page = document.start_page_with(PageSettings::new(svg_size.width(), svg_size.height()));
let mut surface = page.surface();
// Draw the SVG.
surface.draw_svg(&svg_tree, svg_size);

// Finish up and write the resulting PDF.
surface.finish();
page.finish();
let pdf = document.finish().unwrap();
std::fs::write("target/example.pdf", &pdf).unwrap();
# }
```
[krilla]: https://github.com/LaurenzV/krilla
[pdf-writer]: https://github.com/typst/pdf-writer
[examples]: https://github.com/LaurenzV/krilla/tree/main/examples
*/

#![deny(missing_docs)]

mod chunk_container;
mod graphics_state;
mod object;
mod resource;
mod serialize;
#[cfg(feature = "svg")]
mod svg;
mod util;

pub mod document;
pub mod error;
pub mod font;
pub mod geom;
pub use object::*;
pub mod paint;
pub mod path;
pub mod stream;
pub mod surface;

pub mod content;
#[cfg(test)]
pub mod tests;

pub use document::*;
pub use serialize::{SerializeSettings, SvgSettings};
