use crate::image::Image;
use difference::{Changeset, Difference};
use image::{load_from_memory, Rgba, RgbaImage};
use oxipng::{InFile, OutFile};
use sitro::{
    render_ghostscript, render_mupdf, render_pdfbox, render_pdfium, render_pdfjs, render_poppler,
    render_quartz, RenderOptions, RenderedDocument, RenderedPage, Renderer,
};
use std::cmp::max;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use tiny_skia_path::{Path, PathBuilder, Rect};

mod manual;
mod visreg;

const REPLACE: Option<&str> = option_env!("REPLACE");
const STORE: Option<&str> = option_env!("STORE");
pub const SKIP_VISREG: Option<&str> = option_env!("SKIP_VISREG");
pub const SKIP_SNAPSHOT: Option<&str> = option_env!("SKIP_SNAPSHOT");

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
lazy_font!(NOTO_COLOR_EMOJI, FONT_PATH.join("NotoColorEmoji.COLR.subset.ttf"));
#[rustfmt::skip]
lazy_font!(TWITTER_COLOR_EMOJI, FONT_PATH.join("TwitterColorEmoji.subset.ttf"));

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
    std::fs::write(&path, &content).unwrap();
}

fn write_render_to_store(name: &str, content: &[u8]) {
    let mut path = STORE_PATH.clone().join("refs");
    let _ = std::fs::create_dir_all(&path);
    path.push(format!("{}.pdf", name));
    std::fs::write(&path, &content).unwrap();
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
        std::fs::write(&path, &content).unwrap();
        panic!("new snapshot created");
    }

    let actual = std::fs::read(&path).unwrap();

    if REPLACE.is_some() && &actual != content {
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

    if document.is_empty() {
        panic!("empty document");
    } else if document.len() == 1 {
        check_single(format!("{}{}", name, renderer_suffix), &document[0]);
    } else {
        for (index, page) in document.iter().enumerate() {
            check_single(format!("{}{}_{}", name, renderer_suffix, index), page);
        }
    }

    if STORE.is_some() {
        write_render_to_store(&name, pdf);
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
