use crate::font::Font;
use crate::object::image::Image;
use crate::stream::StreamBuilder;
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider, Tag};
use tiny_skia_path::{Size, Transform};

pub fn draw_glyph(font: &Font, glyph: GlyphId, stream_builder: &mut StreamBuilder) -> Option<()> {
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
                let dynamic_image = image::load_from_memory(data.data()).ok().unwrap();
                let size_factor = upem / (ppem);
                let width = dynamic_image.width() as f32 * size_factor;
                let height = dynamic_image.height() as f32 * size_factor;
                let size = Size::from_wh(width, height).unwrap();
                stream_builder.save_graphics_state();
                stream_builder.concat_transform(&Transform::from_translate(0.0, -height));
                stream_builder.draw_image(Image::new(&dynamic_image), size);
                stream_builder.restore_graphics_state();

                return Some(());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {

    use crate::font::{draw, Font};

    use skrifa::instance::Location;

    use crate::font::bitmap::draw_glyph;
    use std::rc::Rc;

    #[test]
    fn sbix_apple_color() {
        let font_data = std::fs::read("/System/Library/Fonts/Apple Color Emoji.ttc").unwrap();
        let font = Font::new(Rc::new(font_data), Location::default()).unwrap();

        let glyphs = (90..=300).collect::<Vec<_>>();

        draw(&font, &glyphs, "sbix_apple_color", draw_glyph);
    }
}
