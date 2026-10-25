use krilla::blend::BlendMode;
use krilla::geom::{Point, Size, Transform};
use krilla::page::{Page, PageSettings};
use krilla::text::{Font, TextDirection};
use krilla_macros::snapshot;

use crate::embed::file_1;
use crate::{blue_fill, load_png_image, red_fill, NOTO_SANS};
use crate::{green_fill, rect_to_path, Document};

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

// Make sure page streams, images, etc. are flate encoded with default settings.
#[snapshot(document, settings_29)]
fn stream_compress_by_default(document: &mut Document) {
    document.embed_file(file_1());

    let mut page = document.start_page();
    let mut surface = page.surface();
    let path1 = rect_to_path(0.0, 0.0, 100.0, 100.0);
    surface.set_fill(Some(green_fill(0.5)));
    surface.draw_path(&path1);

    let image = load_png_image("rgba8.png");
    let size = Size::from_wh(100.0, 100.0).unwrap();
    surface.draw_image(image, size);

    let font = Font::new(NOTO_SANS.clone(), 0).unwrap();
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_text(
        Point::from_xy(0.0, 50.0),
        font,
        20.0,
        "Hello World",
        false,
        TextDirection::Auto,
    );
}

#[test]
fn stream_absolute_transform() {
    let mut d = Document::new();
    let mut page = d.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());
    let mut surface = page.surface();

    let scale = Transform::from_scale(2.0, 3.0);
    let translate = Transform::from_translate(5.0, 0.0);
    surface.push_transform(&scale);
    surface.push_isolated();
    surface.push_transform(&translate);

    let scaled_translate = Transform::from_row(2.0, 0.0, 0.0, 3.0, 10.0, 0.0);
    // The isolated group is drawn with `scale`, so it starts a new content stream.
    assert_eq!(surface.cur_transform(), translate);
    assert_eq!(surface.absolute_transform(), scaled_translate);

    {
        let mut stream_builder = surface.stream_builder();
        let sub_surface = stream_builder.surface();
        assert_eq!(sub_surface.cur_transform(), Transform::identity());
        assert_eq!(sub_surface.absolute_transform(), scaled_translate);
    }

    surface.pop();
    surface.pop();
    assert_eq!(surface.absolute_transform(), scale);
    surface.pop();
    assert_eq!(surface.absolute_transform(), Transform::identity());

    surface.finish();
    page.finish();
    d.finish().unwrap();
}
