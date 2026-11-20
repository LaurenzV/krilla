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
    let (svg_tree, svg_dpi) = {
        let mut fontdb = Database::new();
        fontdb.load_system_fonts();
        let opts = usvg::Options {
            fontdb: Arc::new(fontdb),
            ..Default::default()
        };

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/svgs/custom_integration_wikimedia_coat_of_the_arms_of_edinburgh_city_council.svg");
        let data = std::fs::read(&path).unwrap();
        (usvg::Tree::from_data(&data, &opts).unwrap(), opts.dpi)
    };

    // Create a new document.
    let mut document = Document::new();

    // Start a new page, with the same dimensions as the SVG, taking
    // care of the different units of measurement: svg_size is in
    // CSS pixels, while pdf_size is in PDF points (1 pt = 1/72 in).
    let svg_size = svg_tree.size();
    let pt_per_px = 72. / svg_dpi;
    let pdf_size =
        Size::from_wh(svg_size.width() * pt_per_px, svg_size.height() * pt_per_px).unwrap();
    let page_settings = PageSettings::from_wh(pdf_size.width(), pdf_size.height()).unwrap();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    // Draw the SVG.
    surface.draw_svg(&svg_tree, pdf_size, SvgSettings::default());
    surface.finish();
    page.finish();

    let pdf = document.finish().unwrap();

    let path = path::absolute("svg.pdf").unwrap();
    eprintln!("Saved PDF to '{}'", path.display());

    // Write the PDF to a file.
    std::fs::write(path, &pdf).unwrap();
}
