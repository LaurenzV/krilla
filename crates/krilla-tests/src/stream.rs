use krilla::blend::BlendMode;
use krilla::geom::{Size, Transform};
use krilla::page::Page;
use krilla_macros::snapshot;

use crate::{blue_fill, load_png_image, red_fill};
use crate::{green_fill, rect_to_path};

#[snapshot(settings_2)]
fn stream_resource_cache(page: &mut Page) {
    let mut surface = page.surface();
    let path1 = rect_to_path(0.0, 0.0, 100.0, 100.0);
    let path2 = rect_to_path(50.0, 50.0, 150.0, 150.0);
    let path3 = rect_to_path(100.0, 100.0, 200.0, 200.0);

    surface.set_fill(Some(green_fill(1.0)));
    surface.draw_path(&path1);
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&path2);
    surface.set_fill(Some(blue_fill(1.0)));
    surface.draw_path(&path3);
}

#[snapshot]
fn stream_nested_transforms(page: &mut Page) {
    let mut surface = page.surface();
    let path1 = rect_to_path(0.0, 0.0, 100.0, 100.0);

    surface.push_transform(&Transform::from_translate(50.0, 50.0));
    surface.set_fill(Some(green_fill(1.0)));
    surface.draw_path(&path1);
    surface.push_transform(&Transform::from_translate(100.0, 100.0));
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&path1);

    surface.pop();
    surface.pop();
}

#[snapshot]
fn stream_reused_graphics_state(page: &mut Page) {
    let mut surface = page.surface();
    let path1 = rect_to_path(0.0, 0.0, 100.0, 100.0);
    surface.set_fill(Some(green_fill(0.5)));
    surface.draw_path(&path1);
    surface.push_blend_mode(BlendMode::ColorBurn);
    surface.set_fill(Some(green_fill(0.5)));
    surface.draw_path(&path1);
    surface.pop();
    surface.set_fill(Some(green_fill(0.5)));
    surface.draw_path(&path1);
}

// Make sure page streams and images are flate encoded with default settings.
#[snapshot(settings_29)]
fn stream_compress_by_default(page: &mut Page) {
    let mut surface = page.surface();
    let path1 = rect_to_path(0.0, 0.0, 100.0, 100.0);
    surface.set_fill(Some(green_fill(0.5)));
    surface.draw_path(&path1);

    let image = load_png_image("luma8.png");
    let size = Size::from_wh(image.size().0 as f32, image.size().1 as f32).unwrap();
    surface.draw_image(image, size);
}
