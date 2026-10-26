#![allow(non_snake_case)]

use krilla::geom::{Size, Transform};
use krilla::graphic::Graphic;
use krilla::page::PageSettings;
use krilla::surface::Surface;
use krilla::Document;
use krilla_macros::{snapshot, visreg};
use krilla_svg::{SurfaceExt, SvgSettings};

use crate::{FONTDB, SVGS_PATH};

pub(crate) fn sample_svg() -> usvg::Tree {
    let data = std::fs::read(SVGS_PATH.join("resvg_masking_mask_with_opacity_1.svg")).unwrap();
    usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap()
}

#[visreg]
fn svg_simple(surface: &mut Surface) {
    let tree = sample_svg();
    surface.draw_svg(
        &tree,
        Size::from_wh(tree.size().width(), tree.size().height()).unwrap(),
        SvgSettings::default(),
    );
}

#[visreg]
fn svg_outlined_text(surface: &mut Surface) {
    let data = std::fs::read(SVGS_PATH.join("resvg_text_text_simple_case.svg")).unwrap();
    let tree = usvg::Tree::from_data(
        &data,
        &usvg::Options {
            fontdb: FONTDB.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let settings = SvgSettings {
        embed_text: false,
        ..Default::default()
    };
    surface.draw_svg(
        &tree,
        Size::from_wh(tree.size().width(), tree.size().height()).unwrap(),
        settings,
    );
}

#[visreg(width = 120, height = 80)]
fn svg_resized(surface: &mut Surface) {
    surface.draw_svg(
        &sample_svg(),
        Size::from_wh(120.0, 80.0).unwrap(),
        SvgSettings::default(),
    );
}

#[visreg(width = 343, height = 285)]
fn svg_should_be_clipped(surface: &mut Surface) {
    let data =
        std::fs::read(SVGS_PATH.join("custom_paint_servers_pattern_patterns_2.svg")).unwrap();
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();

    surface.push_transform(&Transform::from_translate(143.0, 1.0));
    surface.draw_svg(
        &tree,
        Size::from_wh(tree.size().width(), tree.size().height()).unwrap(),
        SvgSettings::default(),
    );
    surface.pop();
}

#[visreg(width = 304, height = 206)]
fn issue_199(surface: &mut Surface) {
    let data = std::fs::read(SVGS_PATH.join("issue199.svg")).unwrap();
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();

    surface.push_transform(&Transform::from_translate(2.0, 3.0));
    surface.draw_svg(
        &tree,
        Size::from_wh(tree.size().width(), tree.size().height()).unwrap(),
        SvgSettings::default(),
    );
    surface.pop();
}

#[visreg(svg)]
fn issue291() {}

#[visreg(svg)]
fn issue293() {}

fn typst_issue_5509_common(document: &mut Document, name: &str) {
    const SCALE_FACTOR: f32 = 0.5;

    let data = std::fs::read(SVGS_PATH.join(name)).unwrap();
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();
    let size = tree.size();

    let mut page = document.start_page_with(
        PageSettings::from_wh(size.width() * SCALE_FACTOR, size.height() * SCALE_FACTOR).unwrap(),
    );
    let mut surface = page.surface();

    let mut stream_builder = surface.stream_builder();
    let mut sur = stream_builder.surface();
    sur.draw_svg(
        &tree,
        Size::from_wh(tree.size().width(), tree.size().height()).unwrap(),
        SvgSettings {
            embed_text: true,
            ..Default::default()
        },
    );
    sur.finish();
    let stream = stream_builder.finish();
    let graphic = Graphic::new(stream, true);

    surface.push_transform(&krilla::geom::Transform::from_scale(
        SCALE_FACTOR,
        SCALE_FACTOR,
    ));
    surface.draw_graphic(graphic);
    surface.pop();
}

// Unfortunately since macOS 26, it seems like a new bug was introduced such that
// the below issues render incorrectly when rendered as a thumbnail (the one shown
// in Finder), but it still displays fine in Apple Preview. Therefore, we have
// to disable the tests.

#[visreg(document, pdfium, quartz, ignore)]
fn typst_issue_5509_1(document: &mut Document) {
    typst_issue_5509_common(document, "custom_typst_issue_5509_1.svg");
}

#[visreg(document, pdfium, quartz, ignore)]
fn typst_issue_5509_2(document: &mut Document) {
    typst_issue_5509_common(document, "custom_typst_issue_5509_2.svg");
}

#[visreg(document, pdfium, quartz, ignore)]
fn typst_issue_5509_3(document: &mut Document) {
    typst_issue_5509_common(document, "custom_typst_issue_5509_3.svg");
}

#[visreg]
fn svg_with_filter(surface: &mut Surface) {
    let data = std::fs::read(SVGS_PATH.join("small_text_with_filter.svg")).unwrap();
    let tree = usvg::Tree::from_data(
        &data,
        &usvg::Options {
            fontdb: FONTDB.clone(),
            ..usvg::Options::default()
        },
    )
    .unwrap();

    surface.draw_svg(
        &tree,
        Size::from_wh(tree.size().width(), tree.size().height()).unwrap(),
        SvgSettings::default(),
    );
}

include!("svg_generated.rs");

fn filters_in_groups(document: &mut Document, scale: f32) {
    let data = std::fs::read(SVGS_PATH.join("filters_in_groups.svg")).unwrap();
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();
    let size = Size::from_wh(tree.size().width() * scale, tree.size().height() * scale).unwrap();

    let mut page =
        document.start_page_with(PageSettings::from_wh(size.width(), size.height()).unwrap());
    let mut surface = page.surface();
    surface.draw_svg(
        &tree,
        size,
        SvgSettings {
            filter_scale: 1.0,
            ..Default::default()
        },
    );
    surface.finish();
    page.finish();
}

#[snapshot(document)]
fn svg_filter_upscaled(document: &mut Document) {
    filters_in_groups(document, 10.0);
}

#[visreg(document)]
fn svg_filter_upscaled(document: &mut Document) {
    filters_in_groups(document, 10.0);
}

#[snapshot(document)]
fn svg_filter_downscaled(document: &mut Document) {
    filters_in_groups(document, 0.1);
}
