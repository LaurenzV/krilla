//! Drawing SVG-based glyphs on a surface.

use crate::color::rgb;
use crate::font::{Font, PaintMode};
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
    paint_mode: PaintMode,
    svg_settings: SvgSettings,
) -> Option<()> {
    let svg_data = font
        .font_ref()
        .svg()
        .and_then(|svg_table| svg_table.glyph_data(glyph))
        .ok()??;

    let mut data = svg_data;

    let mut decoded = vec![];
    if data.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = flate2::read::GzDecoder::new(data);
        decoder.read_to_end(&mut decoded).ok()?;
        data = &decoded;
    }

    // TODO: Support CMYK?
    let context_color = match paint_mode {
        PaintMode::Fill(f) => f.paint.as_rgb(),
        PaintMode::Stroke(s) => s.paint.as_rgb(),
    }
    .unwrap_or(rgb::Color::black());

    let xml = std::str::from_utf8(data).ok()?;
    let document = roxmltree::Document::parse(xml).ok()?;

    // Reparsing every time might be pretty slow in some cases, because Noto Color Emoji
    // for example contains hundreds of glyphs in the same SVG document, meaning that we have
    // to reparse it every time. However, Twitter Color Emoji does have each glyph in a
    // separate SVG document, and since we use COLRv1 for Noto Color Emoji anyway, this is
    // good enough.
    let opts = usvg::Options {
        style_sheet: Some(format!(
            "svg {{ color: rgb({}, {}, {}) }}",
            context_color.0, context_color.1, context_color.2
        )),
        ..Default::default()
    };
    let tree = usvg::Tree::from_xmltree(&document, &opts).ok()?;

    if let Some(node) = tree.node_by_id(&format!("glyph{}", glyph.to_u32())) {
        svg::render_node(node, tree.fontdb().clone(), svg_settings, surface)
    } else {
        // Twitter Color Emoji SVGs contain the glyph ID on the root element, which isn't saved by
        // usvg. So in this case, we simply draw the whole document.
        svg::render_tree(&tree, svg_settings, surface)
    };

    Some(())
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::font::Font;
    use crate::surface::{Surface, TextDirection};
    use crate::tests::{all_glyphs_to_pdf, purple_fill, red_fill, SVG_EXTRA, TWITTER_COLOR_EMOJI};
    use krilla_macros::visreg;
    use tiny_skia_path::Point;

    #[visreg(document, all)]
    fn twitter_color_emoji(document: &mut Document) {
        let font_data = TWITTER_COLOR_EMOJI.clone();
        all_glyphs_to_pdf(font_data, None, false, document);
    }

    #[visreg]
    fn svg_extra(surface: &mut Surface) {
        let font_data = SVG_EXTRA.clone();
        let font = Font::new(font_data, 0, vec![]).unwrap();

        surface.fill_text(
            Point::from_xy(0., 30.0),
            purple_fill(1.0),
            font.clone(),
            30.0,
            &[],
            "😀",
            false,
            TextDirection::Auto,
        );

        surface.fill_text(
            Point::from_xy(30., 30.0),
            red_fill(1.0),
            font.clone(),
            30.0,
            &[],
            "😀",
            false,
            TextDirection::Auto,
        );
    }
}
