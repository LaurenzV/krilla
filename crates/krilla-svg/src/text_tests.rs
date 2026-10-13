use std::sync::Arc;

use krilla::geom::Size;
use krilla::page::PageSettings;
use krilla::{Document, SerializeSettings};

use crate::{SurfaceExt, SvgSettings};

#[test]
fn svg_words_remain_contiguous_in_extracted_text() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80">
        <text x="10" y="30" font-family="Noto Sans" font-size="18">Vector table 0.125</text>
    </svg>"#;
    let text = extract_svg_text(svg);
    assert!(text.contains("Vector table 0.125"), "{text:?}");
}

#[test]
fn svg_positioned_and_rotated_words_remain_contiguous() {
    for content in [
        r#"<text x="10 22 34 46 58 70" y="30">Amount</text>"#,
        r#"<text x="10" y="30" transform="rotate(12 10 30)">Amount</text>"#,
        r#"<text x="10 22 34 46 58 70" y="30 31 32 33 34 35">Amount</text>"#,
    ] {
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80"><g font-family="Noto Sans" font-size="18">{content}</g></svg>"#
        );
        let text = extract_svg_text(svg.as_bytes());
        assert!(text.contains("Amount"), "{content}: {text:?}");
    }
}

#[test]
fn svg_unicode_words_keep_their_original_text() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80"><text x="10" y="30" font-family="Noto Sans" font-size="18">Καλημέρα Привет</text></svg>"#;
    let text = extract_svg_text(svg.as_bytes());
    assert!(text.contains("Καλημέρα Привет"), "{text:?}");
}

#[test]
fn svg_positioned_italic_words_remain_contiguous_in_extracted_text() {
    for word in ["Beta", "Delta"] {
        let positions = (0..word.len())
            .map(|index| (10 + 12 * index).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80"><text x="{positions}" y="30" font-family="Noto Sans" font-size="18" font-style="italic">{word}</text></svg>"#
        );
        let text = extract_svg_text(svg.as_bytes());
        assert!(text.contains(word), "{word}: {text:?}");
    }
}

fn extract_svg_text(svg: &[u8]) -> String {
    let pdf = render_svg_pdf(svg, SvgSettings::default(), SerializeSettings::default());
    pdf_extract::extract_text_from_mem(&pdf).unwrap()
}

fn render_svg_pdf(svg: &[u8], svg_settings: SvgSettings, settings: SerializeSettings) -> Vec<u8> {
    render_svg_pdf_on_surface(svg, svg_settings, settings, false)
}

fn render_svg_pdf_on_surface(
    svg: &[u8],
    svg_settings: SvgSettings,
    settings: SerializeSettings,
    in_form: bool,
) -> Vec<u8> {
    let tree = usvg::Tree::from_data(
        svg,
        &usvg::Options {
            fontdb: Arc::new(test_fontdb()),
            ..Default::default()
        },
    )
    .unwrap();
    let mut document = Document::new_with(settings);
    let size = Size::from_wh(300.0, 80.0).unwrap();
    let mut page = document.start_page_with(PageSettings::new(size));
    let mut surface = page.surface();
    if in_form {
        let mut builder = surface.stream_builder();
        let mut sub_surface = builder.surface();
        sub_surface.draw_svg(&tree, size, svg_settings);
        sub_surface.finish();
        let graphic = krilla::graphic::Graphic::new(builder.finish(), true);
        surface.draw_graphic(graphic);
    } else {
        surface.draw_svg(&tree, size, svg_settings);
    }
    surface.finish();
    page.finish();
    document.finish().unwrap()
}

#[test]
fn hidden_spans_do_not_inject_unpainted_source_text() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80">
        <text x="10" y="30" font-family="Noto Sans" font-size="18">Before <tspan visibility="hidden">secret</tspan><tspan>after</tspan></text>
    </svg>"#;
    let text = extract_svg_text(svg);
    assert_eq!(text.trim(), "Before after");
}

#[test]
fn style_and_cluster_boundaries_preserve_logical_source_text() {
    for (content, expected) in [
        (
            r#"<text x="10" y="30">of<tspan font-style="italic">fi</tspan>ce</text>"#,
            "office",
        ),
        (
            r#"<text x="10" y="30" font-variant="small-caps">Office</text>"#,
            "Office",
        ),
        (
            r#"<text x="10" y="30">a&#x301; office</text>"#,
            "a\u{301} office",
        ),
    ] {
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80"><g font-family="Noto Sans" font-size="18">{content}</g></svg>"#
        );
        let text = extract_svg_text(svg.as_bytes());
        assert_eq!(text.trim(), expected, "{content}");
    }
}

fn test_fontdb() -> fontdb::Database {
    let mut db = fontdb::Database::new();
    db.load_font_data(include_bytes!("../../../assets/svg_fonts/NotoSans-Regular.ttf").to_vec());
    db.load_font_data(include_bytes!("../../../assets/svg_fonts/NotoSans-Italic.ttf").to_vec());
    db
}

#[test]
fn outlined_svg_text_remains_unsearchable() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80">
        <text x="10" y="30" font-family="Noto Sans" font-size="18">Outlined label</text>
    </svg>"#;
    let pdf = render_svg_pdf(
        svg,
        SvgSettings {
            embed_text: false,
            ..Default::default()
        },
        SerializeSettings::default(),
    );
    assert!(pdf_extract::extract_text_from_mem(&pdf)
        .unwrap()
        .trim()
        .is_empty());
}

#[test]
fn pdf14_skips_replacement_metadata_but_still_draws_text() {
    use krilla::configure::{ConfigurationBuilder, PdfVersion};
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80">
        <text x="10" y="30" font-family="Noto Sans" font-size="18">Legacy</text>
    </svg>"#;
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_version(PdfVersion::Pdf14)
            .finish()
            .unwrap(),
        compress_content_streams: false,
        ..Default::default()
    };
    let pdf = render_svg_pdf(svg, SvgSettings::default(), settings);
    assert!(!pdf
        .windows(b"/ActualText".len())
        .any(|window| window == b"/ActualText"));
    assert_eq!(
        pdf_extract::extract_text_from_mem(&pdf)
            .unwrap()
            .split_whitespace()
            .collect::<String>(),
        "Legacy"
    );
}

#[test]
fn replacement_scopes_work_in_forms_and_inside_page_tags() {
    use krilla::geom::PathBuilder;
    use krilla::graphic::Graphic;
    use krilla::tagging::ContentTag;

    for in_form in [false, true] {
        let mut document = Document::new();
        let mut page = document.start_page_with(PageSettings::from_wh(100.0, 100.0).unwrap());
        let mut surface = page.surface();
        surface.start_tagged(ContentTag::Other);
        let draw_replacement = |surface: &mut krilla::surface::Surface| {
            let returned = surface.with_actual_text("Diagram", |surface| {
                surface.with_actual_text("inner", |surface| {
                    let mut path = PathBuilder::new();
                    path.move_to(10.0, 10.0);
                    path.line_to(30.0, 10.0);
                    path.line_to(20.0, 30.0);
                    path.close();
                    surface.draw_path(&path.finish().unwrap());
                });
                42
            });
            assert_eq!(returned, 42);
        };
        if in_form {
            let mut builder = surface.stream_builder();
            let mut sub_surface = builder.surface();
            draw_replacement(&mut sub_surface);
            sub_surface.finish();
            let graphic = Graphic::new(builder.finish(), true);
            surface.draw_graphic(graphic);
        } else {
            draw_replacement(&mut surface);
        }
        surface.end_tagged();
        surface.finish();
        page.finish();
        let pdf = document.finish().unwrap();
        assert_eq!(
            pdf_extract::extract_text_from_mem(&pdf).unwrap().trim(),
            "Diagram"
        );
    }
}

#[test]
#[ignore = "requires Poppler pdftotext"]
fn poppler_preserves_separate_svg_label_positions() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80">
        <g font-family="Noto Sans" font-size="18">
            <text x="10" y="30">Sample</text>
            <text x="170" y="30">Count</text>
        </g>
    </svg>"#;
    for in_form in [false, true] {
        let pdf = render_svg_pdf_on_surface(
            svg,
            SvgSettings::default(),
            SerializeSettings::default(),
            in_form,
        );
        let mut process = Command::new("pdftotext")
            .args(["-bbox", "-", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Poppler pdftotext must be installed for this integration test");
        process.stdin.take().unwrap().write_all(&pdf).unwrap();
        let output = process.wait_with_output().unwrap();
        assert!(output.status.success());
        let bounds = String::from_utf8(output.stdout).unwrap();
        for (word, expected_x, minimum_width) in [("Sample", 10.0, 40.0), ("Count", 170.0, 35.0)] {
            let end = bounds
                .find(&format!(">{word}</word>"))
                .unwrap_or_else(|| panic!("word must extract separately: {word}; {bounds}"));
            let start = bounds[..end].rfind("<word ").unwrap();
            let attributes = &bounds[start..end];
            let value = attributes
                .split_once("xMin=\"")
                .unwrap()
                .1
                .split('"')
                .next()
                .unwrap();
            let actual_x: f64 = value.parse().unwrap();
            assert!(
                (actual_x - expected_x).abs() < 0.5,
                "{word}: expected x={expected_x}, got {actual_x}; {bounds}"
            );
            let maximum: f64 = attributes
                .split_once("xMax=\"")
                .unwrap()
                .1
                .split('"')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            assert!(
                maximum - actual_x > minimum_width,
                "replacement must cover the painted word: {word}; {bounds}"
            );
            let coordinate = |name: &str| -> f64 {
                attributes
                    .split_once(&format!("{name}=\""))
                    .unwrap()
                    .1
                    .split('"')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap()
            };
            assert!(
                coordinate("yMax") - coordinate("yMin") > 10.0,
                "replacement must retain the painted font height: {word}; {bounds}"
            );
        }
    }
}
