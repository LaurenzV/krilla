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

    fn single_glyph(
        font_ref: &FontRef,
        location_ref: LocationRef,
        glyph: GlyphId,
    ) -> Option<Canvas> {
        let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), location_ref);
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

            // TODO: Add cache for SVG glyphs
            // Parse SVG.
            let opts = usvg::Options::default();
            let tree = usvg::Tree::from_xmltree(&document, &opts).unwrap();
            let svg_canvas =
                if let Some(node) = tree.node_by_id(&format!("glyph{}", glyph.to_u32())) {
                    svg::render_node(&node)
                } else {
                    // Twitter Color Emoji SVGs contain the glyph ID on the root element, which isn't saved by
                    // usvg. So in this case, we simply draw the whole document.
                    svg::render_tree(&tree)
                };

            let mut canvas = Canvas::new(svg_canvas.size);
            let mut transformed = canvas.transformed(
                Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, metrics.units_per_em as f32)
                    .pre_concat(Transform::from_translate(0.0, metrics.units_per_em as f32)),
            );
            transformed.draw_canvas(svg_canvas);
            transformed.finish();
            Some(canvas)
        } else {
            return None;
        }
    }

    #[test]
    fn svg_twitter() {
        let font_data = std::fs::read("/Library/Fonts/TwitterColorEmoji-SVGinOT.ttf").unwrap();
        let font_ref = FontRef::from_index(&font_data, 0).unwrap();

        let glyphs = (2000..2500).collect::<Vec<_>>();
        // let glyphs = (0..font_ref.maxp().unwrap().num_glyphs() as u32).collect::<Vec<_>>();

        draw(
            &font_ref,
            LocationRef::default(),
            &glyphs,
            "svg_twitter",
            single_glyph,
        );
    }

    #[test]
    fn svg_noto() {
        let font_data = std::fs::read("/Library/Fonts/NotoColorEmoji-Regular.ttf").unwrap();
        let font_ref = FontRef::from_index(&font_data, 0).unwrap();

        let glyphs = vec![
            2928, 2880, 2956, 2962, 3168, 3030, 3102, 3036, 2968, 2974, 2855, 2856, 2854, 2922,
            2862, 3150, 3126, 2891, 2892, 3156, 2868, 2997, 2998, 2210, 2937, 2898, 3132, 3060,
            3114, 3120, 2986, 3108, 3024, 3144, 3143, 2943, 3174, 2904, 3066, 2910, 2624, 2625,
        ];

        draw(
            &font_ref,
            LocationRef::default(),
            &glyphs,
            "svg_noto",
            single_glyph,
        );
    }
}
