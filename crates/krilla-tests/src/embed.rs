use krilla::configure::ValidationError;
use krilla::embed::{AssociationKind, EmbedError, EmbeddedFile};
use krilla::error::KrillaError;
use krilla::metadata::{DateTime, Metadata};
use krilla::tagging::TagTree;
use krilla_macros::snapshot;

use crate::Document;
use crate::{settings_13, settings_23, ASSETS_PATH};

fn file_1() -> EmbeddedFile {
    let data = std::fs::read(ASSETS_PATH.join("emojis.txt")).unwrap();
    EmbeddedFile {
        path: "emojis.txt".to_string(),
        mime_type: Some("text/txt".to_string()),
        description: Some("The description of the file.".to_string()),
        association_kind: AssociationKind::Supplement,
        data: data.into(),
        compress: false,
        location: None,
    }
}

fn file_2() -> EmbeddedFile {
    let data = std::fs::read(ASSETS_PATH.join("svgs/resvg_structure_svg_nested_svg_with_rect.svg"))
        .unwrap();
    EmbeddedFile {
        path: "image.svg".to_string(),
        mime_type: Some("image/svg+xml".to_string()),
        description: Some("A nice SVG image!".to_string()),
        association_kind: AssociationKind::Supplement,
        data: data.into(),
        compress: false,
        location: None,
    }
}

fn file_3() -> EmbeddedFile {
    let data = std::fs::read(ASSETS_PATH.join("images/rgb8.png")).unwrap();

    EmbeddedFile {
        path: "rgb8.png".to_string(),
        mime_type: Some("image/png".to_string()),
        description: Some("A nice picture.".to_string()),
        association_kind: AssociationKind::Unspecified,
        data: data.into(),
        compress: false,
        location: None,
    }
}

#[snapshot(document)]
fn embedded_file(d: &mut Document) {
    let file = file_1();
    d.embed_file(file);
}

#[snapshot(document)]
fn embedded_file_with_compression(d: &mut Document) {
    let mut file = file_1();
    file.compress = true;

    d.embed_file(file);
}

#[snapshot(document)]
fn multiple_embedded_files(d: &mut Document) {
    let f1 = file_1();
    let f2 = file_2();
    let f3 = file_3();

    d.embed_file(f1);
    d.embed_file(f2);
    d.embed_file(f3);
}

fn embedded_file_impl(d: &mut Document) {
    let metadata = Metadata::new()
        .modification_date(DateTime::new(2001))
        .language("en".to_string());
    d.set_metadata(metadata);
    let f1 = file_1();
    d.embed_file(f1);
}

#[snapshot(document, settings_23)]
fn validation_pdf_a3_with_embedded_file(d: &mut Document) {
    embedded_file_impl(d)
}

#[snapshot(document, settings_27)]
fn validation_pdf_a4f_with_embedded_file(d: &mut Document) {
    embedded_file_impl(d)
}

#[snapshot(document, settings_25)]
fn pdf_20_with_embedded_file(d: &mut Document) {
    // Technically PDF 2.0 supports associated files, but we only use them for PDF/A-3.
    embedded_file_impl(d)
}

#[test]
fn duplicate_embedded_file() {
    let mut d = Document::new();
    let f1 = file_1();
    let mut f2 = file_2();
    f2.path = f1.path.clone();

    assert!(d.embed_file(f1).is_some());
    assert!(d.embed_file(f2).is_none());
}

#[test]
fn pdf_a3_missing_fields() {
    let mut d = Document::new_with(settings_23());
    let mut f1 = file_1();
    f1.description = None;
    d.embed_file(f1);

    assert_eq!(
        d.finish(),
        Err(KrillaError::ValidationError(vec![
            ValidationError::EmbeddedFile(EmbedError::MissingDate, None),
            ValidationError::EmbeddedFile(EmbedError::MissingDescription, None)
        ]))
    )
}

#[test]
fn pdf_a2_embedded_file() {
    let mut d = Document::new_with(settings_13());
    let metadata = Metadata::new().language("en".to_string());
    d.set_metadata(metadata);
    d.set_tag_tree(TagTree::new());

    let mut f1 = file_1();
    f1.description = None;
    d.embed_file(f1);

    assert_eq!(
        d.finish(),
        Err(KrillaError::ValidationError(vec![
            ValidationError::EmbeddedFile(EmbedError::Existence, None),
        ]))
    )
}
