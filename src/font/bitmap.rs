use crate::error::{KrillaError, KrillaResult};
use crate::font::Font;
use crate::object::image::Image;
use crate::surface::Surface;
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider, Tag};
use tiny_skia_path::{Size, Transform};

/// Draw a bitmap-based glyph on a surface.
pub fn draw_glyph(font: Font, glyph: GlyphId, surface: &mut Surface) -> KrillaResult<Option<()>> {
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
                let image = Image::from_png(&data.data()).ok_or(KrillaError::GlyphDrawing(
                    "failed to decode png".to_string(),
                ))?;
                let size_factor = upem / (ppem);
                let size = image.size();
                let width = size.width() * size_factor;
                let height = size.height() * size_factor;
                let size = Size::from_wh(width, height).unwrap();
                surface.push_transform(
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
                surface.draw_image(image, size);
                surface.pop();

                return Ok(Some(()));
            }
        }
    }

    Ok(None)
}
