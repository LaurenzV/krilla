//! Drawing outline-based glyphs to a surface.

use crate::font::Font;
use crate::object::font::PaintMode;
use crate::surface::Surface;
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::{Path, PathBuilder, Transform};

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

/// A wrapper struct for implementing the `OutlinePen` trait.
pub struct OutlineBuilder(PathBuilder);

impl OutlineBuilder {
    pub fn new() -> Self {
        Self(PathBuilder::new())
    }

    pub fn finish(self) -> Option<Path> {
        self.0.finish()
    }
}

impl OutlinePen for OutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.0.close()
    }
}
