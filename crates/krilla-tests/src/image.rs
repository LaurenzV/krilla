use krilla::surface::Surface;
use krilla::{Document, Image, Page, Size};
use krilla_macros::{snapshot, visreg};

use crate::{
    load_custom_image, load_gif_image, load_jpg_image, load_png_image, load_webp_image, ASSETS_PATH,
};

fn image_visreg_impl(surface: &mut Surface, name: &str, load_fn: fn(&str) -> Image) {
    let image = load_fn(name);
    let size = image.size();
    surface.draw_image(image, Size::from_wh(size.0 as f32, size.1 as f32).unwrap());
}

#[visreg(all)]
fn image_luma8_png(surface: &mut Surface) {
    image_visreg_impl(surface, "luma8.png", load_png_image);
}

#[visreg]
fn image_luma8_custom_png(surface: &mut Surface) {
    image_visreg_impl(surface, "luma8.png", load_custom_image);
}

#[visreg(all)]
fn image_luma16_png(surface: &mut Surface) {
    image_visreg_impl(surface, "luma16.png", load_png_image);
}

#[visreg(all)]
fn image_rgb8_png(surface: &mut Surface) {
    image_visreg_impl(surface, "rgb8.png", load_png_image);
}

#[visreg]
fn image_rgb8_custom_png(surface: &mut Surface) {
    image_visreg_impl(surface, "rgb8.png", load_custom_image);
}

#[visreg(all)]
fn image_rgb16_png(surface: &mut Surface) {
    image_visreg_impl(surface, "rgb16.png", load_png_image);
}

#[visreg(all)]
fn image_rgba8_png(surface: &mut Surface) {
    image_visreg_impl(surface, "rgba8.png", load_png_image);
}

#[visreg(all)]
fn image_rgba16_png(surface: &mut Surface) {
    image_visreg_impl(surface, "rgba16.png", load_png_image);
}

#[visreg]
fn image_rgba16_custom_png(surface: &mut Surface) {
    image_visreg_impl(surface, "rgba16.png", load_custom_image);
}

#[visreg(pdfium, mupdf, pdfbox, poppler, quartz)]
fn image_luma8_jpg(surface: &mut Surface) {
    image_visreg_impl(surface, "luma8.jpg", load_jpg_image);
}

#[visreg(pdfium, mupdf, pdfbox, poppler, quartz)]
fn image_rgb8_jpg(surface: &mut Surface) {
    image_visreg_impl(surface, "rgb8.jpg", load_jpg_image);
}

#[visreg(pdfium, mupdf, pdfbox, poppler, quartz)]
fn image_cmyk_jpg(surface: &mut Surface) {
    image_visreg_impl(surface, "cmyk.jpg", load_jpg_image);
}

#[visreg(all)]
fn image_rgb8_gif(surface: &mut Surface) {
    image_visreg_impl(surface, "rgb8.gif", load_gif_image);
}

#[visreg(all)]
fn image_rgba8_gif(surface: &mut Surface) {
    image_visreg_impl(surface, "rgba8.gif", load_gif_image);
}

#[visreg(all)]
fn image_rgba8_webp(surface: &mut Surface) {
    image_visreg_impl(surface, "rgba8.webp", load_webp_image);
}

#[visreg]
fn image_cmyk_icc_jpg(surface: &mut Surface) {
    image_visreg_impl(surface, "cmyk_icc.jpg", load_jpg_image);
}

#[visreg]
fn image_rgb8_icc_jpg(surface: &mut Surface) {
    image_visreg_impl(surface, "rgb8_icc.jpg", load_jpg_image);
}

#[visreg]
fn image_luma8_icc_png(surface: &mut Surface) {
    image_visreg_impl(surface, "luma8_icc.png", load_png_image);
}

#[visreg]
fn image_rgba8_icc_png(surface: &mut Surface) {
    image_visreg_impl(surface, "rgba8_icc.png", load_png_image);
}

#[visreg]
fn image_rgb8_icc_png(surface: &mut Surface) {
    image_visreg_impl(surface, "rgb8_icc.png", load_png_image);
}

#[visreg]
fn image_resized(surface: &mut Surface) {
    let image = load_png_image("rgba8.png");
    surface.draw_image(image, Size::from_wh(100.0, 80.0).unwrap());
}

#[snapshot]
fn image(page: &mut Page) {
    let mut surface = page.surface();
    let image = load_png_image("rgb8.png");
    let size = Size::from_wh(image.size().0 as f32, image.size().1 as f32).unwrap();
    surface.draw_image(image, size);
}

#[snapshot(document)]
fn image_deduplicate(document: &mut Document) {
    let size = load_png_image("luma8.png").size();
    let size = Size::from_wh(size.0 as f32, size.1 as f32).unwrap();
    let mut page = document.start_page();
    let mut surface = page.surface();
    surface.draw_image(load_png_image("luma8.png"), size);
    surface.draw_image(load_png_image("luma8.png"), size);
    surface.finish();

    page.finish();

    let mut page = document.start_page();
    let mut surface = page.surface();
    surface.draw_image(load_png_image("luma8.png"), size);
}

#[snapshot]
fn image_interpolate(page: &mut Page) {
    let image = Image::from_png(
        std::fs::read(ASSETS_PATH.join("images").join("rgba8.png"))
            .unwrap()
            .into(),
        true,
    )
    .unwrap();
    let size = image.size();
    let size = Size::from_wh(size.0 as f32, size.1 as f32).unwrap();
    let mut surface = page.surface();
    surface.draw_image(image, size);
}
