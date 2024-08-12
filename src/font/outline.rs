use crate::font::{Font, OutlineBuilder};
use crate::object::color_space::luma::DeviceGray;
use crate::surface::Surface;
use crate::Fill;
use skrifa::outline::DrawSettings;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::Transform;

pub fn draw_glyph(font: &Font, glyph: GlyphId, surface: &mut Surface) -> Option<()> {
    let outline_glyphs = font.font_ref().outline_glyphs();
    let mut outline_builder = OutlineBuilder::new();

    if let Some(outline_glyph) = outline_glyphs.get(glyph) {
        let _ = outline_glyph.draw(
            DrawSettings::unhinted(skrifa::instance::Size::unscaled(), font.location_ref()),
            &mut outline_builder,
        );
    }

    if let Some(path) = outline_builder.finish() {
        surface.push_transform(&Transform::from_scale(1.0, -1.0));
        surface.fill_path_impl(&path, Fill::<DeviceGray>::default(), true);
        surface.pop();

        return Some(());
    }

    None
}

#[cfg(test)]
mod tests {

    use crate::font::draw;

    use skrifa::GlyphId;

    use std::sync::Arc;

    // This will not use Type3
    #[test]
    fn outline_noto_sans() {
        let font_data = std::fs::read("/Library/Fonts/NotoSans-Regular.ttf").unwrap();

        let glyphs = (0..1000)
            .map(|n| (GlyphId::new(n), "".to_string()))
            .collect::<Vec<_>>();

        draw(Arc::new(font_data), Some(glyphs), "outline_noto_sans");
    }
}
