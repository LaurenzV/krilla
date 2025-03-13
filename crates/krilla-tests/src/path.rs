use krilla::Page;
use krilla_macros::snapshot;

use crate::{cmyk_fill, gray_fill, rect_to_path, red_fill};

#[snapshot(single_page)]
fn path_with_rgb(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
    let fill = red_fill(1.0);
    surface.set_fill(fill);
    surface.fill_path(&path);
}

#[snapshot(single_page)]
fn path_with_luma(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
    let fill = gray_fill(1.0);
    surface.set_fill(fill);
    surface.fill_path(&path);
}

#[snapshot(single_page)]
fn path_with_rgb_and_opacity(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
    let fill = red_fill(0.5);
    surface.set_fill(fill);
    surface.fill_path(&path);
}

#[snapshot(single_page)]
fn path_with_cmyk(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
    let fill = cmyk_fill(1.0);
    surface.set_fill(fill);
    surface.fill_path(&path);
}
