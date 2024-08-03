#[cfg(test)]
mod tests {
    use crate::canvas::Page;
    use crate::font::{Font, Glyph};
    use crate::serialize::PageSerialize;
    use crate::Fill;
    use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
    use skrifa::instance::Location;
    use skrifa::GlyphId;
    use std::sync::Arc;
    use tiny_skia_path::{FiniteF32, Transform};

    #[test]
    fn cosmic_text_integration() {
        let mut font_system = FontSystem::new();
        let metrics = Metrics::new(14.0, 20.0);
        let mut buffer = Buffer::new(&mut font_system, metrics);
        buffer.set_size(&mut font_system, Some(200.0), None);
        let attrs = Attrs::new();
        let text = "Some text here. Let's make it a bit longer so that line wrapping kicks in 😊.\n我也要使用一些中文文字。 And also some اللغة العربية arabic text.\n\n";
        buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut font_system, false);

        let page_size = tiny_skia_path::Size::from_wh(200.0, 400.0).unwrap();
        let mut page = Page::new(page_size);
        let mut builder = page.builder();

        // Inspect the output runs
        for run in buffer.layout_runs() {
            let y_offset = run.line_y;
            for glyph in run.glyphs.iter() {
                let text = &run.text[glyph.start..glyph.end];
                let x_offset = glyph.x_offset + glyph.x;
                let y_offset = y_offset + glyph.y_offset;
                let font = Font::new(
                    Arc::new(font_system.get_font(glyph.font_id).unwrap().data().to_vec()),
                    Location::default(),
                )
                .unwrap();
                builder.fill_glyph(
                    Glyph::new(GlyphId::new(glyph.glyph_id as u32), text.to_string()),
                    font,
                    FiniteF32::new(glyph.font_size).unwrap(),
                    &Transform::from_translate(x_offset, y_offset),
                    &Fill::default(),
                )
            }
        }

        let stream = builder.finish();
        let sc = page.finish();

        let pdf = stream.serialize(sc, page_size);
        let finished = pdf.finish();
        let _ = std::fs::write(format!("out/parley.pdf"), &finished);
        let _ = std::fs::write(format!("out/parley.txt"), &finished);
    }
}
