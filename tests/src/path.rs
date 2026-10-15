use krilla::color::rgb;
use krilla::geom::PathBuilder;
use krilla::page::Page;
use krilla::paint::{LineCap, Paint, Stroke};
use krilla::surface::Surface;
use krilla_macros::{snapshot, visreg};

use crate::{cmyk_fill, gray_fill, rect_to_path, red_fill, red_stroke};

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

#[visreg(all, settings_6)]
fn path_line_zero_length_with_stroke(surface: &mut Surface) {
    for (i, line_cap) in [LineCap::Butt, LineCap::Round, LineCap::Square]
        .into_iter()
        .enumerate()
    {
        for j in 0..2 {
            let mut path = PathBuilder::new();
            let x = (i + 1) as f32 * 40.0;
            let y = (j + 1) as f32 * 40.0;
            path.move_to(x, y);
            path.line_to(x, y + j as f32 * 20.0);
            let path = path.finish().unwrap();

            let mut stroke = red_stroke(1.0, 10.0);
            stroke.line_cap = line_cap;
            surface.set_stroke(Some(stroke));

            surface.draw_path(&path);
        }
    }
}
