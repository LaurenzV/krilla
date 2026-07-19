use krilla::geom::Size;
use krilla::image::Image;
use krilla::page::Page;
use krilla::surface::Surface;
use krilla_macros::{snapshot, visreg};

use crate::ASSETS_PATH;

fn png_impl(surface: &mut Surface, name: &str) {
    let image = Image::from_png(
        std::fs::read(
            ASSETS_PATH
                .join("images/png_native")
                .join(format!("{name}.png")),
        )
        .unwrap()
        .into(),
        false,
    )
    .unwrap();
    let (width, height) = image.size();
    let scale = 200.0 / width as f32;
    let size = Size::from_wh(200.0, height as f32 * scale).unwrap();
    surface.draw_image(image, size);
}

#[snapshot]
fn png_grayscale_1(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_1");
}

#[visreg]
fn png_grayscale_1(surface: &mut Surface) {
    png_impl(surface, "grayscale_1");
}

#[snapshot]
fn png_grayscale_2(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_2");
}

#[visreg]
fn png_grayscale_2(surface: &mut Surface) {
    png_impl(surface, "grayscale_2");
}

#[snapshot]
fn png_grayscale_4(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_4");
}

#[visreg]
fn png_grayscale_4(surface: &mut Surface) {
    png_impl(surface, "grayscale_4");
}

#[snapshot]
fn png_grayscale_8(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_8");
}

#[visreg]
fn png_grayscale_8(surface: &mut Surface) {
    png_impl(surface, "grayscale_8");
}

#[snapshot]
fn png_grayscale_16(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_16");
}

#[visreg]
fn png_grayscale_16(surface: &mut Surface) {
    png_impl(surface, "grayscale_16");
}

#[snapshot]
fn png_grayscale_8_icc(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_8_icc");
}

#[visreg]
fn png_grayscale_8_icc(surface: &mut Surface) {
    png_impl(surface, "grayscale_8_icc");
}

#[snapshot]
fn png_grayscale_trns_1(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_trns_1");
}

#[visreg]
fn png_grayscale_trns_1(surface: &mut Surface) {
    png_impl(surface, "grayscale_trns_1");
}

#[snapshot]
fn png_grayscale_trns_2(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_trns_2");
}

#[visreg]
fn png_grayscale_trns_2(surface: &mut Surface) {
    png_impl(surface, "grayscale_trns_2");
}

#[snapshot]
fn png_grayscale_trns_4(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_trns_4");
}

#[visreg]
fn png_grayscale_trns_4(surface: &mut Surface) {
    png_impl(surface, "grayscale_trns_4");
}

#[snapshot]
fn png_grayscale_trns_8(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_trns_8");
}

#[visreg]
fn png_grayscale_trns_8(surface: &mut Surface) {
    png_impl(surface, "grayscale_trns_8");
}

#[snapshot]
fn png_grayscale_trns_16(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_trns_16");
}

#[visreg]
fn png_grayscale_trns_16(surface: &mut Surface) {
    png_impl(surface, "grayscale_trns_16");
}

#[snapshot]
fn png_grayscale_8_interlaced(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_8_interlaced");
}

#[visreg]
fn png_grayscale_8_interlaced(surface: &mut Surface) {
    png_impl(surface, "grayscale_8_interlaced");
}

#[snapshot]
fn png_rgb_8(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_8");
}

#[visreg]
fn png_rgb_8(surface: &mut Surface) {
    png_impl(surface, "rgb_8");
}

#[snapshot]
fn png_rgb_16(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_16");
}

#[visreg]
fn png_rgb_16(surface: &mut Surface) {
    png_impl(surface, "rgb_16");
}

#[snapshot]
fn png_rgb_8_icc(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_8_icc");
}

#[visreg]
fn png_rgb_8_icc(surface: &mut Surface) {
    png_impl(surface, "rgb_8_icc");
}

#[snapshot]
fn png_rgb_8_multiple_idat(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_8_multiple_idat");
}

#[visreg]
fn png_rgb_8_multiple_idat(surface: &mut Surface) {
    png_impl(surface, "rgb_8_multiple_idat");
}

#[snapshot]
fn png_rgb_trns_8(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_trns_8");
}

#[visreg]
fn png_rgb_trns_8(surface: &mut Surface) {
    png_impl(surface, "rgb_trns_8");
}

#[snapshot]
fn png_rgb_trns_16(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_trns_16");
}

#[visreg]
fn png_rgb_trns_16(surface: &mut Surface) {
    png_impl(surface, "rgb_trns_16");
}

#[snapshot]
fn png_rgb_indexed_1(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_indexed_1");
}

#[visreg]
fn png_rgb_indexed_1(surface: &mut Surface) {
    png_impl(surface, "rgb_indexed_1");
}

#[snapshot]
fn png_rgb_indexed_2(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_indexed_2");
}

#[visreg]
fn png_rgb_indexed_2(surface: &mut Surface) {
    png_impl(surface, "rgb_indexed_2");
}

#[snapshot]
fn png_rgb_indexed_4(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_indexed_4");
}

#[visreg]
fn png_rgb_indexed_4(surface: &mut Surface) {
    png_impl(surface, "rgb_indexed_4");
}

#[snapshot]
fn png_rgb_indexed_8(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_indexed_8");
}

#[visreg]
fn png_rgb_indexed_8(surface: &mut Surface) {
    png_impl(surface, "rgb_indexed_8");
}

#[snapshot]
fn png_rgb_indexed_trns_1(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_indexed_trns_1");
}

#[visreg]
fn png_rgb_indexed_trns_1(surface: &mut Surface) {
    png_impl(surface, "rgb_indexed_trns_1");
}

#[snapshot]
fn png_rgb_indexed_trns_2(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_indexed_trns_2");
}

#[visreg]
fn png_rgb_indexed_trns_2(surface: &mut Surface) {
    png_impl(surface, "rgb_indexed_trns_2");
}

#[snapshot]
fn png_rgb_indexed_trns_4(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_indexed_trns_4");
}

#[visreg]
fn png_rgb_indexed_trns_4(surface: &mut Surface) {
    png_impl(surface, "rgb_indexed_trns_4");
}

#[snapshot]
fn png_rgb_indexed_trns_8(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_indexed_trns_8");
}

#[visreg]
fn png_rgb_indexed_trns_8(surface: &mut Surface) {
    png_impl(surface, "rgb_indexed_trns_8");
}

#[snapshot]
fn png_rgb_indexed_trns_binary_8(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_indexed_trns_binary_8");
}

#[visreg]
fn png_rgb_indexed_trns_binary_8(surface: &mut Surface) {
    png_impl(surface, "rgb_indexed_trns_binary_8");
}

#[snapshot]
fn png_grayscale_alpha_8(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_alpha_8");
}

#[visreg]
fn png_grayscale_alpha_8(surface: &mut Surface) {
    png_impl(surface, "grayscale_alpha_8");
}

#[snapshot]
fn png_grayscale_alpha_16(page: &mut Page) {
    png_impl(&mut page.surface(), "grayscale_alpha_16");
}

#[visreg]
fn png_grayscale_alpha_16(surface: &mut Surface) {
    png_impl(surface, "grayscale_alpha_16");
}

#[snapshot]
fn png_rgba_8(page: &mut Page) {
    png_impl(&mut page.surface(), "rgba_8");
}

#[visreg]
fn png_rgba_8(surface: &mut Surface) {
    png_impl(surface, "rgba_8");
}

#[snapshot]
fn png_rgba_16(page: &mut Page) {
    png_impl(&mut page.surface(), "rgba_16");
}

#[visreg]
fn png_rgba_16(surface: &mut Surface) {
    png_impl(surface, "rgba_16");
}

#[snapshot]
fn png_rgb_8_filter_none(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_8_filter_none");
}

#[visreg]
fn png_rgb_8_filter_none(surface: &mut Surface) {
    png_impl(surface, "rgb_8_filter_none");
}

#[snapshot]
fn png_rgb_8_filter_sub(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_8_filter_sub");
}

#[visreg]
fn png_rgb_8_filter_sub(surface: &mut Surface) {
    png_impl(surface, "rgb_8_filter_sub");
}

#[snapshot]
fn png_rgb_8_filter_up(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_8_filter_up");
}

#[visreg]
fn png_rgb_8_filter_up(surface: &mut Surface) {
    png_impl(surface, "rgb_8_filter_up");
}

#[snapshot]
fn png_rgb_8_filter_average(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_8_filter_average");
}

#[visreg]
fn png_rgb_8_filter_average(surface: &mut Surface) {
    png_impl(surface, "rgb_8_filter_average");
}

#[snapshot]
fn png_rgb_8_filter_paeth(page: &mut Page) {
    png_impl(&mut page.surface(), "rgb_8_filter_paeth");
}

#[visreg]
fn png_rgb_8_filter_paeth(surface: &mut Surface) {
    png_impl(surface, "rgb_8_filter_paeth");
}
