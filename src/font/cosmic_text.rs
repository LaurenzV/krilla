#[cfg(test)]
mod tests {
    use crate::canvas::Page;
    use crate::font::Glyph;
    use crate::serialize::PageSerialize;
    use crate::stream::TestGlyph;
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
        let text = "Some text here. Let's make it a bit longer so that line wrapping kicks in 😊";
        buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut font_system, false);

        let page_size = tiny_skia_path::Size::from_wh(200.0, 400.0).unwrap();
        let mut page = Page::new(page_size);
        let mut builder = page.builder();

        // Inspect the output runs
        for run in buffer.layout_runs() {
            let y_offset = run.line_y;
            let iter = run
                .glyphs
                .iter()
                .map(|g| {
                    eprintln!("{:?}", g);
                    TestGlyph::new(
                        g.font_id,
                        GlyphId::new(g.glyph_id as u32),
                        g.w,
                        g.x_offset,
                        g.font_size,
                        run.text[g.start..g.end].to_string(),
                    )
                })
                .peekable();
            builder.encode_glyph_run(0.0, y_offset, font_system.db_mut(), iter);
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
