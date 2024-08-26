#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::font::Font;
    use crate::rgb::Rgb;
    use crate::serialize::SerializeSettings;
    use crate::test_utils::{load_font, simple_shape};
    use crate::Fill;
    use rustybuzz::Direction;
    use skrifa::instance::Location;
    use std::sync::Arc;

    #[ignore]
    #[test]
    fn simple_shape_demo() {
        let mut y = 25.0;

        let mut data = vec![
            (
                "NotoSansArabic-Regular.ttf",
                "هذا نص أطول لتجربة القدرات.",
                Direction::RightToLeft,
                14.0,
            ),
            (
                "NotoSans-Regular.ttf",
                "Hi there, this is a very simple test!",
                Direction::LeftToRight,
                14.0,
            ),
            (
                "NotoSansCJKsc-Regular.otf",
                "你好世界，这是一段很长的测试文章",
                Direction::LeftToRight,
                14.0,
            ),
            (
                "NotoSansDevanagari-Regular.ttf",
                "आ रु॒क्मैरा यु॒धा नर॑ ऋ॒ष्वा ऋ॒ष्टीर॑सृक्षत ।",
                Direction::LeftToRight,
                14.0,
            ),
        ];
        let page_size = tiny_skia_path::Size::from_wh(200.0, 200.0).unwrap();
        let mut document_builder = Document::new(SerializeSettings::default_test());
        let mut builder = document_builder.start_page(page_size);
        let mut surface = builder.surface();

        for (font, text, dir, size) in data {
            let font_data = load_font(font);
            let font = Font::new(Arc::new(font_data), 0, Location::default()).unwrap();
            let glyphs = simple_shape(text, dir, font, size);

            surface.draw_glyph_run(
                0.0,
                y,
                Fill::<Rgb>::default(),
                glyphs.into_iter().peekable(),
            );
            y += size * 1.5;
        }

        surface.finish();
        builder.finish();

        let pdf = document_builder.finish();
        let _ = std::fs::write(format!("out/simple_shape_demo.pdf"), &pdf);
        let _ = std::fs::write(format!("out/simple_shape_demo.txt"), &pdf);
    }
}
