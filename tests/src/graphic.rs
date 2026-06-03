use krilla::geom::{Size, Transform};
use krilla::graphic::Graphic;
use krilla::mask::MaskType;
use krilla::page::Page;
use krilla_macros::snapshot;
use krilla_svg::{SurfaceExt, SvgSettings};

use crate::svg::sample_svg;
use crate::{basic_mask, green_fill, rect_to_path, red_fill};

#[snapshot]
fn graphic(page: &mut Page) {
    let mut surface = page.surface();

    let mut stream_builder = surface.stream_builder();
    let mut stream_surface = stream_builder.surface();
    stream_surface.set_fill(Some(red_fill(0.5)));
    stream_surface.draw_path(&rect_to_path(0.0, 0.0, 20.0, 20.0));
    stream_surface.finish();
    let stream = stream_builder.finish();
    let graphic = Graphic::new(stream, false);

    surface.draw_graphic(graphic.clone());
    surface.push_transform(&Transform::from_translate(20.0, 20.0));
    surface.draw_graphic(graphic.clone());
    surface.pop();

    surface.push_transform(&Transform::from_translate(40.0, 40.0));
    surface.draw_graphic(graphic.clone());
    surface.pop();
}

#[snapshot]
fn graphic_svg(page: &mut Page) {
    let tree = sample_svg();
    let mut surface = page.surface();

    let mut stream_builder = surface.stream_builder();
    let mut stream_surface = stream_builder.surface();
    stream_surface.draw_svg(
        &tree,
        Size::from_wh(100.0, 100.0).unwrap(),
        SvgSettings::default(),
    );
    stream_surface.finish();
    let stream = stream_builder.finish();
    let graphic = Graphic::new(stream, true);

    surface.draw_graphic(graphic.clone());
    surface.push_transform(&Transform::from_translate(100.0, 100.0));
    surface.draw_graphic(graphic.clone());
    surface.pop();

    surface.finish()
}
