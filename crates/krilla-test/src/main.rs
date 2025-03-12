//! Test utilities.

use std::cmp::max;
use std::env;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, OnceLock};

use difference::{Changeset, Difference};
use image::{load_from_memory, DynamicImage, GenericImageView, Rgba, RgbaImage};
use once_cell::sync::Lazy;
use oxipng::{InFile, OutFile};
use sitro::{
    render_ghostscript, render_mupdf, render_pdfbox, render_pdfium, render_poppler, render_quartz,
    RenderOptions, RenderedDocument, RenderedPage, Renderer,
};
use skrifa::instance::{LocationRef, Size};
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::{NormalizedF32, Path, PathBuilder, Point, Rect, Transform};

use krilla::action::LinkAction;
use krilla::annotation::{Annotation, LinkAnnotation, Target};
use krilla::color::{cmyk, luma, rgb, ICCProfile};
use krilla::configure::{Configuration, PdfVersion, Validator};
use krilla::document::{Document, PageSettings};
use krilla::font::{Font, GlyphUnits, KrillaGlyph};
use krilla::image::{BitsPerComponent, CustomImage, Image, ImageColorspace};
use krilla::mask::{Mask, MaskType};
use krilla::paint::{Stop, Stops};
use krilla::path::{Fill, Stroke};
use krilla::stream::Stream;
use krilla::stream::StreamBuilder;
use krilla::surface::Surface;
use krilla::Data;
use krilla::{SerializeSettings, SvgSettings};

mod annotation;
mod color;
mod font;
mod validate;
mod destination;

const REPLACE: Option<&str> = option_env!("REPLACE");
const STORE: Option<&str> = option_env!("STORE");

pub(crate) static WORKSPACE_PATH: Lazy<PathBuf> =
    Lazy::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../"));

pub(crate) static ASSETS_PATH: LazyLock<PathBuf> = LazyLock::new(|| WORKSPACE_PATH.join("assets"));

static SNAPSHOT_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = WORKSPACE_PATH.join("refs/snapshots");
    let _ = std::fs::create_dir_all(&path);
    path
});

static VISREG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = WORKSPACE_PATH.join("refs/visreg");
    let _ = std::fs::create_dir_all(&path);
    path
});

pub static SVGS_PATH: LazyLock<PathBuf> = LazyLock::new(|| WORKSPACE_PATH.join("assets/svgs"));

static DIFFS_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = WORKSPACE_PATH.join("diffs");
    let _ = std::fs::remove_dir_all(&path);
    let _ = std::fs::create_dir_all(&path);
    path
});

static STORE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let path = WORKSPACE_PATH.join("store");
    let _ = std::fs::remove_dir_all(&path);
    let _ = std::fs::create_dir_all(&path);
    path
});

static FONT_PATH: LazyLock<PathBuf> = LazyLock::new(|| WORKSPACE_PATH.join("assets/fonts"));

macro_rules! lazy_font {
    ($name:ident, $path:expr) => {
        pub static $name: LazyLock<Data> =
            LazyLock::new(|| Arc::new(std::fs::read($path).unwrap()).into());
    };
}

#[rustfmt::skip]
lazy_font!(NOTO_SANS, FONT_PATH.join("NotoSans-Regular.ttf"));
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
lazy_font!(SVG_EXTRA, FONT_PATH.join("SVG_extra.ttf"));

#[derive(Clone)]
struct TestImage {
    original_dynamic: Arc<DynamicImage>,
    alpha_channel: OnceLock<Option<Arc<Vec<u8>>>>,
    actual_dynamic: OnceLock<Arc<DynamicImage>>,
    icc: Option<Vec<u8>>,
}

impl TestImage {
    pub fn new(data: Vec<u8>, icc: Option<Vec<u8>>) -> Self {
        let image = image::load_from_memory(&data).unwrap();
        Self {
            original_dynamic: Arc::new(image),
            alpha_channel: OnceLock::new(),
            actual_dynamic: OnceLock::new(),
            icc,
        }
    }
}

impl Hash for TestImage {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.original_dynamic.as_bytes().hash(state);
    }
}

impl CustomImage for TestImage {
    fn color_channel(&self) -> &[u8] {
        self.actual_dynamic
            .get_or_init(|| {
                let dynamic = self.original_dynamic.clone();
                let channel_count = dynamic.color().channel_count();

                match (dynamic.as_ref(), channel_count) {
                    (DynamicImage::ImageLuma8(_), _) => dynamic.clone(),
                    (DynamicImage::ImageRgb8(_), _) => dynamic.clone(),
                    (_, 1 | 2) => Arc::new(DynamicImage::ImageLuma8(dynamic.to_luma8())),
                    _ => Arc::new(DynamicImage::ImageRgb8(dynamic.to_rgb8())),
                }
            })
            .as_bytes()
    }

    fn alpha_channel(&self) -> Option<&[u8]> {
        self.alpha_channel
            .get_or_init(|| {
                self.original_dynamic.color().has_alpha().then(|| {
                    Arc::new(
                        self.original_dynamic
                            .pixels()
                            .map(|(_, _, Rgba([_, _, _, a]))| a)
                            .collect(),
                    )
                })
            })
            .as_ref()
            .map(|v| &***v)
    }

    fn bits_per_component(&self) -> BitsPerComponent {
        BitsPerComponent::Eight
    }

    fn size(&self) -> (u32, u32) {
        self.original_dynamic.dimensions()
    }

    fn icc_profile(&self) -> Option<&[u8]> {
        self.icc.as_deref()
    }

    fn color_space(&self) -> ImageColorspace {
        if self.original_dynamic.color().has_color() {
            ImageColorspace::Rgb
        } else {
            ImageColorspace::Luma
        }
    }
}

fn dummy_glyph(
    glyph_id: GlyphId,
    range: Range<usize>,
    location: Option<krilla::surface::Location>,
) -> KrillaGlyph {
    KrillaGlyph::new(glyph_id, 0.0, 0.0, 0.0, 0.0, range, location)
}

pub fn dummy_text_with_spans() -> (String, Vec<KrillaGlyph>) {
    let text = "Hi.".to_string();
    let glyphs = vec![
        dummy_glyph(GlyphId::new(10), 0..1, Some(3)),
        dummy_glyph(GlyphId::new(0), 1..2, Some(4)),
        dummy_glyph(GlyphId::new(20), 2..3, Some(5)),
    ];

    (text, glyphs)
}

pub fn green_fill(opacity: f32) -> Fill {
    Fill {
        paint: rgb::Color::new(0, 255, 0).into(),
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

pub fn blue_fill(opacity: f32) -> Fill {
    Fill {
        paint: rgb::Color::new(0, 0, 255).into(),
        opacity: NormalizedF32::new(opacity).unwrap(),
        rule: Default::default(),
    }
}

pub fn blue_stroke(opacity: f32) -> Stroke {
    Stroke {
        paint: rgb::Color::new(0, 0, 255).into(),
        opacity: NormalizedF32::new(opacity).unwrap(),
        ..Stroke::default()
    }
}

pub fn red_fill(opacity: f32) -> Fill {
    Fill {
        paint: rgb::Color::new(255, 0, 0).into(),
        opacity: NormalizedF32::new(opacity).unwrap(),
        rule: Default::default(),
    }
}

pub fn red_stroke(opacity: f32, width: f32) -> Stroke {
    Stroke {
        paint: rgb::Color::new(255, 0, 0).into(),
        opacity: NormalizedF32::new(opacity).unwrap(),
        width,
        ..Stroke::default()
    }
}

pub fn purple_fill(opacity: f32) -> Fill {
    Fill {
        paint: rgb::Color::new(128, 0, 128).into(),
        opacity: NormalizedF32::new(opacity).unwrap(),
        rule: Default::default(),
    }
}

pub fn gray_fill(opacity: f32) -> Fill {
    Fill {
        paint: luma::Color::new(127).into(),
        opacity: NormalizedF32::new(opacity).unwrap(),
        rule: Default::default(),
    }
}

pub fn cmyk_fill(opacity: f32) -> Fill {
    Fill {
        paint: cmyk::Color::new(0, 8, 252, 5).into(),
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
    Image::from_png(
        std::fs::read(ASSETS_PATH.join("images").join(name))
            .unwrap()
            .into(),
        false,
    )
    .unwrap()
}

pub fn load_jpg_image(name: &str) -> Image {
    Image::from_jpeg(
        std::fs::read(ASSETS_PATH.join("images").join(name))
            .unwrap()
            .into(),
        false,
    )
    .unwrap()
}

pub fn load_gif_image(name: &str) -> Image {
    Image::from_gif(
        std::fs::read(ASSETS_PATH.join("images").join(name))
            .unwrap()
            .into(),
        false,
    )
    .unwrap()
}

pub fn load_webp_image(name: &str) -> Image {
    Image::from_webp(
        std::fs::read(ASSETS_PATH.join("images").join(name))
            .unwrap()
            .into(),
        false,
    )
    .unwrap()
}

pub fn load_custom_image(name: &str) -> Image {
    Image::from_custom(
        TestImage::new(
            std::fs::read(ASSETS_PATH.join("images").join(name)).unwrap(),
            None,
        ),
        false,
    )
    .unwrap()
}

pub fn load_custom_image_with_icc(name: &str, icc: Vec<u8>) -> Image {
    Image::from_custom(
        TestImage::new(
            std::fs::read(ASSETS_PATH.join("images").join(name)).unwrap(),
            Some(icc),
        ),
        false,
    )
    .unwrap()
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

pub fn check_snapshot(name: &str, actual: &[u8], storable: bool) {
    let path = SNAPSHOT_PATH.join(format!("{}.txt", name));

    if STORE.is_some() && storable {
        write_snapshot_to_store(name, actual);
    }

    if !path.exists() {
        std::fs::write(&path, actual).unwrap();
        panic!("new snapshot created");
    }

    let expected = std::fs::read(&path).unwrap();

    if REPLACE.is_some() && expected != actual {
        std::fs::write(&path, actual).unwrap();
        panic!("test was replaced");
    }

    let changeset = Changeset::new(
        &String::from_utf8_lossy(actual),
        &String::from_utf8_lossy(&expected),
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
    sub_folder: Option<&str>,
    renderer: &Renderer,
    document: RenderedDocument,
    pdf: &[u8],
    ignore_renderer: bool,
) {
    let mut refs_path = VISREG_PATH.clone();

    if let Some(sub_folder) = sub_folder {
        refs_path = refs_path.join(sub_folder);
    }

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
        Renderer::Pdfbox => render_pdfbox(doc, &options).unwrap(),
        Renderer::Ghostscript => render_ghostscript(doc, &options).unwrap(),
        _ => unreachable!(),
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
                _ => {
                    pixel_diff += 1;
                    diff_image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                    diff_image.put_pixel(x + width, y, Rgba([255, 0, 0, 255]));
                }
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
    font_data: Data,
    glyphs: Option<Vec<(GlyphId, String)>>,
    color_cycling: bool,
    allow_color: bool,
    d: &mut Document,
) {
    use krilla::font::KrillaGlyph;
    use krilla::geom::Transform;

    let font = Font::new(font_data, 0, allow_color).unwrap();
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
            Fill {
                paint: color.into(),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            },
            &[KrillaGlyph::new(i, 0.0, 0.0, 0.0, 0.0, 0..text.len(), None)],
            font.clone(),
            &text,
            size as f32,
            GlyphUnits::UserSpace,
            false,
        );
        surface.pop();

        cur_point += size;
    }

    surface.finish();
    builder.finish();
}

pub fn stops_with_1_solid() -> Stops {
    vec![Stop {
        offset: NormalizedF32::new(0.5).unwrap(),
        color: rgb::Color::new(255, 0, 0),
        opacity: NormalizedF32::ONE,
    }]
    .into()
}

pub fn stops_with_2_solid_1() -> Stops {
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
    .into()
}

pub fn stops_with_3_solid_1() -> Stops {
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
    .into()
}

pub fn youtube_link(x: f32, y: f32, w: f32, h: f32) -> Annotation {
    LinkAnnotation::new(
        Rect::from_xywh(x, y, w, h).unwrap(),
        None,
        Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
    )
    .into()
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

pub static FONTDB: Lazy<Arc<fontdb::Database>> = Lazy::new(|| {
    let mut fontdb = fontdb::Database::new();
    fontdb.load_fonts_dir(ASSETS_PATH.join("svg_fonts"));

    fontdb.set_serif_family("Noto Serif");
    fontdb.set_sans_serif_family("Noto Sans");
    fontdb.set_cursive_family("Yellowtail");
    fontdb.set_fantasy_family("Sedgwick Ave Display");
    fontdb.set_monospace_family("Noto Mono");

    Arc::new(fontdb)
});

fn svg_impl(name: &str, renderer: Renderer, ignore_renderer: bool) {
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
    surface.draw_svg(
        &tree,
        tree.size(),
        SvgSettings {
            embed_text: true,
            filter_scale: 2.0,
        },
    );
    surface.finish();
    page.finish();

    let pdf = d.finish().unwrap();
    let rendered = render_document(&pdf, &renderer);
    check_render(
        name,
        Some("svg"),
        &renderer,
        rendered,
        &pdf,
        ignore_renderer,
    );
}

pub fn default() -> SerializeSettings {
    SerializeSettings::default()
}

pub fn settings_1() -> SerializeSettings {
    SerializeSettings {
        ascii_compatible: true,
        compress_content_streams: false,
        no_device_cs: false,
        xmp_metadata: false,
        cmyk_profile: None,
        enable_tagging: true,
        configuration: Configuration::new(),
    }
}

pub fn settings_2() -> SerializeSettings {
    SerializeSettings {
        no_device_cs: true,
        ..settings_1()
    }
}

pub fn settings_5() -> SerializeSettings {
    SerializeSettings {
        xmp_metadata: true,
        ..settings_1()
    }
}

pub fn settings_6() -> SerializeSettings {
    SerializeSettings {
        no_device_cs: true,
        cmyk_profile: Some(
            ICCProfile::new(&std::fs::read(ASSETS_PATH.join("icc/eciCMYK_v2.icc")).unwrap())
                .unwrap(),
        ),
        ..settings_1()
    }
}

pub fn settings_7() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A2_B),
        ..settings_1()
    }
}

pub fn settings_8() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A2_B),
        cmyk_profile: Some(
            ICCProfile::new(&std::fs::read(ASSETS_PATH.join("icc/eciCMYK_v2.icc")).unwrap())
                .unwrap(),
        ),
        ..settings_1()
    }
}

pub fn settings_9() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A2_U),
        ..settings_1()
    }
}

pub fn settings_10() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A3_B),
        ..settings_1()
    }
}

pub fn settings_11() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A3_U),
        ..settings_1()
    }
}

pub fn settings_12() -> SerializeSettings {
    SerializeSettings {
        enable_tagging: false,
        ..settings_1()
    }
}

pub fn settings_13() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A2_A),
        ..settings_1()
    }
}

pub fn settings_14() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A3_A),
        ..settings_1()
    }
}

pub fn settings_15() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::UA1),
        ..settings_1()
    }
}

pub fn settings_16() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_version(PdfVersion::Pdf14),
        xmp_metadata: true,
        ..settings_1()
    }
}

pub fn settings_17() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_version(PdfVersion::Pdf14),
        ..settings_1()
    }
}

pub fn settings_18() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_version(PdfVersion::Pdf14),
        no_device_cs: true,
        ..settings_1()
    }
}

pub fn settings_19() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A1_B),
        ..settings_1()
    }
}

pub fn settings_20() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A1_A),
        ..settings_1()
    }
}

pub fn settings_22() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with(Validator::A2_B, PdfVersion::Pdf14).unwrap(),
        ..settings_1()
    }
}

pub fn settings_23() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A3_B),
        ..settings_1()
    }
}

pub fn settings_24() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A3_A),
        ..settings_1()
    }
}

pub fn settings_25() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_version(PdfVersion::Pdf20),
        ..settings_1()
    }
}

pub fn settings_26() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A4),
        ..settings_1()
    }
}

pub fn settings_27() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A4F),
        ..settings_1()
    }
}

pub fn settings_28() -> SerializeSettings {
    SerializeSettings {
        configuration: Configuration::new_with_validator(Validator::A4E),
        ..settings_1()
    }
}

fn main() {
    panic!("This crate shouldn't be run directly.");
}
