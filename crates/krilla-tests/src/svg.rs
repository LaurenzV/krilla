#![allow(non_snake_case)]

use krilla::geom::{Size, Transform};
use krilla::surface::Surface;
use krilla_macros::visreg;
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

#[visreg]
fn svg_resized(surface: &mut Surface) {
    surface.draw_svg(
        &sample_svg(),
        Size::from_wh(120.0, 80.0).unwrap(),
        SvgSettings::default(),
    );
}

#[visreg]
fn svg_should_be_clipped(surface: &mut Surface) {
    let data =
        std::fs::read(SVGS_PATH.join("custom_paint_servers_pattern_patterns_2.svg")).unwrap();
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();

    surface.push_transform(&Transform::from_translate(100.0, 0.0));
    surface.draw_svg(
        &tree,
        Size::from_wh(tree.size().width(), tree.size().height()).unwrap(),
        SvgSettings::default(),
    );
    surface.pop();
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
