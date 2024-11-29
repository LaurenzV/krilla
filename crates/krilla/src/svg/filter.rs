use crate::object::image::Image;
use crate::surface::Surface;
use crate::svg::ProcessContext;
use std::sync::Arc;
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

    let raster_scale = if let Some(filter_scale) = process_context.svg_settings.filter_scale {
        filter_scale
    } else {
        // By default, I think having a scale of 4 in terms of user space units should be enough.
        // Meaning for example if you have a A4 PDF with dimensions 595x841 and an SVG with a
        // filter across the whole page, you end up with an image of 2380x3364.
        const DEFAULT_SCALE: f32 = 4.0;
        // Find out what dimensions the SVG will actually have in user space units inside of the
        // PDF.
        // Note that this is not a 100% accurate, because the `cur_transform` method of surface will
        // only return the transform in the current content stream, so it's not accurate in case we
        // are currently in a XObject. But it's as good as it gets.
        let actual_bbox = group
            .layer_bounding_box()
            .transform(surface.cur_transform())?
            .transform(group.transform())?;
        // Calculate the necessary scale in the x/y direction, and take the maximum of that.
        let scale = {
            let (x_scale, y_scale) = (
                (actual_bbox.width() / layer_bbox.width()),
                (actual_bbox.height() / layer_bbox.height()),
            );
            x_scale.max(y_scale) * DEFAULT_SCALE
        };

        let max_scale = {
            // Let's try to avoid generating images that have more than 5000 pixels in either direction.
            const PIXEL_THRESHOLD: f32 = 5000.0;
            let (x_scale, y_scale) = (
                (PIXEL_THRESHOLD / layer_bbox.width()),
                (PIXEL_THRESHOLD / layer_bbox.height()),
            );
            // Take the minimum of that.
            x_scale.min(y_scale)
        };

        // Take whichever is smaller.
        scale.min(max_scale)
    };

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

    // TODO: Don't re-encode?
    let encoded_image = pixmap.encode_png().ok()?;
    let image = Image::from_png(Arc::new(encoded_image))?;
    let size = Size::from_wh(layer_bbox.width(), layer_bbox.height())?;

    surface.push_transform(&Transform::from_translate(layer_bbox.x(), layer_bbox.y()));
    surface.draw_image(image, size);
    surface.pop();

    Some(())
}
