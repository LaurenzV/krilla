use krilla::{Document, Page};
use krilla_macros::snapshot;

use crate::metadata::metadata_impl;
use crate::text::simple_text_impl;
use crate::NOTO_SANS;

#[snapshot(document)]
fn pdf_empty(_: &mut Document) {}

#[snapshot(document, settings_16)]
fn pdf_version_14(document: &mut Document) {
    metadata_impl(document);
}

#[snapshot(document, settings_25)]
fn pdf_version_20(document: &mut Document) {
    metadata_impl(document);
}

#[snapshot(single_page, settings_25)]
fn pdf_version_20_simple_text(page: &mut Page) {
    // The main purpose of this test is to ensure that the fonts without CIDSet are
    // still written properly for PDF 2.0.
    simple_text_impl(page, NOTO_SANS.clone());
}
