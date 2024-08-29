use crate::font::Font;
use crate::stream::Glyph;
use difference::{Changeset, Difference};
use image::{Rgba, RgbaImage};
use rustybuzz::{Direction, UnicodeBuffer};
use sitro::{
    render_ghostscript, render_mupdf, render_pdfbox, render_pdfium, render_pdfjs, render_poppler,
    render_quartz, RenderOptions, RenderedDocument, Renderer,
};
use skrifa::GlyphId;
use std::cmp::max;
use std::path::PathBuf;
use std::cell::LazyCell;
use tiny_skia_path::{Path, PathBuilder, Rect};

mod manual;
mod visreg;

const REPLACE: bool = true;
const STORE: bool = true;

const SNAPSHOT_PATH: LazyCell<PathBuf> = LazyCell::new(|| {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    std::fs::create_dir_all(&path).unwrap();
    path
});

const FONT_PATH: LazyCell<PathBuf> = LazyCell::new(|| {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fonts")
});

pub fn rect_path(x1: f32, y1: f32, x2: f32, y2: f32) -> Path {
    let mut builder = PathBuilder::new();
    builder.push_rect(Rect::from_ltrb(x1, y1, x2, y2).unwrap());
    builder.finish().unwrap()
}

pub fn load_font(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fonts")
        .join(name);
    std::fs::read(&path).unwrap()
}

fn snapshot_path(name: &str) -> PathBuf {
    SNAPSHOT_PATH.clone().join(format!("{}.txt", name))
}

fn snapshot_store_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/store/snapshots");

    std::fs::create_dir_all(&path).unwrap();

    path.push(format!("{}.pdf", name));
    path
}

pub fn check_snapshot(name: &str, content: &[u8], storable: bool) {
    let path = snapshot_path(name);

    if STORE && storable {
        let store_path = snapshot_store_path(name);
        std::fs::write(&store_path, &content).unwrap();
    }

    if !path.exists() {
        std::fs::write(&path, &content).unwrap();
        panic!("new snapshot created");
    }

    let actual = std::fs::read(&path).unwrap();

    if REPLACE && &actual != content {
        std::fs::write(&path, content).unwrap();
        panic!("test was replaced");
    }

    let changeset = Changeset::new(
        &String::from_utf8_lossy(content),
        &String::from_utf8_lossy(&actual),
        "\n",
    );

    for diff in changeset.diffs {
        match diff {
            Difference::Same(ref x) => {
                println!(" {}", x);
            }
            Difference::Add(ref x) => {
                println!("+++++++++++++++++++\n{}\n+++++++++++++++++++", x);
            }
            Difference::Rem(ref x) => {
                println!("-------------------\n{}\n-------------------", x);
            }
        }
    }

    assert_eq!(changeset.distance, 0);
}

pub fn store_manual(name: &str, data: &[u8]) {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/store/manual");

    std::fs::create_dir_all(&path).unwrap();

    let pdf_path = path.join(format!("{}.pdf", name));
    let txt_path = path.join(format!("{}.txt", name));

    std::fs::write(pdf_path, data).unwrap();
    std::fs::write(txt_path, data).unwrap();
}

pub fn simple_shape(text: &str, dir: Direction, font: Font, size: f32) -> Vec<Glyph> {
    let data = font.font_data();
    let rb_font = rustybuzz::Face::from_slice(data.as_ref().as_ref(), 0).unwrap();

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(dir);

    let output = rustybuzz::shape(&rb_font, &[], buffer);

    let positions = output.glyph_positions();
    let infos = output.glyph_infos();

    let mut glyphs = vec![];

    for i in 0..output.len() {
        let pos = positions[i];
        let start_info = infos[i];

        let start = start_info.cluster as usize;

        let end = if dir == Direction::LeftToRight {
            let mut e = i.checked_add(1);
            loop {
                if let Some(index) = e {
                    if let Some(end_info) = infos.get(index) {
                        if end_info.cluster == start_info.cluster {
                            e = index.checked_add(1);
                            continue;
                        }
                    }
                }

                break;
            }

            e
        } else {
            let mut e = i.checked_sub(1);
            loop {
                if let Some(index) = e {
                    if let Some(end_info) = infos.get(index) {
                        if end_info.cluster == start_info.cluster {
                            e = index.checked_sub(1);
                        } else {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }

            e
        }
        .and_then(|last| infos.get(last))
        .map_or(text.len(), |info| info.cluster as usize);

        glyphs.push(Glyph::new(
            GlyphId::new(start_info.glyph_id),
            (pos.x_advance as f32 / font.units_per_em() as f32) * size,
            (pos.x_offset as f32 / font.units_per_em() as f32) * size,
            (pos.y_offset as f32 / font.units_per_em() as f32) * size,
            start..end,
            size,
        ));
    }

    glyphs
}

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
