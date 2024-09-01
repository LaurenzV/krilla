use std::path::PathBuf;
use pdf_writer::Finish;
use sitro::Renderer;
use krilla_macros::visreg;
use crate::{Document, PageSettings, SerializeSettings};
use crate::tests::{ASSETS_PATH, check_render, render_document};


#[test]
fn custom_integration_drawio_diagram1() {
    let settings = SerializeSettings::default();
    let mut d = Document::new_with(settings);

    let svg_path = ASSETS_PATH.join(format!("svgs/{}.svg", "custom_integration_drawio_diagram1"));
    let data = std::fs::read(&svg_path).unwrap();
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();

    let mut page = d.start_page_with(PageSettings::with_size(tree.size().width(), tree.size().height()));
    let mut surface = page.surface();
    surface.draw_svg(&tree, tree.size());
    surface.finish();
    page.finish();

    let pdf = d.finish().unwrap();
    let rendered = render_document(&pdf, &Renderer::Pdfium);
    check_render("svg_custom_integration_drawio_diagram1", &Renderer::Pdfium, rendered, &pdf, true);
}