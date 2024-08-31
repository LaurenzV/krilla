use crate::error::{KrillaError, KrillaResult};
use crate::font::Font;
use crate::serialize::SvgSettings;
use crate::surface::Surface;
use crate::svg;
use skrifa::raw::TableProvider;
use skrifa::GlyphId;
use std::io::Read;
use usvg::roxmltree;

/// Draw an SVG-based glyph on a surface.
pub fn draw_glyph(
    font: Font,
    glyph: GlyphId,
    surface: &mut Surface,
    svg_settings: SvgSettings,
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
                KrillaError::GlyphDrawing("failed to parse svg for glyph".to_string())
            })?;
            data = &decoded;
        }

        let xml = std::str::from_utf8(data)
            .map_err(|_| KrillaError::GlyphDrawing("failed to parse svg for glyph".to_string()))?;
        let document = roxmltree::Document::parse(xml)
            .map_err(|_| KrillaError::GlyphDrawing("failed to parse svg for glyph".to_string()))?;

        // TODO: Add cache for SVG glyphs
        let opts = usvg::Options::default();
        let tree = usvg::Tree::from_xmltree(&document, &opts).map_err(|_| {
            KrillaError::GlyphDrawing("failed to convert SVG for glyph".to_string())
        })?;
        if let Some(node) = tree.node_by_id(&format!("glyph{}", glyph.to_u32())) {
            svg::render_node(&node, tree.fontdb().clone(), svg_settings, surface)
        } else {
            // Twitter Color Emoji SVGs contain the glyph ID on the root element, which isn't saved by
            // usvg. So in this case, we simply draw the whole document.
            svg::render_tree(&tree, svg_settings, surface)
        };

        return Ok(Some(()));
    };

    Ok(None)
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::tests::{all_glyphs_to_pdf, TWITTER_COLOR_EMOJI};
    use krilla_macros::visreg;

    #[visreg(document)]
    fn twitter_color_emoji(document: &mut Document) {
        let font_data = TWITTER_COLOR_EMOJI.clone();
        all_glyphs_to_pdf(font_data, None, false, document);
    }
}
