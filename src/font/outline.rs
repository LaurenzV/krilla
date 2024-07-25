#[cfg(test)]
mod tests {
    use crate::canvas::{Canvas, Surface};
    use crate::font::OutlineBuilder;
    use crate::serialize::{PageSerialize, SerializeSettings};
    use crate::Fill;
    use skrifa::outline::DrawSettings;
    use skrifa::prelude::LocationRef;
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
    fn try_it() {
        let font_data = std::fs::read("/Library/Fonts/NotoSans-Regular.ttf").unwrap();
        let font_ref = FontRef::from_index(&font_data, 0).unwrap();
        let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), LocationRef::default());

        let glyphs = (10u16..=300).collect::<Vec<_>>();

        let num_glyphs = glyphs.len();

        let width = 2000;

        let size = 150u32;
        let num_cols = width / size;
        let height = (num_glyphs as f32 / num_cols as f32).ceil() as u32 * size;
        let units_per_em = metrics.units_per_em as f32;
        let mut cur_point = 0;

        let mut parent_canvas = Canvas::new(Size::from_wh(width as f32, height as f32).unwrap());

        for i in glyphs.iter().copied() {
            let canvas = single_glyph(&font_ref, GlyphId::new(i));

            fn get_transform(
                cur_point: u32,
                size: u32,
                num_cols: u32,
                units_per_em: f32,
            ) -> crate::Transform {
                let el = cur_point / size;
                let col = el % num_cols;
                let row = el / num_cols;

                crate::Transform::from_row(
                    (1.0 / units_per_em) * size as f32,
                    0.0,
                    0.0,
                    (1.0 / units_per_em) * size as f32,
                    col as f32 * size as f32,
                    row as f32 * size as f32,
                )
            }

            let mut transformed = parent_canvas.transformed(
                get_transform(cur_point, size, num_cols, units_per_em).pre_concat(
                    tiny_skia_path::Transform::from_row(
                        1.0,
                        0.0,
                        0.0,
                        -1.0,
                        0.0,
                        units_per_em as f32,
                    ),
                ),
            );
            transformed.draw_canvas(canvas);
            transformed.finish();

            cur_point += size;
        }

        let pdf = parent_canvas.serialize(SerializeSettings::default());
        let _ = std::fs::write("out/outline.pdf", pdf.finish());
    }
}
