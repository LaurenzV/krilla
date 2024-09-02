A high-level, ergonomic Rust library creating beautiful PDF files.

`krilla` is a high-level Rust crate that allows for the creation of PDF files. It builds
on top of the excellent [pdf-writer](https://github.com/typst/pdf-writer) crate, 
but abstracts away all complexities that are involved in creating a PDF file, 
instead providing an interface with high-level primitives, such
as fills, strokes, gradient, glyphs and images which can be used and combined easily
without having to worry about low-level details.

# Features
krilla supports a good amount of features you would expect from a graphics library, including:

- Support for filling and stroking arbitrary paths.
- Affine transformations.
- Alpha and luminosity masks.
- Clip paths.
- A high-level text API for drawing simple strings.
- A low-level API text API for drawing sequences of glyphs.
- Excellent OpenType font support, supporting all major font types, including color fonts. krilla also has
  great subsetting support for both, CFF-flavored and TTF-flavored fonts.
- Linear, radial and sweep gradients as well as patterns.
- Embedding bitmap and SVG images.
- PDF features like outlines, page labels and links.

# Non-goals
The PDF specification is huge and supports tons of features with a lot of customization, including
complex color spaces and functions. The goal of krilla is not to expose high-level bindings 
for all functionality, but just for a specific subset that is commonly used.

# Testing
Testing is a major pain point for most PDF-creation libraries. The reason is that it is very hard to do:
It is very easy to accidentally invalid PDF files, and just testing PDF files in one 
PDF viewer is not enough to be confident about the correctness. The reason for this 
is that PDF viewers are often tolerant in what they accept, meaning that it is possible 
that a PDF just happens to show up fine in one viewer you tested, but fails in all other ones.

Because of this, ensuring proper testing has been **one of my main priorities** when building this crate,
and is probably one of the main distinguishing features from other crates. krilla has two approaches for testing:

## Snapshot-based tests
*We currently have 50+ snapshot tests*, which basically contain an ASCII representation of various
"PDF snippets" and have been manually checked to ensure they look as expected. This allows us to detect
regressions in the actual output of our PDFs.

## Visual-based tests

### Unit tests
As mentioned above, checking one PDF viewer for correct output is not enough. Because of this, our visual
regression tests are run against **7 distinct PDF viewers** (although only 6 are run in CI) to ensure that basic 
krilla features are displayed correctly in all major viewers. The current selection of viewers includes:
- ghostscript
- mupdf
- poppler
- pdfbox
- pdfium (used in Google Chrome)
- pdf.js (used in Firefox)
- Quartz (used in Safari, this one is not checked in CI, but will run locally on any Mac machine)

*We currently have a combined 210+ tests* that run against those viewers, which check that basic
aspects such as text and gradients are encoded correctly and supported on all (or most) viewers.

This selection unfortunately does not include Adobe Acrobat, which is arguably the most important viewer.
The reason for this is simply that it is pretty much impossible to conveniently render PDFs with it. However,
all tests have been opened at least once with Acrobat to ensure that no errors appear when opening it.

### Integration tests
Finally, we also have visual integration tests to test distinct features as well as combinations of them
(like for example gradients with transforms). We use the `resvg` test suite for that, which conveniently
also allows us to automatically test the accuracy of the SVG conversion of `krilla`. Those tests are
only run against one viewer (in most cases `pdfium`), as it would be pretty wasteful to save reference
images for all of them. 

*Currently, we have over 1500 such tests*, and although they mostly focus on
testing adherence to the SVG specification, they indirectly also test various interactions of `krilla`-specific
features.

## Summary
While `krilla` does have a very extensive test suite, there is still a lot that is untested, and `krilla` also
hasn't been used on a wide scale, so there are bound to be bugs. However, I think the current test setup makes
it very easy to track future bugs and puts `krilla` in a very good position to ensure that no 
regressions occur in the future.

# Future work
For the future, I plan to at least add support for:
- Adding document metadata.
- Support for tagged PDFs for accessibility.
- Support for validated PDF export, like for example PDF/UA

# Example

The following example shows some of the features of krilla in full action.

The example creates a PDF file with two pages. On the first page,
we add two small pieces of text, and on the second page we embed a full-page SVG.
Consult the documentation to see all features that are available in krilla.

For more example, feel free to consult the [examples] directory of the GitHub repository.

```
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
```

# Safety
This crate has one direct usage of `unsafe`, which is needed when loading a font
via `fontdb`. The only reason this is unsafe is that fonts can be memory-mapped,
in which case other programs could tamper with that memory. However, this part of the
code will only be invoked if you use the `svg` or `fontdb` feature.

Other than that, this crate has no unsafe code, although it relies on crates such as
`bytemuck` and `yoke` that do.

# License
This crate is dual-licensed under the MIT and Apache 2.0 licenses.