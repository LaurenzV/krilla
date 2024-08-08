#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::object::color_space::srgb::Srgb;
    use crate::serialize::{PageSerialize, SerializeSettings};
    use crate::stream::TestGlyph;
    use crate::Fill;
    use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
    use fontdb::Source;
    use skrifa::GlyphId;
    use std::sync::Arc;

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
        let mut document_builder = Document::new(SerializeSettings::default());
        let mut builder = document_builder.add_page(page_size);

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
            builder.fill_glyph_run(
                0.0,
                y_offset,
                font_system.db_mut(),
                &Fill::<Srgb>::default(),
                iter,
            );
        }

        builder.finish_page();

        let pdf = document_builder.finish(font_system.db());
        let _ = std::fs::write(format!("out/cosmic_text.pdf"), &pdf);
        let _ = std::fs::write(format!("out/cosmic_text.txt"), &pdf);
    }
}
