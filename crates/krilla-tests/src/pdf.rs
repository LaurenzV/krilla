use krilla::error::KrillaError;
use krilla::geom::Size;
use krilla::page::Page;
use krilla::Document;
use krilla_macros::snapshot;

use crate::metadata::metadata_impl;
use crate::text::simple_text_impl;
use crate::{load_png_image, rect_to_path, red_fill, settings_16, NOTO_SANS};

#[snapshot(document)]
fn pdf_empty(_: &mut Document) {}

#[snapshot(document, settings_16)]
fn pdf_14(document: &mut Document) {
    metadata_impl(document);
}

#[snapshot(document, settings_25)]
fn pdf_20(document: &mut Document) {
    metadata_impl(document);
}

#[snapshot(settings_25)]
fn pdf_20_simple_text(page: &mut Page) {
    // The main purpose of this test is to ensure that the fonts without CIDSet are
    // still written properly for PDF 2.0.
    simple_text_impl(page, NOTO_SANS.clone());
}

#[snapshot(settings_18)]
fn pdf_14_icc_srgb(page: &mut Page) {
    let mut surface = page.surface();
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(50.0, 50.0, 100.0, 100.0));
}

#[snapshot(settings_18)]
fn pdf_14_icc_sgray(page: &mut Page) {
    let mut surface = page.surface();
    surface.draw_path(&rect_to_path(50.0, 50.0, 100.0, 100.0));
}

#[test]
fn pdf_14_no_sixteen_bit_images() {
    let mut document = Document::new_with(settings_16());
    let mut page = document.start_page();
    let mut surface = page.surface();
    let image = load_png_image("luma16.png");
    let size = image.size();
    surface.draw_image(
        image.clone(),
        Size::from_wh(size.0 as f32, size.1 as f32).unwrap(),
    );

    surface.finish();
    page.finish();

    assert_eq!(
        document.finish(),
        Err(KrillaError::SixteenBitImage(image.clone(), None))
    );
}
