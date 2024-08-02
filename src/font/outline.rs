use crate::canvas::CanvasBuilder;
use crate::font::{Font, OutlineBuilder};
use crate::Fill;
use skrifa::outline::DrawSettings;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::Transform;

pub fn draw_glyph(font: &Font, glyph: GlyphId, canvas_builder: &mut CanvasBuilder) -> Option<()> {
    let font_ref = font.font_ref();
    let outline_glyphs = font_ref.outline_glyphs();
    let mut outline_builder = OutlineBuilder::new();

    if let Some(outline_glyph) = outline_glyphs.get(glyph) {
        let _ = outline_glyph.draw(
            DrawSettings::unhinted(skrifa::instance::Size::unscaled(), font.location_ref()),
            &mut outline_builder,
        );
    }

    if let Some(path) = outline_builder.finish() {
        canvas_builder.push_transform(&Transform::from_scale(1.0, -1.0));
        canvas_builder.fill_path_impl(&path, &Fill::default(), true);
        canvas_builder.pop_transform();

        return Some(());
    }

    None
}

#[cfg(test)]
mod tests {

    use crate::font::{draw, Font};

    use skrifa::instance::Location;

    use skrifa::GlyphId;
    use std::rc::Rc;

    // This will not use Type3
    #[test]
    fn outline_noto_sans() {
        let font_data = std::fs::read("/Library/Fonts/NotoSans-Regular.ttf").unwrap();
        let font = Font::new(Rc::new(font_data), Location::default()).unwrap();

        let glyphs = (0..1000)
            .map(|n| (GlyphId::new(n), "".to_string()))
            .collect::<Vec<_>>();

        draw(&font, Some(glyphs), "outline_noto_sans");
    }
}
