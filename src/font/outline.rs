//! Drawing outline-based glyphs to a surface.

use crate::font::{Font, OutlineBuilder, PaintMode};
use crate::surface::Surface;
use skrifa::outline::DrawSettings;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::{Path, Transform};

pub fn glyph_path(font: Font, glyph: GlyphId) -> Option<Path> {
    let outline_glyphs = font.font_ref().outline_glyphs();
    let mut outline_builder = OutlineBuilder::new();

    if let Some(outline_glyph) = outline_glyphs.get(glyph) {
        outline_glyph
            .draw(
                DrawSettings::unhinted(skrifa::instance::Size::unscaled(), font.location_ref()),
                &mut outline_builder,
            )
            .ok()?;
    }

    outline_builder.finish()
}

/// Draw an outline-based glyph on a surface.
pub fn draw_glyph(
    font: Font,
    glyph: GlyphId,
    paint_mode: PaintMode,
    base_transform: Transform,
    surface: &mut Surface,
) -> Option<()> {
    let path = glyph_path(font, glyph).and_then(|p| p.transform(base_transform))?;

    match paint_mode {
        PaintMode::Fill(f) => surface.fill_path(&path, f.clone()),
        PaintMode::Stroke(s) => {
            surface.stroke_path(&path, s.clone());
        }
    }

    Some(())
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::tests::{all_glyphs_to_pdf, NOTO_SANS};
    use krilla_macros::visreg;
    use skrifa::GlyphId;

    #[visreg(document, settings_4, all)]
    fn noto_sans_type3_glyphs(document: &mut Document) {
        let font_data = NOTO_SANS.clone();
        all_glyphs_to_pdf(
            font_data,
            Some(
                (20..=50)
                    .map(|n| (GlyphId::new(n), "".to_string()))
                    .collect::<Vec<_>>(),
            ),
            true,
            document,
        );
    }
}
