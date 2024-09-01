//! This example contains the most basic document you can create: A document
//! with a single empty page.

use krilla::{Document, PageSettings};
use pdf_writer::Finish;

fn main() {
    // First, we create a new document. This represents a single PDF document.
    let mut document = Document::new();
    // We can now successively add new pages by calling `start_page`, or `start_page_with`
    // if we want to pass custom page settings.
    let mut page = document.start_page_with(PageSettings::with_size(300.0, 600.0));
    page.finish();

    // Create the PDF
    let pdf = document.finish().unwrap();

    // Write the PDF to a file.
    std::fs::write("target/empty_document.pdf", &pdf).unwrap();
}
