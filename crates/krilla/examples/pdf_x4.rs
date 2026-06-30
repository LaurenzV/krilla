//! A minimal PDF/X-4 example using an embedded output intent and live transparency.
//!
//! The bundled CMYK profile is a compact synthetic fixture that keeps the
//! example PDF small. Replace it with a real press/output ICC profile for
//! production work.

use std::path;
use std::path::PathBuf;

use krilla::color::rgb;
use krilla::configure::{ConfigurationBuilder, Prepress};
use krilla::geom::{Path, PathBuilder, Rect};
use krilla::icc::ICCProfile;
use krilla::metadata::{DateTime, Metadata};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::Fill;
use krilla::{Document, SerializeSettings};

fn rect_path(x: f32, y: f32, width: f32, height: f32) -> Path {
    let mut builder = PathBuilder::new();
    builder.move_to(x, y);
    builder.line_to(x + width, y);
    builder.line_to(x + width, y + height);
    builder.line_to(x, y + height);
    builder.close();
    builder.finish().unwrap()
}

fn main() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../");
    let icc_path = workspace_root.join("assets/icc/krilla-generic-cmyk-v2.icc");
    let icc_data = std::fs::read(&icc_path).unwrap();
    let cmyk_profile = ICCProfile::new(&icc_data).unwrap();

    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X4)
            .finish()
            .unwrap(),
        cmyk_profile: Some(cmyk_profile),
        ..SerializeSettings::default()
    };

    let mut document = Document::new_with(settings);
    document.set_metadata(
        Metadata::new()
            .title("PDF/X-4 Example".to_string())
            .language("en".to_string())
            .creation_date(DateTime::new(2026)),
    );

    let trim_box = Rect::from_xywh(0.0, 0.0, 200.0, 200.0).unwrap();
    let page_settings = PageSettings::from_wh(200.0, 200.0)
        .unwrap()
        .with_trim_box(Some(trim_box));
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(Fill {
        paint: rgb::Color::new(250, 235, 210).into(),
        ..Default::default()
    }));
    surface.draw_path(&rect_path(20.0, 20.0, 120.0, 120.0));

    // PDF/X-4 allows live transparency, unlike PDF/X-1a and PDF/X-3.
    surface.set_fill(Some(Fill {
        paint: rgb::Color::new(40, 110, 235).into(),
        opacity: NormalizedF32::new(0.55).unwrap(),
        ..Default::default()
    }));
    surface.draw_path(&rect_path(70.0, 55.0, 110.0, 110.0));

    surface.finish();
    page.finish();

    let pdf = document.finish().unwrap();
    let path = path::absolute("pdf_x4.pdf").unwrap();
    eprintln!("Saved PDF to '{}'", path.display());
    std::fs::write(path, &pdf).unwrap();
}
