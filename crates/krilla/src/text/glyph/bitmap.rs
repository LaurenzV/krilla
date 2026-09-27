use skrifa::bitmap::{BitmapData, BitmapGlyph, Origin};
use skrifa::MetadataProvider;

use crate::geom::{Size, Transform};
use crate::graphics::image::Image;
use crate::surface::Surface;
use crate::text::Font;
use crate::text::GlyphId;

pub(crate) fn has_bitmap_data(font: &Font, glyph: GlyphId) -> bool {
    // We only support PNG.
    get_bitmap_glyph(font, glyph).is_some_and(|b| matches!(b.data, BitmapData::Png(_)))
}

pub(crate) fn get_bitmap_glyph(font: &Font, glyph: GlyphId) -> Option<BitmapGlyph<'_>> {
    let bitmap_strikes = font.font_ref().bitmap_strikes();
    bitmap_strikes.glyph_for_size(skrifa::instance::Size::unscaled(), glyph.to_skrifa())
}

/// Draw a bitmap-based glyph on a surface.
pub(crate) fn draw_glyph(font: Font, glyph: GlyphId, surface: &mut Surface) -> Option<()> {
    let metrics = font
        .font_ref()
        .metrics(skrifa::instance::Size::unscaled(), font.location_ref());

    let bitmap_strikes = font.font_ref().bitmap_strikes();
    let bitmap_glyph =
        bitmap_strikes.glyph_for_size(skrifa::instance::Size::unscaled(), glyph.to_skrifa())?;

    let upem = metrics.units_per_em as f32;

    match bitmap_glyph.data {
        BitmapData::Png(data) => {
            let image = Image::from_png(data.to_vec().into(), false).ok()?;
            let size = Size::from_wh(image.size().0 as f32, image.size().1 as f32).unwrap();

            // Adapted from vello.
            let scale_factor = upem / (bitmap_glyph.ppem_y);
            let outer_bearing = if font.is_apple_color_emoji() {
                (0.0, upem / 8.0)
            } else {
                (bitmap_glyph.bearing_x, -bitmap_glyph.bearing_y)
            };
            let mut transform = Transform::from_translate(outer_bearing.0, outer_bearing.1)
                .pre_concat(Transform::from_scale(scale_factor, scale_factor))
                .pre_concat(Transform::from_translate(
                    bitmap_glyph.inner_bearing_x,
                    -bitmap_glyph.inner_bearing_y,
                ));

            transform = match bitmap_glyph.placement_origin {
                Origin::TopLeft => transform,
                Origin::BottomLeft => {
                    transform.pre_concat(Transform::from_translate(0.0, -(image.size().1 as f32)))
                }
            };

            surface.push_transform(&transform);
            surface.draw_image(image, size);
            surface.pop();

            Some(())
        }
        BitmapData::Bgra(_) => None,
        BitmapData::Mask(_) => None,
    }
}
