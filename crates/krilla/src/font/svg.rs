//! Drawing SVG-based glyphs on a surface.

use skrifa::raw::TableProvider;
use skrifa::GlyphId;

use crate::color::rgb;
use crate::font::Font;
use crate::object::font::PaintMode;
use crate::surface::Surface;

/// Draw an SVG-based glyph on a surface.
pub(crate) fn draw_glyph(
    font: Font,
    glyph: GlyphId,
    surface: &mut Surface,
    paint_mode: PaintMode,
) -> Option<()> {
    let svg_data = font
        .font_ref()
        .svg()
        .and_then(|svg_table| svg_table.glyph_data(glyph))
        .ok()??;

    let context_color = match paint_mode {
        PaintMode::Fill(f) => f.paint.as_rgb(),
        PaintMode::Stroke(s) => s.paint.as_rgb(),
    }
    .unwrap_or(rgb::Color::black());

    let fn_ = surface.sc.serialize_settings().render_svg_glyph_fn;
    fn_(svg_data, context_color, glyph, surface)?;

    Some(())
}
