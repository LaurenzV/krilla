#[cfg(test)]
mod tests {
    use crate::canvas::{Canvas, Surface};
    use crate::font::{draw, OutlineBuilder};
    use crate::serialize::{PageSerialize, SerializeSettings};
    use crate::Fill;
    use skrifa::outline::DrawSettings;
    use skrifa::prelude::LocationRef;
    use skrifa::raw::TableProvider;
    use skrifa::{FontRef, GlyphId, MetadataProvider};
    use tiny_skia_path::{Size, Transform};

    fn single_glyph(font_ref: &FontRef, glyph: GlyphId) -> Canvas {
        let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), LocationRef::default());
        let outline_glyphs = font_ref.outline_glyphs();
        let mut outline_builder = OutlineBuilder::new();

        if let Some(outline_glyph) = outline_glyphs.get(glyph) {
            let _ = outline_glyph.draw(
                DrawSettings::unhinted(skrifa::instance::Size::unscaled(), LocationRef::default()),
                &mut outline_builder,
            );
        }

        let mut canvas = Canvas::new(
            Size::from_wh(metrics.units_per_em as f32, metrics.units_per_em as f32).unwrap(),
        );

        if let Some(path) = outline_builder.finish() {
            canvas.fill_path(path, Transform::identity(), Fill::default());
        }

        canvas
    }

    #[test]
    fn outline_noto_sans() {
        let font_data = std::fs::read("/Library/Fonts/NotoSans-Regular.ttf").unwrap();
        let font_ref = FontRef::from_index(&font_data, 0).unwrap();

        let glyphs = (0..font_ref.maxp().unwrap().num_glyphs() as u32).collect::<Vec<_>>();

        draw(&font_ref, &glyphs, "outline_noto_sans", single_glyph);
    }
}
