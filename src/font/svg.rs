use crate::canvas::CanvasBuilder;
use crate::font::Font;
use crate::svg;
use skrifa::raw::TableProvider;
use skrifa::GlyphId;
use std::io::Read;
use usvg::roxmltree;

pub fn draw_glyph(font: &Font, glyph: GlyphId, builder: &mut CanvasBuilder) -> Option<()> {
    let font_ref = font.font_ref();

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
        if let Some(node) = tree.node_by_id(&format!("glyph{}", glyph.to_u32())) {
            svg::render_node(&node, tree.fontdb().clone(), builder)
        } else {
            // Twitter Color Emoji SVGs contain the glyph ID on the root element, which isn't saved by
            // usvg. So in this case, we simply draw the whole document.
            svg::render_tree(&tree, builder)
        };

        return Some(());
    };

    None
}

#[cfg(test)]
mod tests {
    use crate::font::{draw, Font};
    use skrifa::instance::Location;

    use std::sync::Arc;

    #[test]
    fn svg_twitter() {
        let font_data = std::fs::read("/Library/Fonts/TwitterColorEmoji-SVGinOT.ttf").unwrap();
        let font = Font::new(Arc::new(font_data), Location::default()).unwrap();

        draw(&font, None, "svg_twitter");
    }
}
