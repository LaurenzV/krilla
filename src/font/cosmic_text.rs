#[cfg(test)]
mod tests {
    use crate::canvas::Page;
    use crate::font::Glyph;
    use crate::serialize::PageSerialize;
    use crate::Fill;
    use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
    use fontdb::Source;
    use skrifa::GlyphId;
    use std::sync::Arc;
    use tiny_skia_path::{FiniteF32, Transform};

    #[test]
    fn cosmic_text_integration() {
        let mut font_system = FontSystem::new_with_fonts([Source::Binary(Arc::new(std::fs::read("/Users/lstampfl/Programming/GitHub/resvg/crates/resvg/tests/fonts/NotoSans-Regular.ttf").unwrap()))]);
        let metrics = Metrics::new(14.0, 20.0);
        let mut buffer = Buffer::new(&mut font_system, metrics);
        buffer.set_size(&mut font_system, Some(200.0), None);
        let attrs = Attrs::new();
        let text = "Some text here. Let's make it a bit longer so that line wrapping kicks in 😊.\n我也要使用一些中文文字。 And also some اللغة العربية arabic text.\n\nहो। गए, उनका एक समय में\n\n\nz͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔̦̳͆̏̓͢";
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
                let x_offset = glyph.x_offset * glyph.font_size + glyph.x;
                let y_offset = y_offset + glyph.y_offset * glyph.font_size;
                builder.fill_glyph(
                    Glyph::new(GlyphId::new(glyph.glyph_id as u32), text.to_string()),
                    glyph.font_id,
                    font_system.db_mut(),
                    FiniteF32::new(glyph.font_size).unwrap(),
                    &Transform::from_translate(x_offset, y_offset),
                    &Fill::default(),
                )
            }
        }

        // panic!();

        let stream = builder.finish();
        let sc = page.finish();

        let pdf = stream.serialize(sc, font_system.db(), page_size);
        let finished = pdf.finish();
        let _ = std::fs::write(format!("out/cosmic_text.pdf"), &finished);
        let _ = std::fs::write(format!("out/cosmic_text.txt"), &finished);
    }
}
