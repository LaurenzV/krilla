use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use fontdb::Source;

use crate::color::rgb::Rgb;
use crate::document::{Document, PageSettings};
use crate::font::Font;
use crate::font::Glyph;
use crate::path::Fill;
use crate::serialize::SerializeSettings;
use crate::tests::{
    all_glyphs_to_pdf, write_manual_to_store, COLR_TEST_GLYPHS, DEJAVU_SANS_MONO, NOTO_SANS,
    NOTO_SANS_ARABIC, NOTO_SANS_CJK, NOTO_SANS_DEVANAGARI,
};
use crate::util::SliceExt;
use skrifa::GlyphId;
use std::sync::Arc;
use tiny_skia_path::Point;

#[ignore]
#[test]
fn simple_shape_demo() {
    let mut y = 25.0;

    let data = vec![
        (
            NOTO_SANS_ARABIC.clone(),
            "هذا نص أطول لتجربة القدرات.",
            14.0,
        ),
        (
            NOTO_SANS.clone(),
            "Hi there, this is a very simple test!",
            14.0,
        ),
        (
            DEJAVU_SANS_MONO.clone(),
            "Here with a mono font, some longer text.",
            16.0,
        ),
        (NOTO_SANS.clone(), "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔̦̳͆̏̓͢", 14.0),
        (NOTO_SANS.clone(), " birth\u{ad}day ", 14.0),
        (
            NOTO_SANS_CJK.clone(),
            "你好世界，这是一段很长的测试文章",
            14.0,
        ),
        (
            NOTO_SANS_DEVANAGARI.clone(),
            "आ रु॒क्मैरा यु॒धा नर॑ ऋ॒ष्वा ऋ॒ष्टीर॑सृक्षत ।",
            14.0,
        ),
    ];

    let page_settings = PageSettings::new(200.0, 300.0);

    let mut document_builder = Document::new_with(SerializeSettings::settings_1());
    let mut builder = document_builder.start_page_with(page_settings);
    let mut surface = builder.surface();

    for (font, text, size) in data {
        let font = Font::new(font.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, y),
            Fill::<Rgb>::default(),
            font,
            size,
            &[],
            text,
        );

        y += size * 2.0;
    }

    surface.finish();
    builder.finish();

    let pdf = document_builder.finish().unwrap();
    write_manual_to_store("simple_shape", &pdf);
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

    let page_settings = PageSettings::new(200.0, 400.0);

    let mut document_builder = Document::new_with(SerializeSettings::settings_1());
    let mut builder = document_builder.start_page_with(page_settings);
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

            surface.fill_glyphs(
                Point::from_xy(start_x, y_offset),
                Fill::<Rgb>::default(),
                &glyphs,
                font,
                run.text,
            );
        }
    }

    surface.finish();
    builder.finish();

    let pdf = document_builder.finish().unwrap();
    write_manual_to_store("cosmic_text", &pdf);
}

#[ignore]
#[test]
fn twitter_color_emoji() {
    let font_data = std::fs::read("/Library/Fonts/TwitterColorEmoji-SVGinOT.ttf").unwrap();
    let mut document = Document::new_with(SerializeSettings::settings_1());
    all_glyphs_to_pdf(Arc::new(font_data), None, false, &mut document);
    write_manual_to_store("twitter_color_emoji", &document.finish().unwrap());
}

#[ignore]
#[test]
fn colr_test_glyphs() {
    let font_data = COLR_TEST_GLYPHS.clone();

    let glyphs = (180..=180)
        .map(|n| (GlyphId::new(n), "".to_string()))
        .collect::<Vec<_>>();

    let mut document = Document::new_with(SerializeSettings::settings_1());
    all_glyphs_to_pdf(font_data, Some(glyphs), false, &mut document);
    write_manual_to_store("colr_test_glyphs", &document.finish().unwrap());
}

#[ignore]
#[test]
fn noto_sans() {
    let font_data = NOTO_SANS.clone();

    let glyphs = (0..1000)
        .map(|n| (GlyphId::new(n), "".to_string()))
        .collect::<Vec<_>>();

    let mut document = Document::new_with(SerializeSettings::settings_1());
    all_glyphs_to_pdf(font_data, Some(glyphs), false, &mut document);
    write_manual_to_store("noto_sans", &document.finish().unwrap());
}

#[ignore]
#[test]
fn apple_color_emoji() {
    let font_data = std::fs::read("/System/Library/Fonts/Apple Color Emoji.ttc").unwrap();

    let mut document = Document::new_with(SerializeSettings::settings_1());
    all_glyphs_to_pdf(Arc::new(font_data), None, false, &mut document);
    write_manual_to_store("apple_color_emoji", &document.finish().unwrap());
}

#[ignore]
#[test]
fn noto_color_emoji() {
    let font_data = std::fs::read("/Library/Fonts/NotoColorEmoji-Regular.ttf").unwrap();
    let mut document = Document::new_with(SerializeSettings::settings_1());
    all_glyphs_to_pdf(Arc::new(font_data), None, false, &mut document);
    write_manual_to_store("NOTO_COLOR_EMOJI_COLR", &document.finish().unwrap());
}

#[ignore]
#[test]
fn segoe_ui_emoji() {
    let font_data = std::fs::read("/Library/Fonts/seguiemj.ttf").unwrap();
    let mut document = Document::new_with(SerializeSettings::settings_1());
    all_glyphs_to_pdf(Arc::new(font_data), None, false, &mut document);
    write_manual_to_store("segoe_ui_emoji", &document.finish().unwrap());
}
