//! This example contains the most basic document you can create: A document
//! with a single empty page.

use std::path;

use krilla::page::PageSettings;
use krilla::Document;

fn main() {
    // First, we create a new document. This represents a single PDF document.
    let mut document = Document::new();
    // We can now successively add new pages by calling `start_page`, or `start_page_with`
    // if we want to pass custom page settings.
    let page = document.start_page_with(PageSettings::from_wh(300.0, 600.0).unwrap());
    page.finish();

    // Create the PDF
    let pdf = document.finish().unwrap();

    let path = path::absolute("empty_document.pdf").unwrap();
    eprintln!("Saved PDF to '{}'", path.display());

    // Write the PDF to a file.
    std::fs::write(path, &pdf).unwrap();
}
