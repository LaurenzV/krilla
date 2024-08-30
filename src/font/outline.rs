use crate::error::{KrillaError, KrillaResult};
use crate::font::{Font, OutlineBuilder};
use crate::object::color_space::luma::DeviceGray;
use crate::surface::Surface;
use crate::Fill;
use skrifa::outline::DrawSettings;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::Transform;

pub fn draw_glyph(font: Font, glyph: GlyphId, surface: &mut Surface) -> KrillaResult<Option<()>> {
    let outline_glyphs = font.font_ref().outline_glyphs();
    let mut outline_builder = OutlineBuilder::new();

    if let Some(outline_glyph) = outline_glyphs.get(glyph) {
        let drawn = outline_glyph.draw(
            DrawSettings::unhinted(skrifa::instance::Size::unscaled(), font.location_ref()),
            &mut outline_builder,
        );

        if let Err(err) = drawn {
            return Err(KrillaError::GlyphDrawing(format!(
                "failed to draw outline glyph: {}",
                err
            )));
        }
    }

    if let Some(path) = outline_builder.finish() {
        surface.push_transform(&Transform::from_scale(1.0, -1.0));
        surface.fill_path_impl(&path, Fill::<DeviceGray>::default(), true);
        surface.pop();

        return Ok(Some(()));
    }

    Ok(None)
}
