use std::num::NonZeroUsize;
use tiny_skia_path::Rect;
use krilla::{Document, PageSettings};
use krilla::page::{NumberingStyle, PageLabel};
use krilla_macros::{snapshot2, visreg, visreg2};
use crate::{blue_fill, green_fill, purple_fill, rect_to_path, red_fill};

#[snapshot2(document)]
fn page_label_complex(d: &mut Document) {
    d.start_page_with(PageSettings::new(200.0, 200.0));
    d.start_page_with(PageSettings::new(250.0, 200.0));

    let settings = PageSettings::new(250.0, 200.0).with_page_label(PageLabel::new(
        Some(NumberingStyle::LowerRoman),
        None,
        NonZeroUsize::new(2),
    ));

    d.start_page_with(settings);
}

fn media_box_impl(d: &mut Document, media_box: Rect) {
    let mut page =
        d.start_page_with(PageSettings::new(200.0, 200.0).with_media_box(Some(media_box)));
    let mut surface = page.surface();
    surface.fill_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), red_fill(0.5));
    surface.fill_path(&rect_to_path(100.0, 0.0, 200.0, 100.0), green_fill(0.5));
    surface.fill_path(&rect_to_path(0.0, 100.0, 100.0, 200.0), blue_fill(0.5));
    surface.fill_path(&rect_to_path(100.0, 100.0, 200.0, 200.0), purple_fill(0.5));
}

#[visreg2(document)]
fn custom_media_box_top_left(d: &mut Document) {
    media_box_impl(d, Rect::from_xywh(-100.0, -100.0, 200.0, 200.0).unwrap())
}

#[visreg2(document)]
fn custom_media_box_top_right(d: &mut Document) {
    media_box_impl(d, Rect::from_xywh(100.0, -100.0, 200.0, 200.0).unwrap())
}

#[visreg2(document)]
fn custom_media_box_bottom_left(d: &mut Document) {
    media_box_impl(d, Rect::from_xywh(-100.0, 100.0, 200.0, 200.0).unwrap())
}

#[visreg2(document)]
fn custom_media_box_bottom_right(d: &mut Document) {
    media_box_impl(d, Rect::from_xywh(100.0, 100.0, 200.0, 200.0).unwrap())
}

#[visreg2(document)]
fn custom_media_box_zoomed_out(d: &mut Document) {
    media_box_impl(d, Rect::from_xywh(-150.0, -200.0, 500.0, 500.0).unwrap())
}