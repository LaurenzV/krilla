use crate::canvas::{Canvas, Surface};
use crate::font::Font;
use crate::svg;
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider};
use std::io::Read;
use tiny_skia_path::Transform;
use usvg::roxmltree;

pub fn draw_glyph(font: &Font, glyph: GlyphId) -> Option<Canvas> {
    let font_ref = font.font_ref();
    let location_ref = font.location_ref();

    let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), location_ref);

    if let Ok(Some(svg_data)) = font_ref
        .svg()
        .and_then(|svg_table| svg_table.glyph_data(glyph))
    {
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
        let svg_canvas = if let Some(node) = tree.node_by_id(&format!("glyph{}", glyph.to_u32())) {
            svg::render_node(&node)
        } else {
            // Twitter Color Emoji SVGs contain the glyph ID on the root element, which isn't saved by
            // usvg. So in this case, we simply draw the whole document.
            svg::render_tree(&tree)
        };

        return Some(svg_canvas);
    };

    return None;
}

#[cfg(test)]
mod tests {
    use crate::font::svg::draw_glyph;
    use crate::font::{draw, Font};
    use skrifa::instance::Location;
    use std::sync::Arc;

    #[test]
    fn svg_twitter() {
        let font_data = std::fs::read("/Library/Fonts/TwitterColorEmoji-SVGinOT.ttf").unwrap();
        let font = Font::new(Arc::new(font_data), Location::default()).unwrap();

        let glyphs = (2000..2500).collect::<Vec<_>>();
        // let glyphs = (0..font_ref.maxp().unwrap().num_glyphs() as u32).collect::<Vec<_>>();

        draw(&font, &glyphs, "svg_twitter", draw_glyph);
    }
}
