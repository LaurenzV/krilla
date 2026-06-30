//! A minimal PDF/X-4p example using an external output-profile reference.
//!
//! PDF/X-4p (ISO 15930-7) is like PDF/X-4, but the output intent ICC profile
//! is referenced externally via `DestOutputProfileRef` instead of being
//! embedded in the document.
//!
//! # Important: production use
//!
//! The CMYK profile bundled at `assets/icc/krilla-generic-cmyk-v2.icc` is a
//! *synthetic* fixture designed to keep example output small. It is **not** a
//! press characterisation profile. For real prepress work you must substitute
//! the characterisation profile that matches your workflow (e.g. FOGRA51,
//! FOGRA52, GRACoL2013, SWOP2013) and update the `urls` entry to point at the
//! canonical external location of that profile.

use std::path;
use std::path::PathBuf;

use krilla::color::rgb;
use krilla::configure::{ConfigurationBuilder, Prepress};
use krilla::geom::{PathBuilder, Rect};
use krilla::icc::ICCProfile;
use krilla::metadata::{DateTime, Metadata};
use krilla::page::PageSettings;
use krilla::paint::Fill;
use krilla::{Document, ExternalOutputProfile, SerializeSettings};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../");
    let icc_path = workspace_root.join("assets/icc/krilla-generic-cmyk-v2.icc");
    let icc_data = std::fs::read(&icc_path)
        .map_err(|e| format!("reading {} failed: {e}", icc_path.display()))?;
    let cmyk_profile = ICCProfile::new(&icc_data)
        .ok_or("bundled CMYK profile failed to parse as ICC v2 4-component profile")?;

    let external_profile = ExternalOutputProfile::cmyk(
        cmyk_profile,
        vec!["https://example.com/profiles/krilla-generic-cmyk-v2.icc".to_string()],
        "krilla-generic-cmyk-v2".to_string(),
        "Synthetic CMYK v2 test profile bundled with krilla".to_string(),
    )?
    .with_output_condition("Synthetic CMYK (krilla-generic-cmyk-v2)".to_string());

    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X4P)
            .finish()
            .unwrap(),
        external_output_profile: Some(external_profile),
        ..SerializeSettings::default()
    };

    let mut document = Document::new_with(settings);
    document.set_metadata(
        Metadata::new()
            .title("PDF/X-4p Example".to_string())
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
        paint: rgb::Color::new(230, 80, 42).into(),
        ..Default::default()
    }));
    surface.draw_path(&path);
    surface.finish();
    page.finish();

    let pdf = document
        .finish()
        .map_err(|e| format!("PDF serialisation failed: {e:?}"))?;
    let out = path::absolute("pdf_x4p.pdf")?;
    eprintln!("Saved PDF to '{}'", out.display());
    std::fs::write(out, &pdf)?;
    Ok(())
}
