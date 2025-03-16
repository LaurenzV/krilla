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
# use krilla::graphics::color::rgb;
# use krilla::text::Font;
# use krilla::geom::Point;
# use krilla::graphics::paint::Paint;
# use krilla::text::TextDirection;
# use krilla::graphics::paint::Fill;
# use krilla::{Document, PageSettings};
# // use krilla::SvgSettings;
# use std::path::PathBuf;
# use std::sync::Arc;
# use krilla::NormalizedF32;
# // TODO: Fix example
# fn main() {
// Create a new document.
let mut document = Document::new();
// Load a font.
let mut font = {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/NotoSans-Regular.ttf");
    let data = std::fs::read(&path).unwrap();
    Font::new(data.into(), 0, true).unwrap()
};

// Add a new page with dimensions 200x200.
let mut page = document.start_page_with(PageSettings::new(200.0, 200.0));
// Get the surface of the page.
let mut surface = page.surface();
// Draw some text.
surface.fill_text(
    Point::from_xy(0.0, 25.0),
    font.clone(),
    14.0,
    "This text has font size 14!",
    false,
    TextDirection::Auto
);

surface.set_fill(Fill {
    paint: rgb::Color::new(255, 0, 0).into(),
    opacity: NormalizedF32::new(0.5).unwrap(),
    rule: Default::default(),
});
// Draw some more text, in a different color with an opacity and bigger font size.
surface.fill_text(
    Point::from_xy(0.0, 50.0),
    font.clone(),
    16.0,
    "This text has font size 16!",
    false,
    TextDirection::Auto
);

// Finish the page.
surface.finish();
page.finish();

// // Load an SVG.
// let svg_tree = {
//     let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
//         .join("../../assets/svgs/custom_integration_wikimedia_coat_of_the_arms_of_edinburgh_city_council.svg");
//     let data = std::fs::read(&path).unwrap();
//     // usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap()
// };

// Start a new page, with the same dimensions as the SVG.
// let svg_size = svg_tree.size();
// let mut page = document.start_page_with(PageSettings::new(svg_size.width(), svg_size.height()));
// let mut surface = page.surface();
// Draw the SVG.
//
// surface.draw_svg(&svg_tree, svg_size, SvgSettings::default());

// Finish up and write the resulting PDF.
// surface.finish();
// page.finish();
let pdf = document.finish().unwrap();
std::fs::write("../../target/example.pdf", &pdf).unwrap();
# }
```
[krilla]: https://github.com/LaurenzV/krilla
[pdf-writer]: https://github.com/typst/pdf-writer
[examples]: https://github.com/LaurenzV/krilla/tree/main/examples
*/

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod chunk_container;
mod prelude;
mod resource;
mod serialize;
mod util;

pub(crate) mod content;

pub mod configure;
pub mod document;
pub mod error;
pub mod geom;
pub mod graphics;
pub mod interactive;
pub mod interchange;
pub mod page;
pub mod path;
pub mod stream;
pub mod surface;
pub mod text;

use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use chunk_container::ChunkContainerFn;
use pdf_writer::{Chunk, Ref};
pub use prelude::*;

use crate::resource::Resource;
use crate::serialize::SerializeContext;
use crate::util::SipHashable;

/// A type that holds some bytes.
#[derive(Clone)]
pub struct Data(Arc<dyn AsRef<[u8]> + Send + Sync>);

impl AsRef<[u8]> for Data {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref().as_ref()
    }
}

impl Hash for Data {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl From<Arc<dyn AsRef<[u8]> + Send + Sync>> for Data {
    fn from(value: Arc<dyn AsRef<[u8]> + Send + Sync>) -> Self {
        Self(value)
    }
}

impl From<Vec<u8>> for Data {
    fn from(value: Vec<u8>) -> Self {
        Self(Arc::new(value))
    }
}

impl From<Arc<Vec<u8>>> for Data {
    fn from(value: Arc<Vec<u8>>) -> Self {
        Self(value)
    }
}

impl Debug for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Data {{..}}")
    }
}

pub(crate) trait Cacheable: SipHashable {
    fn chunk_container(&self) -> ChunkContainerFn;
    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk;
}

