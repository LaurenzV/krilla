use crate::error::{KrillaError, KrillaResult};
use crate::font::Font;
use crate::serialize::SvgSettings;
use crate::surface::Surface;
use crate::svg;
use skrifa::raw::TableProvider;
use skrifa::GlyphId;
use std::io::Read;
use usvg::roxmltree;

pub fn draw_glyph(
    font: Font,
    svg_settings: SvgSettings,
    glyph: GlyphId,
    builder: &mut Surface,
) -> KrillaResult<Option<()>> {
    if let Ok(Some(svg_data)) = font
        .font_ref()
        .svg()
        .and_then(|svg_table| svg_table.glyph_data(glyph))
    {
        let mut data = svg_data;

        let mut decoded = vec![];
        if data.starts_with(&[0x1f, 0x8b]) {
            let mut decoder = flate2::read::GzDecoder::new(data);
            decoder.read_to_end(&mut decoded).map_err(|_| {
                KrillaError::GlyphDrawing("failed to parse SVG for glyph".to_string())
            })?;
            data = &decoded;
        }

        let xml = std::str::from_utf8(data)
            .map_err(|_| KrillaError::GlyphDrawing("failed to parse SVG for glyph".to_string()))?;
        let document = roxmltree::Document::parse(xml)
            .map_err(|_| KrillaError::GlyphDrawing("failed to parse SVG for glyph".to_string()))?;

        // TODO: Add cache for SVG glyphs
        let opts = usvg::Options::default();
        let tree = usvg::Tree::from_xmltree(&document, &opts).map_err(|_| {
            KrillaError::GlyphDrawing("failed to convert SVG for glyph".to_string())
        })?;
        if let Some(node) = tree.node_by_id(&format!("glyph{}", glyph.to_u32())) {
            svg::render_node(&node, tree.fontdb().clone(), svg_settings, builder)
        } else {
            // Twitter Color Emoji SVGs contain the glyph ID on the root element, which isn't saved by
            // usvg. So in this case, we simply draw the whole document.
            svg::render_tree(&tree, svg_settings, builder)
        };

        return Ok(Some(()));
    };

    Ok(None)
}
