use crate::canvas::{Canvas, Surface};
use crate::font::Font;
use crate::object::image::Image;
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider, Tag};
use tiny_skia_path::{Size, Transform};

pub fn draw_glyph(font: &Font, glyph: GlyphId) -> Option<Canvas> {
    let font_ref = font.font_ref();
    let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), font.location_ref());

    if let Ok(table) = font_ref.sbix() {
        if let Some((strike, data)) = table
            .strikes()
            .iter()
            .map(|s| s.ok())
            .filter_map(|s| Some((s.clone()?, s?.glyph_data(glyph).ok()??)))
            .last()
        {
            // TODO: Apply the "magic shift"
            let upem = metrics.units_per_em as f32;
            let ppem = strike.ppem() as f32;

            if data.graphic_type() == Tag::new(b"png ") {
                let mut canvas = Canvas::new(
                    Size::from_wh(metrics.units_per_em as f32, metrics.units_per_em as f32)
                        .unwrap(),
                );
                let dynamic_image = image::load_from_memory(data.data()).ok()?;
                let size_factor = upem / (ppem);
                let width = dynamic_image.width() as f32 * size_factor;
                let height = dynamic_image.height() as f32 * size_factor;
                let size = Size::from_wh(width, height).unwrap();
                canvas.draw_image(
                    Image::new(&dynamic_image),
                    size,
                    Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, height),
                );
                return Some(canvas);
            }
        }
    }

    return None;
}

#[cfg(test)]
mod tests {

    use crate::font::{draw, Font};

    use skrifa::instance::Location;

    use skrifa::raw::TableProvider;

    use crate::font::bitmap::draw_glyph;
    use std::sync::Arc;

    #[test]
    fn sbix_apple_color() {
        let font_data = std::fs::read("/System/Library/Fonts/Apple Color Emoji.ttc").unwrap();
        let font = Font::new(Arc::new(font_data), Location::default()).unwrap();

        let glyphs = (0..=300).collect::<Vec<_>>();

        draw(&font, &glyphs, "sbix_apple_color", draw_glyph);
    }
}
