use crate::canvas::{Canvas, Surface};
use crate::font::{Font, OutlineBuilder};
use crate::Fill;
use skrifa::outline::DrawSettings;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::{Size, Transform};

pub fn draw_glyph(font: &Font, glyph: GlyphId) -> Option<Canvas> {
    let font_ref = font.font_ref();
    let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), font.location_ref());
    let outline_glyphs = font_ref.outline_glyphs();
    let mut outline_builder = OutlineBuilder::new();

    if let Some(outline_glyph) = outline_glyphs.get(glyph) {
        let _ = outline_glyph.draw(
            DrawSettings::unhinted(skrifa::instance::Size::unscaled(), font.location_ref()),
            &mut outline_builder,
        );
    } else {
        return None;
    }

    let mut canvas = Canvas::new(
        Size::from_wh(metrics.units_per_em as f32, metrics.units_per_em as f32).unwrap(),
    );

    if let Some(path) = outline_builder.finish() {
        canvas.fill_path(path, Transform::from_scale(1.0, -1.0), Fill::default());
    }

    Some(canvas)
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
        let font_data =
            std::fs::read("/Users/lstampfl/Programming/GitHub/krilla/NotoSans.ttf").unwrap();
        let font = Font::new(Arc::new(font_data), Location::default()).unwrap();

        let glyphs = (36..100).collect::<Vec<_>>();

        draw(&font, &glyphs, "outline_noto_sans", draw_glyph);
    }
}
