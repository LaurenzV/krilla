use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use fontdb::Source;

use crate::document::Document;
use crate::font::Font;
use crate::rgb::Rgb;
use crate::serialize::SerializeSettings;
use crate::stream::Glyph;
use crate::tests::{
    simple_shape, write_manual_to_store, ASSETS_PATH, COLR_TEST_GLYPHS, DEJAVU_SANS_MONO,
    NOTO_SANS, NOTO_SANS_ARABIC, NOTO_SANS_CJK, NOTO_SANS_DEVANAGARI,
};
use crate::util::SliceExt;
use crate::Fill;
use rustybuzz::Direction;
use skrifa::instance::{Location, LocationRef, Size};
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider};
use std::sync::Arc;

#[ignore]
#[test]
fn simple_shape_demo() {
    let mut y = 25.0;

    let data = vec![
        (
            NOTO_SANS_ARABIC.clone(),
            "هذا نص أطول لتجربة القدرات.",
            Direction::RightToLeft,
            14.0,
        ),
        (
            NOTO_SANS.clone(),
            "Hi there, this is a very simple test!",
            Direction::LeftToRight,
            14.0,
        ),
        (
            DEJAVU_SANS_MONO.clone(),
            "Here with a mono font, some longer text.",
            Direction::LeftToRight,
            16.0,
        ),
        (NOTO_SANS.clone(), "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔̦̳͆̏̓͢", Direction::LeftToRight, 14.0),
        (
            NOTO_SANS.clone(),
            " birth\u{ad}day ",
            Direction::LeftToRight,
            14.0,
        ),
        (
            NOTO_SANS_CJK.clone(),
            "你好世界，这是一段很长的测试文章",
            Direction::LeftToRight,
            14.0,
        ),
        (
            NOTO_SANS_DEVANAGARI.clone(),
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
    write_manual_to_store("cosmic_text", &pdf);
}

#[ignore]
#[test]
fn twitter_color_emoji() {
    let font_data = std::fs::read("/Library/Fonts/TwitterColorEmoji-SVGinOT.ttf").unwrap();
    let mut document = Document::new(SerializeSettings::settings_1());
    all_glyphs_to_pdf(Arc::new(font_data), None, &mut document);
    write_manual_to_store("twitter_color_emoji", &document.finish());
}


#[ignore]
#[test]
fn colr_test_glyphs() {
    let font_data = COLR_TEST_GLYPHS.clone();

    let glyphs = (180..=180)
        .map(|n| (GlyphId::new(n), "".to_string()))
        .collect::<Vec<_>>();

    let mut document = Document::new(SerializeSettings::settings_1());
    all_glyphs_to_pdf(font_data, Some(glyphs), &mut document);
    write_manual_to_store("colr_test_glyphs", &document.finish());
}


#[ignore]
#[test]
fn noto_sans() {
    let font_data = NOTO_SANS.clone();

    let glyphs = (0..1000)
        .map(|n| (GlyphId::new(n), "".to_string()))
        .collect::<Vec<_>>();

    let mut document = Document::new(SerializeSettings::settings_1());
    all_glyphs_to_pdf(font_data, Some(glyphs), &mut document);
    write_manual_to_store("noto_sans", &document.finish());
}

#[ignore]
#[test]
fn apple_color_emoji() {
    let font_data = std::fs::read("/System/Library/Fonts/Apple Color Emoji.ttc").unwrap();

    let mut document = Document::new(SerializeSettings::settings_1());
    all_glyphs_to_pdf(Arc::new(font_data), None, &mut document);
    write_manual_to_store("apple_color_emoji", &document.finish());
}

#[ignore]
#[test]
fn noto_color_emoji() {
    let font_data = std::fs::read("/Library/Fonts/NotoColorEmoji-Regular.ttf").unwrap();
    let mut document = Document::new(SerializeSettings::settings_1());
    all_glyphs_to_pdf(Arc::new(font_data), None, &mut document);
    write_manual_to_store("noto_color_emoji", &document.finish());
}

#[ignore]
#[test]
fn segoe_ui_emoji() {
    let font_data = std::fs::read("/Library/Fonts/seguiemj.ttf").unwrap();
    let mut document = Document::new(SerializeSettings::settings_1());
    all_glyphs_to_pdf(Arc::new(font_data), None, &mut document);
    write_manual_to_store("segoe_ui_emoji", &document.finish());
}

pub fn all_glyphs_to_pdf(
    font_data: Arc<Vec<u8>>,
    glyphs: Option<Vec<(GlyphId, String)>>,
    db: &mut Document,
) {
    use crate::object::color_space::rgb::Rgb;
    use crate::stream::Glyph;
    use crate::Transform;

    let font = Font::new(font_data, 0, Location::default()).unwrap();
    let font_ref = font.font_ref();

    let glyphs = glyphs.unwrap_or_else(|| {
        let file = std::fs::read(ASSETS_PATH.join("emojis.txt")).unwrap();
        let file = std::str::from_utf8(&file).unwrap();
        file.chars()
            .filter_map(|c| {
                font_ref
                    .cmap()
                    .unwrap()
                    .map_codepoint(c)
                    .map(|g| (g, c.to_string()))
            })
            .collect::<Vec<_>>()
    });

    let metrics = font_ref.metrics(Size::unscaled(), LocationRef::default());
    let num_glyphs = glyphs.len();
    let width = 400;

    let size = 40u32;
    let num_cols = width / size;
    let height = (num_glyphs as f32 / num_cols as f32).ceil() as u32 * size;
    let units_per_em = metrics.units_per_em as f32;
    let mut cur_point = 0;

    let page_size = tiny_skia_path::Size::from_wh(width as f32, height as f32).unwrap();
    let mut builder = db.start_page(page_size);
    let mut surface = builder.surface();

    for (i, text) in glyphs.iter().cloned() {
        fn get_transform(cur_point: u32, size: u32, num_cols: u32, _: f32) -> Transform {
            let el = cur_point / size;
            let col = el % num_cols;
            let row = el / num_cols;

            Transform::from_row(
                1.0,
                0.0,
                0.0,
                1.0,
                col as f32 * size as f32,
                (row + 1) as f32 * size as f32,
            )
        }

        surface.push_transform(&get_transform(cur_point, size, num_cols, units_per_em));
        surface.draw_glyph_run(
            0.0,
            0.0,
            crate::Fill::<Rgb>::default(),
            &[Glyph::new(i, 0.0, 0.0, 0.0, 0..text.len(), size as f32)],
            font.clone(),
            &text,
        );
        surface.pop();

        cur_point += size;
    }

    surface.finish();
    builder.finish();
}
