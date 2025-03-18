use krilla::blend::BlendMode;
use krilla::geom::Transform;
use krilla::page::Page;
use krilla_macros::snapshot;

use crate::{blue_fill, red_fill};
use crate::{green_fill, rect_to_path};

#[snapshot(settings_2)]
fn stream_resource_cache(page: &mut Page) {
    let mut surface = page.surface();
    let path1 = rect_to_path(0.0, 0.0, 100.0, 100.0);
    let path2 = rect_to_path(50.0, 50.0, 150.0, 150.0);
    let path3 = rect_to_path(100.0, 100.0, 200.0, 200.0);

    surface.set_fill(green_fill(1.0));
    surface.fill_path(&path1);
    surface.set_fill(red_fill(1.0));
    surface.fill_path(&path2);
    surface.set_fill(blue_fill(1.0));
    surface.fill_path(&path3);
}

#[snapshot]
fn stream_nested_transforms(page: &mut Page) {
    let mut surface = page.surface();
    let path1 = rect_to_path(0.0, 0.0, 100.0, 100.0);

    surface.push_transform(&Transform::from_translate(50.0, 50.0));
    surface.set_fill(green_fill(1.0));
    surface.fill_path(&path1);
    surface.push_transform(&Transform::from_translate(100.0, 100.0));
    surface.set_fill(red_fill(1.0));
    surface.fill_path(&path1);

    surface.pop();
    surface.pop();
}

#[snapshot]
fn stream_reused_graphics_state(page: &mut Page) {
    let mut surface = page.surface();
    let path1 = rect_to_path(0.0, 0.0, 100.0, 100.0);
    surface.set_fill(green_fill(0.5));
    surface.fill_path(&path1);
    surface.push_blend_mode(BlendMode::ColorBurn);
    surface.set_fill(green_fill(0.5));
    surface.fill_path(&path1);
    surface.pop();
    surface.set_fill(green_fill(0.5));
    surface.fill_path(&path1);
}
