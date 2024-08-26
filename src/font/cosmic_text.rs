#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::object::color_space::rgb::Rgb;
    use crate::serialize::SerializeSettings;
    use crate::stream::{Cluster, Glyph};

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
        let mut builder = document_builder.start_page(page_size);
        let mut surface = builder.surface();

        let font_map = surface.convert_fontdb(font_system.db_mut(), None);

        // Inspect the output runs
        for run in buffer.layout_runs() {
            let y_offset = run.line_y;
            let iter = run
                .glyphs
                .iter()
                .map(|g| {
                    Cluster::new(
                        &run.text[g.start..g.end],
                        Glyph::new(
                            font_map.get(&g.font_id).unwrap().clone(),
                            GlyphId::new(g.glyph_id as u32),
                            g.w,
                            g.x_offset,
                            g.font_size,
                        )
                    )
                })
                .peekable();
            surface.draw_glyph_run(0.0, y_offset, Fill::<Rgb>::default(), iter);
        }

        surface.finish();
        builder.finish();

        let pdf = document_builder.finish();
        let _ = std::fs::write(format!("out/cosmic_text.pdf"), &pdf);
        let _ = std::fs::write(format!("out/cosmic_text.txt"), &pdf);
    }
}
