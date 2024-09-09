use crate::error::{KrillaError, KrillaResult};
use crate::font::{Font, OutlineBuilder};
use crate::object::color::luma::DeviceGray;
use crate::path::Fill;
use crate::surface::Surface;
use skrifa::outline::DrawSettings;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::{Path, Transform};

pub fn glyph_path(font: Font, glyph: GlyphId) -> KrillaResult<Option<Path>> {
    let outline_glyphs = font.font_ref().outline_glyphs();
    let mut outline_builder = OutlineBuilder::new();

    if let Some(outline_glyph) = outline_glyphs.get(glyph) {
        outline_glyph
            .draw(
                DrawSettings::unhinted(skrifa::instance::Size::unscaled(), font.location_ref()),
                &mut outline_builder,
            )
            .map_err(|err| {
                KrillaError::GlyphDrawing(format!("failed to draw outline glyph: {}", err))
            })?;
    }

    Ok(outline_builder
        .finish()
        .and_then(|p| p.transform(Transform::from_scale(1.0, -1.0))))
}

/// Draw an outline-based glyph on a surface.
pub fn draw_glyph(font: Font, glyph: GlyphId, surface: &mut Surface) -> KrillaResult<Option<()>> {
    if let Some(path) = glyph_path(font, glyph)? {
        surface.fill_path_impl(&path, Fill::<DeviceGray>::default(), false);

        return Ok(Some(()));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::tests::{all_glyphs_to_pdf, NOTO_SANS};
    use krilla_macros::visreg;
    use skrifa::GlyphId;

    #[visreg(document, settings_4)]
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
