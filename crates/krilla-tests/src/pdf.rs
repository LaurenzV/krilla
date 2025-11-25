use std::sync::Arc;

use image::load_from_memory;
use krilla::configure::{PdfVersion, ValidationError};
use krilla::error::KrillaError;
use krilla::geom::{Size, Transform};
use krilla::page::{Page, PageSettings};
use krilla::pdf::{PdfDocument, PdfError};
use krilla::surface::Surface;
use krilla::{Document, SerializeSettings};
use krilla_macros::{snapshot, visreg};
use krilla_svg::{SurfaceExt, SvgSettings};

use crate::metadata::metadata_impl;
use crate::svg::sample_svg;
use crate::text::simple_text_impl;
use crate::{load_pdf, load_png_image, loc, rect_to_path, red_fill, settings_16, NOTO_SANS};

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

#[snapshot(document)]
fn pdf_embedded_simple(document: &mut Document) {
    let pdf = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");
    document.embed_pdf_pages(&pdf, &[0]);
}

#[snapshot(document)]
fn pdf_embedded_repeated_page(document: &mut Document) {
    let pdf = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");
    document.embed_pdf_pages(&pdf, &[0, 0, 0]);
}

#[snapshot(document)]
fn pdf_embedded_multiple(document: &mut Document) {
    let pdf1 = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");
    let pdf2 = load_pdf("page_media_box_bottom_right.pdf");
    document.embed_pdf_pages(&pdf1, &[0]);
    document.embed_pdf_pages(&pdf2, &[0]);
}

#[test]
fn pdf_embedded_out_of_bounds() {
    let mut document = Document::new();

    let pdf = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");
    document.set_location(loc(1));
    document.embed_pdf_pages(&pdf, &[1]);

    assert_eq!(
        document.finish(),
        Err(KrillaError::Pdf(
            pdf.clone(),
            PdfError::InvalidPage(1),
            Some(loc(1))
        ))
    )
}

#[test]
fn pdf_embedded_version_mismatch() {
    let mut document = Document::new_with(crate::settings_17());

    let pdf = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");
    document.embed_pdf_pages(&pdf, &[0]);

    assert_eq!(
        document.finish(),
        Err(KrillaError::Pdf(
            pdf.clone(),
            PdfError::VersionMismatch(PdfVersion::Pdf17),
            None
        ))
    )
}

#[test]
fn pdf_embedded_validated_export() {
    // While it is in principle possible to support embedded PDFs in validated export if the
    // embedded PDF also conforms, for now we outright reject it.
    let mut document = Document::new_with(crate::settings_23());

    let pdf = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");
    document.embed_pdf_pages(&pdf, &[0]);

    assert_eq!(
        document.finish(),
        Err(KrillaError::Validation(vec![ValidationError::EmbeddedPDF(
            None
        )]))
    )
}

#[test]
fn pdf_embedded_consistency() {
    let mut last = None;
    for _ in 0..30 {
        let mut document = Document::new();
        let pdf1 = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");
        let pdf2 = load_pdf("page_media_box_bottom_right.pdf");
        document.embed_pdf_pages(&pdf1, &[0]);
        document.embed_pdf_pages(&pdf2, &[0]);

        let finished = document.finish().unwrap();

        if let Some(last) = &last {
            assert_eq!(&finished, last);
        } else {
            last = Some(finished);
        }
    }
}

#[visreg]
fn pdf_embedded_as_xobject_basic(surface: &mut Surface) {
    let pdf = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");
    surface.draw_pdf_page(&pdf, Size::from_wh(200.0, 200.0).unwrap(), 0);
}

#[snapshot]
fn pdf_embedded_as_xobject_basic(page: &mut Page) {
    let mut surface = page.surface();
    let pdf = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");
    surface.draw_pdf_page(&pdf, Size::from_wh(200.0, 200.0).unwrap(), 0);
}

#[visreg(document)]
fn pdf_embedded_as_xobject_different_sizes(document: &mut Document) {
    let mut page = document.start_page_with(PageSettings::from_wh(600.0, 600.0).unwrap());
    let mut surface = page.surface();

    let sizes = [(50.0, 50.0), (150.0, 150.0), (300.0, 150.0), (200.0, 400.0)];
    let positions = [(10.0, 10.0), (100.0, 10.0), (30.0, 200.0), (350.0, 200.0)];

    let pdf = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");

    for (size, position) in sizes.iter().zip(positions) {
        surface.push_transform(&Transform::from_translate(
            position.0 as f32,
            position.1 as f32,
        ));
        surface.draw_pdf_page(&pdf, Size::from_wh(size.0, size.1).unwrap(), 0);
        surface.pop();
    }
}

#[visreg(document)]
fn pdf_embedded_as_xobject_multiple(document: &mut Document) {
    let pdf1 = load_pdf("typst_doc.pdf");
    let pdf2 = load_pdf("pdftc_100k_1894.pdf");
    let pdf3 = load_pdf("page_media_box_bottom_right.pdf");

    let mut page1 = document.start_page_with(PageSettings::from_wh(600.0, 800.0).unwrap());
    let mut surface = page1.surface();

    surface.push_transform(&Transform::from_translate(10.0, 15.0));
    surface.draw_pdf_page(&pdf1, Size::from_wh(200.0, 282.8).unwrap(), 0);
    surface.pop();

    surface.push_transform(&Transform::from_translate(350.0, 15.0));
    surface.draw_pdf_page(&pdf1, Size::from_wh(250.0, 353.0).unwrap(), 3);
    surface.pop();

    surface.push_transform(&Transform::from_translate(200.0, 400.0));
    surface.draw_pdf_page(&pdf3, Size::from_wh(250.0, 300.0).unwrap(), 0);
    surface.pop();

    surface.finish();
    page1.finish();

    let mut page2 = document.start_page_with(PageSettings::from_wh(500.0, 500.0).unwrap());
    let mut surface = page2.surface();

    surface.draw_pdf_page(&pdf2, Size::from_wh(250.0, 250.8).unwrap(), 3);

    surface.push_transform(&Transform::from_translate(250.0, 0.0));
    surface.draw_pdf_page(&pdf2, Size::from_wh(250.0, 250.8).unwrap(), 8);
    surface.pop();

    surface.push_transform(&Transform::from_translate(0.0, 250.0));
    surface.draw_pdf_page(&pdf2, Size::from_wh(250.0, 250.8).unwrap(), 9);
    surface.pop();

    surface.push_transform(&Transform::from_translate(250.0, 250.0));
    surface.draw_pdf_page(&pdf2, Size::from_wh(250.0, 250.8).unwrap(), 10);
    surface.pop();
}

#[visreg(document)]
fn pdf_embedded_simple(document: &mut Document) {
    let pdf = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");
    document.embed_pdf_pages(&pdf, &[0]);
}

#[visreg(document)]
fn pdf_embedded_multiple(document: &mut Document) {
    let pdf1 = load_pdf("resvg_masking_clipPath_mixed_clip_rule.pdf");
    let pdf2 = load_pdf("page_media_box_bottom_right.pdf");
    document.embed_pdf_pages(&pdf1, &[0]);
    document.embed_pdf_pages(&pdf2, &[0]);
}

#[visreg(document)]
fn pdf_embedded_multi_page_document(document: &mut Document) {
    let pdf = load_pdf("typst_doc.pdf");
    document.embed_pdf_pages(&pdf, &[0, 2, 3, 5, 7]);
}

#[snapshot]
fn pdf_embedded_as_xobject_caching(page: &mut Page) {
    let pdf = load_pdf("xobject_1.pdf");
    let mut surface = page.surface();
    surface.draw_pdf_page(&pdf, Size::from_wh(100.0, 100.0).unwrap(), 0);
    surface.push_transform(&Transform::from_translate(100.0, 0.0));
    surface.draw_pdf_page(&pdf, Size::from_wh(100.0, 100.0).unwrap(), 0);
    surface.pop();
    surface.push_transform(&Transform::from_translate(0.0, 100.0));
    surface.draw_pdf_page(&pdf, Size::from_wh(100.0, 100.0).unwrap(), 0);
    surface.pop();
    surface.push_transform(&Transform::from_translate(100.0, 100.0));
    surface.draw_pdf_page(&pdf, Size::from_wh(100.0, 100.0).unwrap(), 0);
    surface.pop();
}
