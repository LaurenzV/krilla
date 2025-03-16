use krilla::path::Fill;
use krilla::surface::Surface;
use krilla::Page;
use krilla_macros::{snapshot, visreg};

use crate::{cmyk_fill, rect_to_path, red_fill};

#[visreg(all)]
fn cmyk_color(surface: &mut Surface) {
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    surface.set_fill(cmyk_fill(1.0));
    surface.fill_path(&path);
}

#[visreg(all, settings_6)]
fn cmyk_with_icc(surface: &mut Surface) {
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    surface.set_fill(cmyk_fill(1.0));
    surface.fill_path(&path);
}
