use crate::document::Document;
use crate::rgb::Rgb;
use crate::stream::Glyph;
use crate::surface::Surface;
use crate::tests::manual::all_glyphs_to_pdf;
use crate::tests::{
    load_image, COLR_TEST_GLYPHS, NOTO_COLOR_EMOJI, NOTO_SANS, TWITTER_COLOR_EMOJI,
};
use crate::util::SliceExt;
use crate::{rgb, Fill, LinearGradient, Paint, SpreadMethod, Stop};
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use fontdb::{Database, Source};
use krilla_macros::visreg;
use skrifa::GlyphId;
use std::sync::Arc;
use tiny_skia_path::{NormalizedF32, PathBuilder, Point, Rect, Transform};

#[visreg(all)]
fn linear_gradient(surface: &mut Surface) {
    let mut builder = PathBuilder::new();
    builder.push_rect(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap());
    let path = builder.finish().unwrap();

    let gradient = LinearGradient {
        x1: 20.0,
        y1: 0.0,
        x2: 180.0,
        y2: 0.0,
        transform: Transform::identity(),
        spread_method: SpreadMethod::Pad,
        stops: vec![
            Stop::<Rgb> {
                offset: NormalizedF32::new(0.0).unwrap(),
                color: rgb::Color::new(255, 0, 0),
                opacity: NormalizedF32::new(1.0).unwrap(),
            },
            Stop {
                offset: NormalizedF32::new(0.5).unwrap(),
                color: rgb::Color::new(0, 255, 0),
                opacity: NormalizedF32::new(0.5).unwrap(),
            },
            Stop {
                offset: NormalizedF32::new(1.0).unwrap(),
                color: rgb::Color::new(0, 0, 255),
                opacity: NormalizedF32::new(1.0).unwrap(),
            },
        ],
    };

    surface.fill_path(
        &path,
        Fill {
            paint: Paint::LinearGradient(gradient),
            opacity: NormalizedF32::new(0.5).unwrap(),
            rule: Default::default(),
        },
    );
}

#[visreg(all)]
fn cosmic_text(surface: &mut Surface) {
    let mut db = Database::new();
    db.load_font_source(Source::Binary(NOTO_SANS.clone()));
    let mut font_system = FontSystem::new_with_locale_and_db("".to_string(), db);
    assert_eq!(font_system.db().len(), 1);
    let metrics = Metrics::new(14.0, 20.0);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    buffer.set_size(&mut font_system, Some(200.0), None);
    let attrs = Attrs::new();
    let text = "Some text here. Let's make it a bit longer so that line wrapping kicks in";
    buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    let font_map = surface.convert_fontdb(font_system.db_mut(), None);

    // Inspect the output runs
    for run in buffer.layout_runs() {
        let y_offset = run.line_y;

        let segmented = run
            .glyphs
            .group_by_key(|g| (font_map.get(&g.font_id).unwrap().clone(), g.font_size));

        let mut x = 0.0;
        for ((font, size), glyphs) in segmented {
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
                        size,
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
}

#[visreg(document, settings_3)]
fn colr_test_glyphs(document: &mut Document) {
    let font_data = COLR_TEST_GLYPHS.clone();

    let glyphs = (0..=220)
        .map(|n| (GlyphId::new(n), "".to_string()))
        .collect::<Vec<_>>();

    all_glyphs_to_pdf(font_data, Some(glyphs), document);
}

#[visreg(document)]
fn noto_color_emoji(document: &mut Document) {
    let font_data = NOTO_COLOR_EMOJI.clone();
    all_glyphs_to_pdf(font_data, None, document);
}

#[visreg(document)]
#[cfg(target_os = "macos")]
fn apple_color_emoji(document: &mut Document) {
    let font_data = Arc::new(std::fs::read("/System/Library/Fonts/Apple Color Emoji.ttc").unwrap());
    all_glyphs_to_pdf(font_data, None, document);
}

#[visreg(document)]
fn twitter_color_emoji(document: &mut Document) {
    let font_data = TWITTER_COLOR_EMOJI.clone();
    all_glyphs_to_pdf(font_data, None, document);
}

fn image_impl(surface: &mut Surface, name: &str) {
    let image = load_image(name);
    let size = image.size();
    surface.draw_image(image, size);
}

#[visreg(all)]
fn image_luma8_png(surface: &mut Surface) {
    image_impl(surface, "luma8.png");
}

#[visreg(all)]
fn image_luma16_png(surface: &mut Surface) {
    image_impl(surface, "luma16.png");
}

#[visreg(all)]
fn image_rgb8_png(surface: &mut Surface) {
    image_impl(surface, "rgb8.png");
}

#[visreg(all)]
fn image_rgb16_png(surface: &mut Surface) {
    image_impl(surface, "rgb16.png");
}

#[visreg(all)]
fn image_rgba8_png(surface: &mut Surface) {
    image_impl(surface, "rgba8.png");
}

#[visreg(all)]
fn image_rgba16_png(surface: &mut Surface) {
    image_impl(surface, "rgba16.png");
}
