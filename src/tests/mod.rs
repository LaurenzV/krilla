use crate::color::cmyk::DeviceCmyk;
use crate::color::luma::Luma;
use crate::color::rgb::Rgb;
use crate::color::{cmyk, luma, rgb};
use crate::document::{Document, PageSettings};
use crate::font::{Font, GlyphUnits};
use crate::image::Image;
use crate::mask::{Mask, MaskType};
use crate::paint::{Paint, Stop};
use crate::path::{Fill, Stroke};
use crate::stream::Stream;
use crate::stream::StreamBuilder;
use crate::surface::Surface;
use crate::SerializeSettings;
use difference::{Changeset, Difference};
use image::{load_from_memory, Rgba, RgbaImage};
use once_cell::sync::Lazy;
use oxipng::{InFile, OutFile};
use sitro::{
    render_ghostscript, render_mupdf, render_pdfbox, render_pdfium, render_pdfjs, render_poppler,
    render_quartz, RenderOptions, RenderedDocument, RenderedPage, Renderer,
};
use skrifa::instance::{LocationRef, Size};
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider};
use std::cmp::max;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use tiny_skia_path::{NormalizedF32, Path, PathBuilder, Point, Rect, Transform};

#[allow(dead_code)]
#[rustfmt::skip]
mod svg;

const REPLACE: Option<&str> = option_env!("REPLACE");
const STORE: Option<&str> = option_env!("STORE");

static ASSETS_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));

static SNAPSHOT_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/snapshots");
    let _ = std::fs::create_dir_all(&path);
    path
});

static REFS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/refs");
    let _ = std::fs::create_dir_all(&path);
    path
});

pub static SVGS_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/svgs"));

static DIFFS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/diffs");
    let _ = std::fs::remove_dir_all(&path);
    let _ = std::fs::create_dir_all(&path);
    path
});

static STORE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/store");
    let _ = std::fs::remove_dir_all(&path);
    let _ = std::fs::create_dir_all(&path);
    path
});

static FONT_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts"));

macro_rules! lazy_font {
    ($name:ident, $path:expr) => {
        pub static $name: LazyLock<Arc<Vec<u8>>> =
            LazyLock::new(|| Arc::new(std::fs::read($path).unwrap()));
    };
}

#[rustfmt::skip]
lazy_font!(NOTO_SANS, FONT_PATH.join("NotoSans-Regular.ttf"));
#[rustfmt::skip]
lazy_font!(DEJAVU_SANS_MONO, FONT_PATH.join("DejaVuSansMono.ttf"));
#[rustfmt::skip]
lazy_font!(LATIN_MODERN_ROMAN, FONT_PATH.join("LatinModernRoman-Regular.otf"));
#[rustfmt::skip]
lazy_font!(NOTO_SANS_ARABIC, FONT_PATH.join("NotoSansArabic-Regular.ttf"));
#[rustfmt::skip]
lazy_font!(NOTO_SANS_CJK, FONT_PATH.join("NotoSansCJKsc-Regular.otf"));
#[rustfmt::skip]
lazy_font!(NOTO_SANS_DEVANAGARI, FONT_PATH.join("NotoSansDevanagari-Regular.ttf"));
#[rustfmt::skip]
lazy_font!(COLR_TEST_GLYPHS, FONT_PATH.join("colr_test_glyphs.ttf"));
#[rustfmt::skip]
lazy_font!(NOTO_COLOR_EMOJI_COLR, FONT_PATH.join("NotoColorEmoji.COLR.subset.ttf"));
#[rustfmt::skip]
lazy_font!(NOTO_COLOR_EMOJI_CBDT, FONT_PATH.join("NotoColorEmoji.CBDT.subset.ttf"));
#[rustfmt::skip]
lazy_font!(TWITTER_COLOR_EMOJI, FONT_PATH.join("TwitterColorEmoji.subset.ttf"));
#[rustfmt::skip]
lazy_font!(NOTO_SANS_VARIABLE, FONT_PATH.join("NotoSans_variable.ttf"));

pub fn green_fill(opacity: f32) -> Fill<Rgb> {
    Fill {
        paint: Paint::Color(rgb::Color::new(0, 255, 0)),
        opacity: NormalizedF32::new(opacity).unwrap(),
        rule: Default::default(),
    }
}

pub fn basic_mask(surface: &mut Surface, mask_type: MaskType) -> Mask {
    let mut stream_builder = surface.stream_builder();
    let mut sub_surface = stream_builder.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    sub_surface.fill_path(&path, red_fill(0.2));
    sub_surface.finish();

    Mask::new(stream_builder.finish(), mask_type)
}

pub fn blue_fill(opacity: f32) -> Fill<Rgb> {
    Fill {
        paint: Paint::Color(rgb::Color::new(0, 0, 255)),
        opacity: NormalizedF32::new(opacity).unwrap(),
        rule: Default::default(),
    }
}

pub fn blue_stroke(opacity: f32) -> Stroke<Rgb> {
    Stroke {
        paint: Paint::Color(rgb::Color::new(0, 0, 255)),
        opacity: NormalizedF32::new(opacity).unwrap(),
        ..Stroke::default()
    }
}

pub fn red_fill(opacity: f32) -> Fill<Rgb> {
    Fill {
        paint: Paint::Color(rgb::Color::new(255, 0, 0)),
        opacity: NormalizedF32::new(opacity).unwrap(),
        rule: Default::default(),
    }
}

pub fn red_stroke(opacity: f32) -> Stroke<Rgb> {
    Stroke {
        paint: Paint::Color(rgb::Color::new(255, 0, 0)),
        opacity: NormalizedF32::new(opacity).unwrap(),
        ..Stroke::default()
    }
}

pub fn purple_fill(opacity: f32) -> Fill<Rgb> {
    Fill {
        paint: Paint::Color(rgb::Color::new(128, 0, 128)),
        opacity: NormalizedF32::new(opacity).unwrap(),
        rule: Default::default(),
    }
}

pub fn gray_luma(opacity: f32) -> Fill<Luma> {
    Fill {
        paint: Paint::Color(luma::Color::new(127)),
        opacity: NormalizedF32::new(opacity).unwrap(),
        rule: Default::default(),
    }
}

pub fn cmyk_fill(opacity: f32) -> Fill<DeviceCmyk> {
    Fill {
        paint: Paint::Color(cmyk::Color::new(0, 8, 252, 5)),
        opacity: NormalizedF32::new(opacity).unwrap(),
        rule: Default::default(),
    }
}

pub fn rect_to_path(x1: f32, y1: f32, x2: f32, y2: f32) -> Path {
    let mut builder = PathBuilder::new();
    builder.push_rect(Rect::from_ltrb(x1, y1, x2, y2).unwrap());
    builder.finish().unwrap()
}

pub fn load_png_image(name: &str) -> Image {
    Image::from_png(&std::fs::read(ASSETS_PATH.join("images").join(name)).unwrap()).unwrap()
}

pub fn load_jpg_image(name: &str) -> Image {
    Image::from_jpeg(&std::fs::read(ASSETS_PATH.join("images").join(name)).unwrap()).unwrap()
}

pub fn load_gif_image(name: &str) -> Image {
    Image::from_gif(&std::fs::read(ASSETS_PATH.join("images").join(name)).unwrap()).unwrap()
}

pub fn load_webp_image(name: &str) -> Image {
    Image::from_webp(&std::fs::read(ASSETS_PATH.join("images").join(name)).unwrap()).unwrap()
}

fn write_snapshot_to_store(name: &str, content: &[u8]) {
    let mut path = STORE_PATH.clone().join("snapshots");
    let _ = std::fs::create_dir_all(&path);
    path.push(format!("{}.pdf", name));
    std::fs::write(&path, content).unwrap();
}

fn write_render_to_store(name: &str, content: &[u8]) {
    let mut path = STORE_PATH.clone().join("refs");
    let _ = std::fs::create_dir_all(&path);
    path.push(format!("{}.pdf", name));
    std::fs::write(&path, content).unwrap();
}

pub fn write_manual_to_store(name: &str, data: &[u8]) {
    let path = STORE_PATH.clone().join("manual");
    let _ = std::fs::create_dir_all(&path);

    let pdf_path = path.join(format!("{}.pdf", name));
    let txt_path = path.join(format!("{}.txt", name));
    std::fs::write(pdf_path, data).unwrap();
    std::fs::write(txt_path, data).unwrap();
}

pub fn check_snapshot(name: &str, content: &[u8], storable: bool) {
    let path = SNAPSHOT_PATH.join(format!("{}.txt", name));

    if STORE.is_some() && storable {
        write_snapshot_to_store(name, content);
    }

    if !path.exists() {
        std::fs::write(&path, content).unwrap();
        panic!("new snapshot created");
    }

    let actual = std::fs::read(&path).unwrap();

    if REPLACE.is_some() && actual != content {
        std::fs::write(&path, content).unwrap();
        panic!("test was replaced");
    }

    let changeset = Changeset::new(
        &String::from_utf8_lossy(content),
        &String::from_utf8_lossy(&actual),
        "\n",
    );

    if changeset.distance != 0 {
        for diff in changeset.diffs {
            match diff {
                Difference::Same(ref x) => {
                    eprintln!(" {}", x);
                }
                Difference::Add(ref x) => {
                    eprintln!("+++++++++++++++++++\n{}\n+++++++++++++++++++", x);
                }
                Difference::Rem(ref x) => {
                    eprintln!("-------------------\n{}\n-------------------", x);
                }
            }
        }
    }

    assert_eq!(changeset.distance, 0);
}

pub fn check_render(
    name: &str,
    renderer: &Renderer,
    document: RenderedDocument,
    pdf: &[u8],
    ignore_renderer: bool,
) {
    let refs_path = REFS_PATH.clone();

    let renderer_suffix = if ignore_renderer {
        "".to_string()
    } else {
        format!("_{}", renderer.name())
    };

    let check_single = |name: String, page: &RenderedPage| {
        let ref_path = refs_path.join(format!("{}.png", name));

        if !ref_path.exists() {
            std::fs::write(&ref_path, page).unwrap();
            oxipng::optimize(
                &InFile::Path(ref_path.clone()),
                &OutFile::from_path(ref_path),
                &oxipng::Options::max_compression(),
            )
            .unwrap();
            panic!("new reference image was created");
        }

        let reference = load_from_memory(&std::fs::read(&ref_path).unwrap())
            .unwrap()
            .into_rgba8();
        let actual = load_from_memory(&document[0]).unwrap().into_rgba8();

        let (diff_image, pixel_diff) = get_diff(&reference, &actual);

        let threshold = env::var("KRILLA_THRESHOLD")
            .unwrap_or("0".to_string())
            .parse::<u32>()
            .unwrap();
        if pixel_diff > threshold {
            let diff_path = DIFFS_PATH.join(format!("{}.png", name));
            diff_image
                .save_with_format(&diff_path, image::ImageFormat::Png)
                .unwrap();

            if REPLACE.is_some() {
                std::fs::write(&ref_path, page).unwrap();
                oxipng::optimize(
                    &InFile::Path(ref_path.clone()),
                    &OutFile::from_path(ref_path),
                    &oxipng::Options::max_compression(),
                )
                .unwrap();
                panic!("test was replaced");
            }

            panic!(
                "pixel diff was {}, while threshold is {}",
                pixel_diff, threshold
            );
        }

        if pixel_diff != 0 {
            eprintln!("Warning: pixel diff was {} instead of 0", pixel_diff);
        }
    };

    if STORE.is_some() {
        write_render_to_store(name, pdf);
    }

    if document.is_empty() {
        panic!("empty document");
    } else if document.len() == 1 {
        check_single(format!("{}{}", name, renderer_suffix), &document[0]);
    } else {
        for (index, page) in document.iter().enumerate() {
            check_single(format!("{}{}_{}", name, renderer_suffix, index), page);
        }
    }
}

pub fn render_document(doc: &[u8], renderer: &Renderer) -> RenderedDocument {
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

pub fn get_diff(expected_image: &RgbaImage, actual_image: &RgbaImage) -> (RgbaImage, u32) {
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

pub fn all_glyphs_to_pdf(
    font_data: Arc<Vec<u8>>,
    glyphs: Option<Vec<(GlyphId, String)>>,
    color_cycling: bool,
    d: &mut Document,
) {
    use crate::font::KrillaGlyph;
    use crate::geom::Transform;
    use crate::object::color::rgb::Rgb;

    let font = Font::new(font_data, 0, vec![]).unwrap();
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

    let mut builder = d.start_page_with(PageSettings::new(width as f32, height as f32));
    let mut surface = builder.surface();

    let colors = if color_cycling {
        vec![
            rgb::Color::new(50, 168, 82),
            rgb::Color::new(154, 50, 168),
            rgb::Color::new(232, 21, 56),
            rgb::Color::new(227, 215, 84),
            rgb::Color::new(16, 16, 230),
        ]
    } else {
        vec![rgb::Color::new(0, 0, 0)]
    };

    let mut color_picker = colors.iter().cycle();
    let mut color = *color_picker.next().unwrap();

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

        if (cur_point / size) % num_cols == 0 {
            color = *color_picker.next().unwrap();
        }

        surface.push_transform(&get_transform(cur_point, size, num_cols, units_per_em));
        surface.fill_glyphs(
            Point::from_xy(0.0, 0.0),
            Fill::<Rgb> {
                paint: Paint::Color(color),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            },
            &[KrillaGlyph::new(i, 0.0, 0.0, 0.0, 0..text.len())],
            font.clone(),
            &text,
            size as f32,
            GlyphUnits::UserSpace,
        );
        surface.pop();

        cur_point += size;
    }

    surface.finish();
    builder.finish();
}

pub fn stops_with_1_solid() -> Vec<Stop<Rgb>> {
    vec![Stop {
        offset: NormalizedF32::new(0.5).unwrap(),
        color: rgb::Color::new(255, 0, 0),
        opacity: NormalizedF32::ONE,
    }]
}

pub fn stops_with_2_solid_1() -> Vec<Stop<Rgb>> {
    vec![
        Stop {
            offset: NormalizedF32::new(0.2).unwrap(),
            color: rgb::Color::new(255, 0, 0),
            opacity: NormalizedF32::ONE,
        },
        Stop {
            offset: NormalizedF32::new(0.8).unwrap(),
            color: rgb::Color::new(255, 255, 0),
            opacity: NormalizedF32::ONE,
        },
    ]
}

pub fn stops_with_2_solid_2() -> Vec<Stop<Rgb>> {
    vec![
        Stop {
            offset: NormalizedF32::new(0.2).unwrap(),
            color: rgb::Color::new(85, 235, 52),
            opacity: NormalizedF32::ONE,
        },
        Stop {
            offset: NormalizedF32::new(0.8).unwrap(),
            color: rgb::Color::new(89, 52, 235),
            opacity: NormalizedF32::ONE,
        },
    ]
}

pub fn stops_with_2_opacity() -> Vec<Stop<Rgb>> {
    vec![
        Stop {
            offset: NormalizedF32::new(0.2).unwrap(),
            color: rgb::Color::new(85, 235, 52),
            opacity: NormalizedF32::new(0.8).unwrap(),
        },
        Stop {
            offset: NormalizedF32::new(0.8).unwrap(),
            color: rgb::Color::new(89, 52, 235),
            opacity: NormalizedF32::new(0.2).unwrap(),
        },
    ]
}

pub fn stops_with_3_solid_1() -> Vec<Stop<Rgb>> {
    vec![
        Stop {
            offset: NormalizedF32::new(0.1).unwrap(),
            color: rgb::Color::new(255, 0, 0),
            opacity: NormalizedF32::ONE,
        },
        Stop {
            offset: NormalizedF32::new(0.3).unwrap(),
            color: rgb::Color::new(255, 255, 0),
            opacity: NormalizedF32::ONE,
        },
        Stop {
            offset: NormalizedF32::new(0.8).unwrap(),
            color: rgb::Color::new(0, 255, 255),
            opacity: NormalizedF32::ONE,
        },
    ]
}

pub fn stops_with_3_solid_2() -> Vec<Stop<Rgb>> {
    vec![
        Stop {
            offset: NormalizedF32::new(0.1).unwrap(),
            color: rgb::Color::new(85, 235, 52),
            opacity: NormalizedF32::ONE,
        },
        Stop {
            offset: NormalizedF32::new(0.5).unwrap(),
            color: rgb::Color::new(89, 52, 235),
            opacity: NormalizedF32::ONE,
        },
        Stop {
            offset: NormalizedF32::new(1.0).unwrap(),
            color: rgb::Color::new(235, 52, 110),
            opacity: NormalizedF32::ONE,
        },
    ]
}

pub fn stops_with_3_opacity() -> Vec<Stop<Rgb>> {
    vec![
        Stop {
            offset: NormalizedF32::new(0.2).unwrap(),
            color: rgb::Color::new(85, 235, 52),
            opacity: NormalizedF32::new(0.4).unwrap(),
        },
        Stop {
            offset: NormalizedF32::new(0.8).unwrap(),
            color: rgb::Color::new(89, 52, 235),
            opacity: NormalizedF32::new(0.8).unwrap(),
        },
        Stop {
            offset: NormalizedF32::new(0.8).unwrap(),
            color: rgb::Color::new(235, 52, 110),
            opacity: NormalizedF32::new(0.1).unwrap(),
        },
    ]
}

pub fn basic_pattern_stream(mut stream_builder: StreamBuilder) -> Stream {
    let path = rect_to_path(0.0, 0.0, 10.0, 10.0);

    let mut surface = stream_builder.surface();
    surface.fill_path(&path, red_fill(1.0));
    surface.push_transform(&Transform::from_translate(10.0, 10.0));
    surface.fill_path(&path, green_fill(1.0));
    surface.pop();
    surface.finish();

    stream_builder.finish()
}

static FONTDB: Lazy<Arc<fontdb::Database>> = Lazy::new(|| {
    let mut fontdb = fontdb::Database::new();
    fontdb.load_fonts_dir(ASSETS_PATH.join("svg_fonts"));

    fontdb.set_serif_family("Noto Serif");
    fontdb.set_sans_serif_family("Noto Sans");
    fontdb.set_cursive_family("Yellowtail");
    fontdb.set_fantasy_family("Sedgwick Ave Display");
    fontdb.set_monospace_family("Noto Mono");

    Arc::new(fontdb)
});

fn svg_impl(name: &str, renderer: Renderer) {
    let settings = SerializeSettings::default();
    let mut d = Document::new_with(settings);
    let svg_path = ASSETS_PATH.join(format!("svgs/{}.svg", name));
    let data = std::fs::read(&svg_path).unwrap();
    let tree = usvg::Tree::from_data(
        &data,
        &usvg::Options {
            fontdb: FONTDB.clone(),
            ..Default::default()
        },
    )
    .unwrap();

    let mut page = d.start_page_with(PageSettings::new(tree.size().width(), tree.size().height()));
    let mut surface = page.surface();
    surface.draw_svg(&tree, tree.size());
    surface.finish();
    page.finish();

    let pdf = d.finish().unwrap();
    let rendered = render_document(&pdf, &renderer);
    check_render(&format!("svg_{}", name), &renderer, rendered, &pdf, true);
}
