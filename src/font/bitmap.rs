use crate::canvas::{CanvasBuilder, Surface};
use crate::font::Font;
use crate::object::image::Image;
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider, Tag};
use tiny_skia_path::{Size, Transform};

pub fn draw_glyph(font: &Font, glyph: GlyphId, canvas_builder: &mut CanvasBuilder) -> Option<()> {
    let metrics = font
        .font_ref()
        .metrics(skrifa::instance::Size::unscaled(), font.location_ref());

    if let Ok(table) = font.font_ref().sbix() {
        if let Some((strike, data)) = table
            .strikes()
            .iter()
            .map(|s| s.ok())
            .filter_map(|s| Some((s.clone()?, s?.glyph_data(glyph).ok()??)))
            .last()
        {
            let upem = metrics.units_per_em as f32;
            let ppem = strike.ppem() as f32;

            if data.graphic_type() == Tag::new(b"png ") {
                let dynamic_image = image::load_from_memory(data.data()).ok().unwrap();
                let size_factor = upem / (ppem);
                let width = dynamic_image.width() as f32 * size_factor;
                let height = dynamic_image.height() as f32 * size_factor;
                let size = Size::from_wh(width, height).unwrap();
                canvas_builder.push_transform(
                    &Transform::from_translate(0.0, -height)
                        // For unknown reasons, using Apple Color Emoji will lead to a vertical shift on MacOS, but this shift
                        // doesn't seem to be coming from the font and most likely is somehow hardcoded. On Windows,
                        // this shift will not be applied. However, if this shift is not applied the emojis are a bit
                        // too high up when being together with other text, so we try to imitate this.
                        // See also https://github.com/harfbuzz/harfbuzz/issues/2679#issuecomment-1345595425
                        // We approximate this vertical shift that seems to be produced by it.
                        // This value seems to be pretty close to what is happening on MacOS.
                        .pre_concat(Transform::from_translate(0.0, 0.128 * upem)),
                );
                canvas_builder.draw_image(Image::new(&dynamic_image), size);
                canvas_builder.pop_transform();

                return Some(());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {

    use crate::font::draw;

    use std::sync::Arc;

    #[test]
    fn sbix_apple_color() {
        let font_data = std::fs::read("/System/Library/Fonts/Apple Color Emoji.ttc").unwrap();

        draw(Arc::new(font_data), None, "sbix_apple_color");
    }
}
