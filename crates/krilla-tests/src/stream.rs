use krilla::font::Font;
use krilla::mask::MaskType;
use krilla::page::Page;
use krilla::paint::{LinearGradient, Paint, SpreadMethod};
use krilla::path::{Fill, Stroke};
use krilla::surface::TextDirection;
use krilla::surface::{BlendMode, Surface};
use krilla::{Data, Point, Size, Transform};
use krilla_macros::{snapshot, visreg};
use krilla_svg::{SurfaceExt, SvgSettings};

use crate::{
    basic_mask, cmyk_fill, gray_fill, green_fill, load_png_image, rect_to_path, FONTDB,
    LATIN_MODERN_ROMAN,
};
use crate::{
    blue_fill, blue_stroke, red_fill, red_stroke, stops_with_3_solid_1, NOTO_COLOR_EMOJI_COLR,
    NOTO_SANS, NOTO_SANS_CJK, NOTO_SANS_DEVANAGARI, SVGS_PATH,
};

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
