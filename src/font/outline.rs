use crate::font::{Font, OutlineBuilder};
use crate::stream::StreamBuilder;
use crate::Fill;
use skrifa::outline::DrawSettings;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::{Size, Transform};

pub fn draw_glyph(font: &Font, glyph: GlyphId, stream_builder: &mut StreamBuilder) {
    let font_ref = font.font_ref();
    let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), font.location_ref());
    let outline_glyphs = font_ref.outline_glyphs();
    let mut outline_builder = OutlineBuilder::new();

    if let Some(outline_glyph) = outline_glyphs.get(glyph) {
        let _ = outline_glyph.draw(
            DrawSettings::unhinted(skrifa::instance::Size::unscaled(), font.location_ref()),
            &mut outline_builder,
        );
    }

    if let Some(path) = outline_builder.finish() {
        stream_builder.save_graphics_state();
        stream_builder.concat_transform(&Transform::from_scale(1.0, -1.0));
        stream_builder.draw_fill_path(&path, &Fill::default());
        stream_builder.restore_graphics_state();
    }
}

#[cfg(test)]
mod tests {

    use crate::font::outline::draw_glyph;
    use crate::font::{draw, Font};

    use skrifa::instance::Location;

    use skrifa::raw::TableProvider;

    use std::sync::Arc;

    // This will not use Type3
    #[test]
    fn outline_noto_sans() {
        let font_data = std::fs::read("/Library/Fonts/NotoSans-Regular.ttf").unwrap();
        let font = Font::new(Arc::new(font_data), Location::default()).unwrap();

        let glyphs = (0..1000).collect::<Vec<_>>();

        draw(&font, &glyphs, "outline_noto_sans", draw_glyph);
    }
}
