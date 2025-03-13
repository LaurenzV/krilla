use std::num::NonZeroUsize;

use krilla::page::{NumberingStyle, PageLabel};
use krilla::path::Fill;
use krilla::{Document, Page, PageSettings, Rect};
use krilla_macros::{snapshot, visreg};
use tiny_skia_path::PathBuilder;

use crate::{blue_fill, green_fill, purple_fill, rect_to_path, red_fill};

fn media_box_impl(d: &mut Document, media_box: Rect) {
    let mut page =
        d.start_page_with(PageSettings::new(200.0, 200.0).with_media_box(Some(media_box)));
    let mut surface = page.surface();
    surface.set_fill(red_fill(0.5));
    surface.fill_path(&rect_to_path(0.0, 0.0, 100.0, 100.0));
    surface.set_fill(green_fill(0.5));
    surface.fill_path(&rect_to_path(100.0, 0.0, 200.0, 100.0));
    surface.set_fill(blue_fill(0.5));
    surface.fill_path(&rect_to_path(0.0, 100.0, 100.0, 200.0));
    surface.set_fill(purple_fill(0.5));
    surface.fill_path(&rect_to_path(100.0, 100.0, 200.0, 200.0));
}

#[snapshot(document)]
fn page_label(d: &mut Document) {
    d.start_page_with(PageSettings::new(200.0, 200.0));
    d.start_page_with(PageSettings::new(250.0, 200.0));

    let settings = PageSettings::new(250.0, 200.0).with_page_label(PageLabel::new(
        Some(NumberingStyle::LowerRoman),
        None,
        NonZeroUsize::new(2),
    ));

    d.start_page_with(settings);
}

#[snapshot(document)]
fn page_with_crop_bleeding_trim_art_boxes(d: &mut Document) {
    // Create page settings with different boxes
    let page_settings = PageSettings::new(200.0, 200.0)
        .with_media_box(Some(Rect::from_xywh(0.0, 0.0, 200.0, 200.0).unwrap()))
        .with_crop_box(Some(Rect::from_xywh(10.0, 10.0, 180.0, 180.0).unwrap()))
        .with_bleed_box(Some(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap()))
        .with_trim_box(Some(Rect::from_xywh(30.0, 30.0, 140.0, 140.0).unwrap()))
        .with_art_box(Some(Rect::from_xywh(40.0, 40.0, 120.0, 120.0).unwrap()));

    let _ = d.start_page_with(page_settings);
}

#[visreg(document)]
fn custom_media_box_top_left(d: &mut Document) {
    media_box_impl(d, Rect::from_xywh(-100.0, -100.0, 200.0, 200.0).unwrap())
}

#[visreg(document)]
fn custom_media_box_top_right(d: &mut Document) {
    media_box_impl(d, Rect::from_xywh(100.0, -100.0, 200.0, 200.0).unwrap())
}

#[visreg(document)]
fn custom_media_box_bottom_left(d: &mut Document) {
    media_box_impl(d, Rect::from_xywh(-100.0, 100.0, 200.0, 200.0).unwrap())
}

#[visreg(document)]
fn custom_media_box_bottom_right(d: &mut Document) {
    media_box_impl(d, Rect::from_xywh(100.0, 100.0, 200.0, 200.0).unwrap())
}

#[visreg(document)]
fn custom_media_box_zoomed_out(d: &mut Document) {
    media_box_impl(d, Rect::from_xywh(-150.0, -200.0, 500.0, 500.0).unwrap())
}
