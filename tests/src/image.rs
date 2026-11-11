use std::sync::Arc;

use krilla::configure::{ConfigurationBuilder, PdfVersion};
use krilla::geom::Size;
use krilla::image::{BitsPerComponent, CustomImage, Image, ImageColorspace};
use krilla::page::{Page, PageSettings};
use krilla::surface::Surface;
use krilla::Document;
use krilla_macros::{snapshot, visreg};

use crate::{
    load_custom_image, load_gif_image, load_jpg_image, load_png_image, load_webp_image, ASSETS_PATH,
};
use crate::{settings_1, SerializeSettings};

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

#[snapshot]
fn image_rgba8_opaque(page: &mut Page) {
    image_visreg_impl(&mut page.surface(), "rgba8_opaque.png", load_png_image);
}

#[visreg]
fn image_rgba8_opaque(surface: &mut Surface) {
    image_visreg_impl(surface, "rgba8_opaque.png", load_png_image);
}

#[visreg(all)]
fn image_rgba16_png(surface: &mut Surface) {
    image_visreg_impl(surface, "rgba16.png", load_png_image);
}

#[snapshot]
fn image_rgba16_opaque(page: &mut Page) {
    image_visreg_impl(&mut page.surface(), "rgba16_opaque.png", load_png_image);
}

#[visreg]
fn image_rgba16_opaque(surface: &mut Surface) {
    image_visreg_impl(surface, "rgba16_opaque.png", load_png_image);
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
    image_visreg_impl(surface, "ccyk_icc.jpg", load_jpg_image);
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

#[visreg(width = 100, height = 80)]
fn image_resized(surface: &mut Surface) {
    let image = load_png_image("rgba8.png");
    surface.draw_image(image, Size::from_wh(100.0, 80.0).unwrap());
}

#[snapshot]
fn image_rgb8(page: &mut Page) {
    let mut surface = page.surface();
    let image = load_png_image("rgb8.png");
    let size = Size::from_wh(image.size().0 as f32, image.size().1 as f32).unwrap();
    surface.draw_image(image, size);
}

// See https://github.com/typst/typst/issues/7268#issuecomment-3497148544.
// The image has a APP14 marker with CMYK but it actually is RGB. This was
// fixed in zune_jpeg 0.5.
#[snapshot]
fn image_rgb8_app14_cmyk(page: &mut Page) {
    let mut surface = page.surface();
    let image = load_jpg_image("rgb8_app14_cmyk.jpg");
    let size = Size::from_wh(image.size().0 as f32, image.size().1 as f32).unwrap();
    surface.draw_image(image, size);
}

#[snapshot]
fn image_luma8(page: &mut Page) {
    let mut surface = page.surface();
    let image = load_png_image("luma8.png");
    let size = Size::from_wh(image.size().0 as f32, image.size().1 as f32).unwrap();
    surface.draw_image(image, size);
}

#[snapshot]
fn image_rgba16(page: &mut Page) {
    let mut surface = page.surface();
    let image = load_png_image("rgba16.png");
    let size = Size::from_wh(image.size().0 as f32, image.size().1 as f32).unwrap();
    surface.draw_image(image, size);
}

#[snapshot]
fn image_luma16(page: &mut Page) {
    let mut surface = page.surface();
    let image = load_png_image("luma16.png");
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

#[snapshot(settings_2)]
// See https://github.com/LaurenzV/krilla/issues/152
fn image_no_device_cs(page: &mut Page) {
    let mut surface = page.surface();
    let image = load_png_image("rgb8.png");
    let size = Size::from_wh(image.size().0 as f32, image.size().1 as f32).unwrap();
    surface.draw_image(image, size);
}

#[derive(Clone, Hash)]
struct CmykImage {
    icc: Arc<Vec<u8>>,
}

impl CustomImage for CmykImage {
    fn color_channel(&self) -> &[u8] {
        &[0, 255, 255, 0]
    }

    fn alpha_channel(&self) -> Option<&[u8]> {
        None
    }

    fn bits_per_component(&self) -> BitsPerComponent {
        BitsPerComponent::Eight
    }

    fn size(&self) -> (u32, u32) {
        (1, 1)
    }

    fn icc_profile(&self) -> Option<&[u8]> {
        Some(&self.icc)
    }

    fn color_space(&self) -> ImageColorspace {
        ImageColorspace::Cmyk
    }
}

#[test]
fn image_cmyk_icc_v2_4_is_embedded() {
    let icc = std::fs::read(ASSETS_PATH.join("icc/eciCMYK_v2.icc")).unwrap();
    assert_eq!((icc[8], icc[9]), (2, 0x40));

    let image = Image::from_custom(CmykImage { icc: Arc::new(icc) }, false).unwrap();
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_version(PdfVersion::Pdf17)
            .finish()
            .unwrap(),
        ..settings_1()
    };
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(PageSettings::from_wh(100.0, 100.0).unwrap());
    let mut surface = page.surface();
    surface.draw_image(image, Size::from_wh(100.0, 100.0).unwrap());
    surface.finish();
    page.finish();
    let pdf = document.finish().unwrap();

    let contains = |needle: &[u8]| pdf.windows(needle.len()).any(|w| w == needle);
    assert!(contains(b"/ICCBased"));
    assert!(!contains(b"/DeviceCMYK"));
}
