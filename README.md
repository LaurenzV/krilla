# krilla

[![Crates.io](https://img.shields.io/crates/v/krilla.svg)](https://crates.io/crates/krilla)
[![Documentation](https://docs.rs/krilla/badge.svg)](https://docs.rs/krilla)

`krilla` is a high-level Rust crate that allows for the creation of PDF files. It builds
on top of the [pdf-writer](https://github.com/typst/pdf-writer) crate,
but abstracts away all complexities that are involved in creating a PDF file,
instead providing an interface with high-level primitives, such
as fills, strokes, gradient, glyphs and images which can be used and combined easily
without having to worry about low-level details.

## Features
`krilla` supports most features you would expect from a 2D graphics library, including:

- Filling and stroking arbitrary paths.
- Affine transformations.
- Alpha and luminosity masks.
- Clip paths.
- Blend modes and layer isolation.
- A high-level text API for rendering sequences of characters.
- A low-level text API for drawing sequences of positioned glyphs.
- Excellent OpenType font support, supporting all major font types, including color fonts.
- Linear, radial and sweep gradients, as well as patterns.
- Embedding bitmap images, and SVG images via `krilla-svg`.
- Optional support for multi-threading via `rayon`, allowing for great speedups when creating
compressed PDFs or PDF with lots of images.

In addition to that, the library also supports the following PDF features:
- Great subsetting for both, CFF-flavored and TTF-flavored fonts, ensuring small file sizes.
- Creating document outlines.
- Setting page labels.
- Annotations, links, (named) destinations.
- Adding document metadata.
- Creating accessible PDFs via tagged PDF.
- Support for different PDF versions (1.4, 1.5, 1.6, 1.7, 2.0).
- Support for validated some validated export modes (PDF/A-1, PDF/A-2, PDF/A-3, PDF/A-4, PDF/UA-1).

## Scope
This crate labels itself as a high-level crate, and this is what it is: It abstracts away most
of the complexity of the PDF format and instead provides high-level primitives for
creating PDF files. However from a document-creation perspective, this crate is still
very low-level: It does not provide functionality like text layouting, creation of tables,
page breaking, inserting headers/footers, etc. This kind of functionality is strictly out of scope for
`krilla`.

`krilla`'s main "target group" is libraries that have some kind of intermediate representation
of layouted content (whether it be from HTML or other input sources), and want to easily
translate this representation into a PDF file. If this is your use case, then `krilla` is probably
a very suitable, if not the most suitable choice for you.

If not, depending on what exactly you want to do, there are other Rust crates you can use:

- Creating PDF files with very low-level access to the resulting file: [pdf-writer](https://github.com/typst/pdf-writer).
- Creating documents requiring high-level functionality like automatic text layouting,
page breaking, inserting headers and footers: [typst](https://github.com/typst/typst/).
- Reading existing PDF documents and manipulating them in a certain way: [pdf-rs](https://github.com/pdf-rs/pdf).

Also worth mentioning is [printpdf](https://github.com/fschutt/printpdf) which operates at a similar level of abstraction as `krilla`, but is based on `lopdf` has a greater focus on creating print-friendly PDFs.

The PDF specification is *huge* and supports tons of features with a lot of customization, including
complex color spaces and shadings. The goal of `krilla` is not to expose high-level bindings
for all functionality, but instead expose only a relevant subset of it. Implementing features like encryption or digital signatures, as well as many other PDF features, are (for now) out-of-scope for this crate.

## Testing
Testing is a major pain point for most PDF-creation libraries. The reason is that it is very hard to do:
It is very easy to accidentally create invalid PDF files, and just testing PDF files in one
PDF viewer is not enough to be confident about its correctness. The reason for this
is that PDF viewers are often tolerant in what they accept, meaning that it is possible
that a PDF just happens to show up fine in one viewer you tested, but fails in all other ones.

**Because of this, ensuring proper testing has been one of my main priorities when building this crate,
and `krilla` has by far the most comprehensive testing infrastructure, compared to other PDF creation crates.** `krilla` has two approaches for testing:

### Snapshot-based tests
*We currently have 90+ snapshot tests*, which basically contain an ASCII representation of various
"PDF snippets" and have been manually checked to ensure they look as expected. This allows us to detect
regressions in the actual output of our PDFs. These snippets are also tested against the Arlington PDF model and veraPDF in CI.

### Visual-based tests

#### Unit tests
As mentioned above, checking one PDF viewer for correct output is not enough. Because of this, our visual
regression tests are run against **6 distinct PDF viewers** (although only 5 are run in CI) to ensure that basic
`krilla` features are displayed correctly in all major viewers. The current selection of viewers includes:
- ghostscript
- mupdf
- poppler
- pdfbox
- pdfium (used in Google Chrome)
- Quartz (used in Safari, this one is not checked in CI, but will run locally on any Mac machine)

*We currently have a combined 210+ tests* that run against those viewers, which check that basic
aspects such as text and gradients are encoded correctly and supported on all (or most) viewers.

This selection unfortunately does not include Adobe Acrobat, which is arguably the most important viewer.
The reason for this is simply that it is pretty much impossible to conveniently render PDFs with it. However,
all tests have been opened at least once with Acrobat to ensure that no errors appear when opening it.

#### Integration tests
Finally, we also have visual integration tests to test distinct features as well as combinations of them
(like for example gradients with transforms). We use the `resvg` test suite for that, which conveniently
also allows us to automatically test the accuracy of the SVG conversion of `krilla-svg`. Those tests are
only run against one viewer (in most cases `pdfium`), as it would be pretty wasteful to save reference images for all of them.

*Currently, we have over 1500 such tests*, and although they mostly focus on
testing adherence to the SVG specification, they indirectly also test various interactions of `krilla`-specific
features.

Note: If you want to run the tests locally, make sure to read `CONTRIBUTING.md` for instructions on how
to set up everything, as running the visual-based tests unfortunately requires a very specific setup.

### Summary
I think the current test setup makes it very easy to track future bugs and puts `krilla` in a very good spot to ensure that no regressions occur in the future, hopefully convincing you that it is a solid choice for production use cases.

## Example

The following example shows some of the features of `krilla` in action.

The example creates a PDF file with two pages. On the first page,
we add two small pieces of text, and on the second page we draw a triangle with a gradient fill.

For more examples, feel free to take a look at the [examples](https://github.com/LaurenzV/krilla/tree/main/crates/krilla/examples) directory of the GitHub repository.

```rs
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
```

# Safety
krilla has zero usages of `unsafe` and forbids unsafe code via a crate-level attribute. krilla-svg has one direct usage of `unsafe`, which is needed when loading a font
via `fontdb`. The only reason this is unsafe is that fonts can be memory-mapped,
in which case other programs could tamper with that memory.

With that said, we do rely on crates such as `bytemuck` and `yoke` that do use unsafe code.

# License
This crate is dual-licensed under the MIT and Apache 2.0 licenses.
