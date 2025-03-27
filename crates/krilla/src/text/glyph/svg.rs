use skrifa::raw::TableProvider;

use crate::surface::Surface;
use crate::text::GlyphId;
use crate::text::{Font, PaintMode};

/// Draw an SVG-based glyph on a surface.
pub(crate) fn draw_glyph(
    font: Font,
    paint_mode: PaintMode,
    glyph: GlyphId,
    surface: &mut Surface,
) -> Option<()> {
    let svg_data = font
        .font_ref()
        .svg()
        .and_then(|svg_table| svg_table.glyph_data(glyph.to_skrifa()))
        .ok()??;

    let context_color = match paint_mode {
        PaintMode::Fill(f) => f.paint.as_rgb(),
        PaintMode::Stroke(s) => s.paint.as_rgb(),
        PaintMode::FillStroke(f, _) => f.paint.as_rgb(),
    }
    .unwrap_or_default();

    let fn_ = surface.sc.serialize_settings().render_svg_glyph_fn;
    fn_(svg_data, context_color, glyph, surface)?;

    Some(())
}
