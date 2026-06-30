//! A minimal PDF/X-1a example using an embedded CMYK output profile.
//!
//! The bundled CMYK profile is a compact synthetic fixture that keeps the
//! example PDF small. Replace it with a real press/output ICC profile for
//! production work.

use std::path;
use std::path::PathBuf;

use krilla::color::cmyk;
use krilla::configure::{ConfigurationBuilder, Prepress};
use krilla::geom::{PathBuilder, Rect};
use krilla::icc::ICCProfile;
use krilla::metadata::{DateTime, Metadata};
use krilla::page::PageSettings;
use krilla::paint::Fill;
use krilla::{Document, SerializeSettings};

fn main() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../");
    let icc_path = workspace_root.join("assets/icc/krilla-generic-cmyk-v2.icc");
    let icc_data = std::fs::read(&icc_path).unwrap();
    let cmyk_profile = ICCProfile::new(&icc_data).unwrap();

    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X1A)
            .finish()
            .unwrap(),
        cmyk_profile: Some(cmyk_profile),
        ..SerializeSettings::default()
    };

    let mut document = Document::new_with(settings);
    document.set_metadata(
        Metadata::new()
            .title("PDF/X-1a Example".to_string())
            .language("en".to_string())
            .creation_date(DateTime::new(2026)),
    );

    let trim_box = Rect::from_xywh(0.0, 0.0, 200.0, 200.0).unwrap();
    let page_settings = PageSettings::from_wh(200.0, 200.0)
        .unwrap()
        .with_trim_box(Some(trim_box));
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    let path = {
        let mut builder = PathBuilder::new();
        builder.move_to(20.0, 20.0);
        builder.line_to(180.0, 20.0);
        builder.line_to(180.0, 180.0);
        builder.line_to(20.0, 180.0);
        builder.close();
        builder.finish().unwrap()
    };

    surface.set_fill(Some(Fill {
        paint: cmyk::Color::new(255, 64, 0, 0).into(),
        ..Default::default()
    }));
    surface.draw_path(&path);
    surface.finish();
    page.finish();

    let pdf = document.finish().unwrap();
    let path = path::absolute("pdf_x1a.pdf").unwrap();
    eprintln!("Saved PDF to '{}'", path.display());
    std::fs::write(path, &pdf).unwrap();
}
