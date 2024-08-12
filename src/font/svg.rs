use crate::font::Font;
use crate::serialize::SvgSettings;
use crate::surface::Surface;
use crate::svg;
use fontdb::Database;
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId};
use std::io::Read;
use usvg::roxmltree;

pub fn draw_glyph(
    font: Font,
    svg_settings: SvgSettings,
    glyph: GlyphId,
    fontdb: &mut Database,
    builder: &mut Surface,
) -> Option<()> {
    if let Ok(Some(svg_data)) = font
        .font_ref()
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
            svg::render_node(&node, tree.fontdb().clone(), svg_settings, builder, fontdb)
        } else {
            // Twitter Color Emoji SVGs contain the glyph ID on the root element, which isn't saved by
            // usvg. So in this case, we simply draw the whole document.
            svg::render_tree(&tree, svg_settings, builder, fontdb)
        };

        return Some(());
    };

    None
}

#[cfg(test)]
mod tests {
    use crate::font::draw;
    use std::sync::Arc;

    #[test]
    fn svg_twitter() {
        let font_data = std::fs::read("/Library/Fonts/TwitterColorEmoji-SVGinOT.ttf").unwrap();
        draw(Arc::new(font_data), None, "svg_twitter");
    }
}
