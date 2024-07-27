#[cfg(test)]
mod tests {
    use crate::canvas::{Canvas, Surface};
    use crate::font::{draw, OutlineBuilder};
    use crate::serialize::{PageSerialize, SerializeSettings};
    use crate::{svg, Fill};
    use skrifa::outline::DrawSettings;
    use skrifa::prelude::LocationRef;
    use skrifa::raw::TableProvider;
    use skrifa::{FontRef, GlyphId, MetadataProvider};
    use std::io::Read;
    use tiny_skia_path::{Size, Transform};
    use usvg::roxmltree;

    fn single_glyph(font_ref: &FontRef, glyph: GlyphId) -> Canvas {
        let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), LocationRef::default());
        let svg_table = font_ref.svg().unwrap();

        if let Ok(Some(svg_data)) = svg_table.glyph_data(glyph) {
            let mut data = svg_data.get();

            let mut decoded = vec![];
            if data.starts_with(&[0x1f, 0x8b]) {
                let mut decoder = flate2::read::GzDecoder::new(data);
                decoder.read_to_end(&mut decoded).unwrap();
                data = &decoded;
            }

            // Parse XML.
            let xml = std::str::from_utf8(data).ok().unwrap();
            let document = roxmltree::Document::parse(xml).ok().unwrap();

            // Parse SVG.
            let opts = usvg::Options::default();
            let tree = usvg::Tree::from_xmltree(&document, &opts).unwrap();
            // TODO: Should only draw the corresponding node to the glyph
            let svg_canvas = svg::render(&tree);

            let mut canvas = Canvas::new(svg_canvas.size);
            let mut transformed = canvas.transformed(
                Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, metrics.units_per_em as f32)
                    .pre_concat(Transform::from_translate(
                        0.0,
                        (metrics.units_per_em as f32),
                    )),
            );
            transformed.draw_canvas(svg_canvas);
            transformed.finish();
            canvas
        } else {
            Canvas::new(
                Size::from_wh(metrics.units_per_em as f32, metrics.units_per_em as f32).unwrap(),
            )
        }
    }

    #[test]
    fn svg_twitter() {
        let font_data = std::fs::read("/Library/Fonts/TwitterColorEmoji-SVGinOT.ttf").unwrap();
        let font_ref = FontRef::from_index(&font_data, 0).unwrap();

        let glyphs = (0..font_ref.maxp().unwrap().num_glyphs() as u32).collect::<Vec<_>>();

        draw(&font_ref, &glyphs, "svg_twitter", single_glyph);
    }
}
