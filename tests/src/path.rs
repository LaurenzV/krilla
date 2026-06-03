use krilla::page::Page;
use krilla::surface::Surface;
use krilla_macros::{snapshot, visreg};

use crate::{cmyk_fill, gray_fill, rect_to_path, red_fill};

#[snapshot]
fn path_with_rgb(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
    let fill = red_fill(1.0);
    surface.set_fill(Some(fill));
    surface.draw_path(&path);
}

#[snapshot]
fn path_with_luma(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
    let fill = gray_fill(1.0);
    surface.set_fill(Some(fill));
    surface.draw_path(&path);
}

#[snapshot]
fn path_with_rgb_and_opacity(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
    let fill = red_fill(0.5);
    surface.set_fill(Some(fill));
    surface.draw_path(&path);
}

#[snapshot]
fn path_with_cmyk(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
    let fill = cmyk_fill(1.0);
    surface.set_fill(Some(fill));
    surface.draw_path(&path);
}

#[visreg(all)]
fn path_with_cmyk(surface: &mut Surface) {
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&path);
}

#[visreg(all, settings_6)]
fn path_with_cmyk_icc(surface: &mut Surface) {
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&path);
}
