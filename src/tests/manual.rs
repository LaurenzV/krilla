use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use fontdb::Source;

use crate::document::Document;
use crate::font::Font;
use crate::rgb::Rgb;
use crate::serialize::SerializeSettings;
use crate::stream::Glyph;
use crate::tests::{DEJAVU_SANS_MONO, NOTO_SANS, NOTO_SANS_ARABIC, NOTO_SANS_CJK, NOTO_SANS_DEVANAGARI, simple_shape, store_manual};
use crate::util::SliceExt;
use crate::Fill;
use rustybuzz::Direction;
use skrifa::instance::Location;
use skrifa::GlyphId;
use std::sync::Arc;

#[test]
fn simple_shape_demo() {
    let mut y = 25.0;

    let data = vec![
        (
            NOTO_SANS_ARABIC,
            "هذا نص أطول لتجربة القدرات.",
            Direction::RightToLeft,
            14.0,
        ),
        (
            NOTO_SANS,
            "Hi there, this is a very simple test!",
            Direction::LeftToRight,
            14.0,
        ),
        (
            DEJAVU_SANS_MONO,
            "Here with a mono font, some longer text.",
            Direction::LeftToRight,
            16.0,
        ),
        (
            NOTO_SANS,
            "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔̦̳͆̏̓͢",
            Direction::LeftToRight,
            14.0,
        ),
        (
            NOTO_SANS,
            " birth\u{ad}day ",
            Direction::LeftToRight,
            14.0,
        ),
        (
            NOTO_SANS_CJK,
            "你好世界，这是一段很长的测试文章",
            Direction::LeftToRight,
            14.0,
        ),
        (
            NOTO_SANS_DEVANAGARI,
            "आ रु॒क्मैरा यु॒धा नर॑ ऋ॒ष्वा ऋ॒ष्टीर॑सृक्षत ।",
            Direction::LeftToRight,
            14.0,
        ),
    ];
    let page_size = tiny_skia_path::Size::from_wh(200.0, 300.0).unwrap();
    let mut document_builder = Document::new(SerializeSettings::settings_1());
    let mut builder = document_builder.start_page(page_size);
    let mut surface = builder.surface();

    for (font, text, dir, size) in data {
        let font = Font::new(font.clone(), 0, Location::default()).unwrap();
        let glyphs = simple_shape(text, dir, font.clone(), size);
        surface.draw_glyph_run(0.0, y, Fill::<Rgb>::default(), &glyphs, font, text);

        y += size * 2.0;
    }

    surface.finish();
    builder.finish();

    let pdf = document_builder.finish();
    store_manual("simple_shape", &pdf);
}

#[ignore]
#[test]
fn cosmic_text_integration() {
    let mut font_system = FontSystem::new_with_fonts([Source::Binary(Arc::new(std::fs::read("/Users/lstampfl/Programming/GitHub/resvg/crates/resvg/tests/fonts/NotoSans-Regular.ttf").unwrap()))]);
    let metrics = Metrics::new(18.0, 20.0);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    buffer.set_size(&mut font_system, Some(200.0), None);
    let attrs = Attrs::new();
    let text = "Some text here. Let's make it a bit longer so that line wrapping kicks in 😊.\n我也要使用一些中文文字。 And also some اللغة العربية arabic text.\n\nहो। गए, उनका एक समय में\n\n\nz͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔̦̳͆̏̓͢";
    buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    let page_size = tiny_skia_path::Size::from_wh(200.0, 400.0).unwrap();
    let mut document_builder = Document::new(SerializeSettings::settings_1());
    let mut builder = document_builder.start_page(page_size);
    let mut surface = builder.surface();

    let font_map = surface.convert_fontdb(font_system.db_mut(), None);

    // Inspect the output runs
    for run in buffer.layout_runs() {
        let y_offset = run.line_y;

        let segmented = run
            .glyphs
            .group_by_key(|g| font_map.get(&g.font_id).unwrap().clone());

        let mut x = 0.0;
        for (font, glyphs) in segmented {
            let start_x = x;
            let glyphs = glyphs
                .iter()
                .map(|glyph| {
                    x += glyph.w;
                    Glyph::new(
                        GlyphId::new(glyph.glyph_id as u32),
                        glyph.w,
                        glyph.x_offset,
                        glyph.y_offset,
                        glyph.start..glyph.end,
                        glyph.font_size,
                    )
                })
                .collect::<Vec<_>>();

            surface.draw_glyph_run(
                start_x,
                y_offset,
                Fill::<Rgb>::default(),
                &glyphs,
                font,
                run.text,
            );
        }
    }

    surface.finish();
    builder.finish();

    let pdf = document_builder.finish();
    store_manual("cosmic_text", &pdf);
}
