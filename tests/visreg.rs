use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use fontdb::{Database, Source};
use image::{Rgba, RgbaImage};
use krilla::document::Document;
use krilla::rgb::Rgb;
use krilla::serialize::SerializeSettings;
use krilla::stream::Glyph;
use krilla::surface::Surface;
use krilla::util::SliceExt;
use krilla::{rgb, Fill, LinearGradient, Paint, SpreadMethod, Stop};
use krilla_macros::visreg;
use sitro::{
    render_ghostscript, render_mupdf, render_pdfbox, render_pdfium, render_pdfjs, render_poppler,
    render_quartz, RenderOptions, RenderedDocument, Renderer,
};
use skrifa::GlyphId;
use std::cmp::max;
use std::path::PathBuf;
use std::sync::Arc;
use tiny_skia_path::{PathBuilder, Rect, Size, Transform};
use usvg::NormalizedF32;

pub fn render_doc(doc: &[u8], renderer: &Renderer) -> RenderedDocument {
    let options = RenderOptions { scale: 1.0 };

    match renderer {
        Renderer::Pdfium => render_pdfium(doc, &options).unwrap(),
        Renderer::Mupdf => render_mupdf(doc, &options).unwrap(),
        Renderer::Poppler => render_poppler(doc, &options).unwrap(),
        Renderer::Quartz => render_quartz(doc, &options).unwrap(),
        Renderer::Pdfjs => render_pdfjs(doc, &options).unwrap(),
        Renderer::Pdfbox => render_pdfbox(doc, &options).unwrap(),
        Renderer::Ghostscript => render_ghostscript(doc, &options).unwrap(),
    }
}

pub fn check_render(name: &str, renderer: &Renderer, document: RenderedDocument) {
    let refs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/refs")
        .join(name);

    let _ = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/diff")
        .join(name);

    std::fs::create_dir_all(&refs_path).unwrap();

    if document.is_empty() {
        panic!("empty document");
    } else if document.len() == 1 {
        let ref_path = refs_path.join(format!("{}.png", renderer.name()));

        // let reference = load_from_memory(&std::fs::read(&ref_path).unwrap()).unwrap().into_rgba8();
        // let actual = load_from_memory(&document[0]).unwrap().into_rgba8();
        //
        // let (diff_image, pixel_diff) = get_diff(&reference, &actual);
        //
        // if pixel_diff != 0 {
        //     std::fs::create_dir_all(&diffs_path).unwrap();
        //     let diff_path = diffs_path.join(format!("{}.png", renderer.name()));
        //     diff_image
        //         .save_with_format(&diff_path, image::ImageFormat::Png)
        //         .unwrap();
        // }
        //
        // assert_eq!(pixel_diff, 0);

        std::fs::write(&ref_path, &document[0]).unwrap();
    } else {
        for (index, page) in document.iter().enumerate() {
            let ref_path = refs_path.join(format!("{}_{}.png", index + 1, renderer.name()));
            std::fs::write(&ref_path, page).unwrap();
        }
    }
}

pub fn get_diff(expected_image: &RgbaImage, actual_image: &RgbaImage) -> (RgbaImage, i32) {
    let width = max(expected_image.width(), actual_image.width());
    let height = max(expected_image.height(), actual_image.height());

    let mut diff_image = RgbaImage::new(width * 3, height);

    let mut pixel_diff = 0;

    for x in 0..width {
        for y in 0..height {
            let actual_pixel = actual_image.get_pixel_checked(x, y);
            let expected_pixel = expected_image.get_pixel_checked(x, y);

            match (actual_pixel, expected_pixel) {
                (Some(actual), Some(expected)) => {
                    diff_image.put_pixel(x, y, *expected);
                    diff_image.put_pixel(x + 2 * width, y, *actual);
                    if is_pix_diff(expected, actual) {
                        pixel_diff += 1;
                        diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                    } else {
                        diff_image.put_pixel(x + width, y, Rgba([0, 0, 0, 255]))
                    }
                }
                (Some(actual), None) => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x + 2 * width, y, *actual);
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                }
                (None, Some(expected)) => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x, y, *expected);
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                }
                _ => unreachable!(),
            }
        }
    }

    (diff_image, pixel_diff)
}

fn is_pix_diff(pixel1: &Rgba<u8>, pixel2: &Rgba<u8>) -> bool {
    if pixel1.0[3] == 0 && pixel2.0[3] == 0 {
        return false;
    }

    pixel1.0[0] != pixel2.0[0]
        || pixel1.0[1] != pixel2.0[1]
        || pixel1.0[2] != pixel2.0[2]
        || pixel1.0[3] != pixel2.0[3]
}

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
    db.load_font_source(Source::Binary(Arc::new(include_bytes!(
        "fonts/NotoSans-Regular.ttf"
    ))));
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
