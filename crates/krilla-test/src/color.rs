use crate::{cmyk_fill, rect_to_path, red_fill};
use krilla::path::Fill;
use krilla::surface::Surface;
use krilla::Page;
use krilla_macros::{snapshot2, visreg2};

#[snapshot2(single_page, settings_18)]
fn icc_v2_srgb(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), red_fill(1.0));
}

#[snapshot2(single_page, settings_18)]
fn icc_v2_sgrey(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), Fill::default());
}

#[visreg2(all)]
fn cmyk_color(surface: &mut Surface) {
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    surface.fill_path(&path, cmyk_fill(1.0));
}

#[visreg2(all, settings_6)]
fn cmyk_with_icc(surface: &mut Surface) {
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    surface.fill_path(&path, cmyk_fill(1.0));
}
