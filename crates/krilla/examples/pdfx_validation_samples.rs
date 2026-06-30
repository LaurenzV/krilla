//! Generates one conformant PDF/X sample per level (X-1a, X-3, X-4, X-4p, X-6,
//! X-6p) using the bundled eciCMYK output profile and embedded text, for
//! validation against an external PDF/X checker. Usage:
//!
//! `cargo run --example pdfx_validation_samples -- <out-dir>`

use std::path::PathBuf;
use std::sync::Arc;

use krilla::color::cmyk;
use krilla::configure::{ConfigurationBuilder, Prepress};
use krilla::geom::{PathBuilder, Point, Rect};
use krilla::icc::ICCProfile;
use krilla::metadata::{DateTime, Metadata};
use krilla::page::PageSettings;
use krilla::paint::Fill;
use krilla::text::{Font, TextDirection};
use krilla::{Document, ExternalOutputProfile, SerializeSettings};

fn rect_path(x: f32, y: f32, w: f32, h: f32) -> krilla::geom::Path {
    let mut b = PathBuilder::new();
    b.move_to(x, y);
    b.line_to(x + w, y);
    b.line_to(x + w, y + h);
    b.line_to(x, y + h);
    b.close();
    b.finish().unwrap()
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../");
    let icc = std::fs::read(root.join("assets/icc/eciCMYK_v2.icc")).unwrap();
    let font = Font::new(
        Arc::new(std::fs::read(root.join("assets/fonts/NotoSans-Regular.ttf")).unwrap()).into(),
        0,
    )
    .unwrap();

    let out = std::env::args()
        .nth(1)
        .expect("usage: pdfx_validation_samples <out-dir>");
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();

    let levels = [
        ("x1a", Prepress::X1A, false),
        ("x3", Prepress::X3, false),
        ("x4", Prepress::X4, false),
        ("x4p", Prepress::X4P, true),
        ("x6", Prepress::X6, false),
        ("x6p", Prepress::X6P, true),
    ];

    for (name, level, external) in levels {
        let config = ConfigurationBuilder::new()
            .with_prepress_validator(level)
            .finish()
            .unwrap();

        let cmyk_profile = ICCProfile::new(&icc).unwrap();
        let settings = if external {
            SerializeSettings {
                configuration: config,
                external_output_profile: Some(
                    ExternalOutputProfile::cmyk(
                        cmyk_profile,
                        vec!["https://www.eci.org/_media/downloads/icc_profiles_from_eci/ecicmyk.zip"
                            .to_string()],
                        "eciCMYK".to_string(),
                        "eciCMYK (FOGRA53) reference profile".to_string(),
                    )
                    .unwrap(),
                ),
                ..SerializeSettings::default()
            }
        } else {
            SerializeSettings {
                configuration: config,
                cmyk_profile: Some(cmyk_profile),
                ..SerializeSettings::default()
            }
        };

        let mut document = Document::new_with(settings);
        document.set_metadata(
            Metadata::new()
                .title(format!("krilla PDF/{} sample", name.to_uppercase()))
                .language("en".to_string())
                .creation_date(DateTime::new(2026)),
        );

        let trim = Rect::from_xywh(0.0, 0.0, 300.0, 200.0).unwrap();
        let page_settings = PageSettings::from_wh(300.0, 200.0)
            .unwrap()
            .with_trim_box(Some(trim));
        let mut page = document.start_page_with(page_settings);
        let mut surface = page.surface();

        // CMYK content only, so it is valid for the CMYK-only PDF/X-1a too.
        surface.set_fill(Some(Fill {
            paint: cmyk::Color::new(15, 70, 95, 0).into(),
            ..Default::default()
        }));
        surface.draw_path(&rect_path(20.0, 20.0, 110.0, 160.0));

        surface.set_fill(Some(Fill {
            paint: cmyk::Color::new(0, 0, 0, 255).into(),
            ..Default::default()
        }));
        surface.draw_text(
            Point::from_xy(150.0, 100.0),
            font.clone(),
            18.0,
            &format!("PDF/{}", name.to_uppercase()),
            false,
            TextDirection::Auto,
        );

        surface.finish();
        page.finish();

        let pdf = document
            .finish()
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let path = out.join(format!("krilla_pdf_{name}.pdf"));
        std::fs::write(&path, &pdf).unwrap();
        eprintln!("wrote {} ({} bytes)", path.display(), pdf.len());
    }
}
