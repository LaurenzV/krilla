//! An example demonstrating how you can create a PDF file with an SVG.

use std::path;
use std::path::PathBuf;
use std::sync::Arc;

use fontdb::Database;
use krilla::geom::Size;
use krilla::page::PageSettings;
use krilla::Document;
use krilla_svg::{SurfaceExt, SvgSettings};

fn main() {
    // Load an SVG.
    let svg_tree = {
        let mut fontdb = Database::new();
        fontdb.load_system_fonts();
        let opts = usvg::Options {
            fontdb: Arc::new(fontdb),
            ..Default::default()
        };

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/svgs/custom_integration_wikimedia_coat_of_the_arms_of_edinburgh_city_council.svg");
        let data = std::fs::read(&path).unwrap();
        usvg::Tree::from_data(&data, &opts).unwrap()
    };

    // Create a new document.
    let mut document = Document::new();

    let svg_size = Size::from_wh(svg_tree.size().width(), svg_tree.size().height()).unwrap();
    // Start a new page, with the same dimensions as the SVG.
    let mut page = document.start_page_with(PageSettings::new(svg_size.width(), svg_size.height()));
    let mut surface = page.surface();
    // Draw the SVG.
    surface.draw_svg(&svg_tree, svg_size, SvgSettings::default());
    surface.finish();
    page.finish();

    let pdf = document.finish().unwrap();

    let path = path::absolute("svg.pdf").unwrap();
    eprintln!("Saved PDF to '{}'", path.display());

    // Write the PDF to a file.
    std::fs::write(path, &pdf).unwrap();
}
