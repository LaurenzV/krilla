use crate::object::image::Image;
use crate::surface::Surface;
use crate::svg::ProcessContext;
use tiny_skia_path::{Size, Transform};

/// Render a filter into a surface by rasterizing it with `resvg` and drawing
/// the image.
///
/// Returns `None` if converting the filter was unsuccessful.
pub fn render(
    group: &usvg::Group,
    surface: &mut Surface,
    process_context: &ProcessContext,
) -> Option<()> {
    let layer_bbox = group.layer_bounding_box().transform(group.transform())?;

    let raster_scale = process_context.svg_settings.filter_scale;

    let pixmap_size = Size::from_wh(
        layer_bbox.width() * raster_scale,
        layer_bbox.height() * raster_scale,
    )?;

    let mut pixmap = tiny_skia::Pixmap::new(
        pixmap_size.width().round() as u32,
        pixmap_size.height().round() as u32,
    )?;

    let initial_transform = Transform::from_scale(raster_scale, raster_scale)
        .pre_concat(Transform::from_translate(-layer_bbox.x(), -layer_bbox.y()))
        // This one is a hack because resvg::render_node will take the absolute layer bbox into consideration
        // and translate by -layer_bbox.x() and -layer_bbox.y(), but we don't want that, so we
        // inverse it.
        .pre_concat(Transform::from_translate(
            group.abs_layer_bounding_box().x(),
            group.abs_layer_bounding_box().y(),
        ));

    resvg::render_node(
        &usvg::Node::Group(Box::new(group.clone())),
        initial_transform,
        &mut pixmap.as_mut(),
    );

    let encoded_image = pixmap.encode_png().ok()?;
    let image = Image::from_png(&encoded_image)?;
    let size = Size::from_wh(layer_bbox.width(), layer_bbox.height())?;

    surface.push_transform(&Transform::from_translate(layer_bbox.x(), layer_bbox.y()));
    surface.draw_image(image, size);
    surface.pop();

    Some(())
}
