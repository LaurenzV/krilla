use crate::rgb::Rgb;
use crate::stream::Glyph;
use crate::surface::Surface;
use crate::tests::NOTO_SANS;
use crate::util::SliceExt;
use crate::{rgb, Fill, LinearGradient, Paint, SpreadMethod, Stop};
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use fontdb::{Database, Source};
use krilla_macros::visreg;
use skrifa::GlyphId;
use std::sync::Arc;
use tiny_skia_path::{NormalizedF32, PathBuilder, Rect, Transform};

#[visreg]
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

    surface.draw_path(
        &path,
        Fill {
            paint: Paint::LinearGradient(gradient),
            opacity: NormalizedF32::new(0.5).unwrap(),
            rule: Default::default(),
        },
    );
}

#[visreg]
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
}
