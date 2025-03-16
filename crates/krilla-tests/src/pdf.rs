use krilla::page::Page;
use krilla::Document;
use krilla_macros::snapshot;

use crate::metadata::metadata_impl;
use crate::text::simple_text_impl;
use crate::{rect_to_path, red_fill, NOTO_SANS};

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
    surface.set_fill(red_fill(1.0));
    surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0));
}

#[snapshot(settings_18)]
fn pdf_14_icc_sgray(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0));
}
