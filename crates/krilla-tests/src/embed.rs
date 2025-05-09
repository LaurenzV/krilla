use krilla::configure::ValidationError;
use krilla::embed::{AssociationKind, EmbedError, EmbeddedFile};
use krilla::error::KrillaError;
use krilla::metadata::{DateTime, Metadata};
use krilla::tagging::TagTree;
use krilla_macros::snapshot;

use crate::{metadata_1, Document};
use crate::{settings_13, settings_23, ASSETS_PATH};

pub(crate) fn file_1() -> EmbeddedFile {
    let data = std::fs::read(ASSETS_PATH.join("emojis.txt")).unwrap();
    EmbeddedFile {
        path: "emojis.txt".to_string(),
        mime_type: Some("text/txt".to_string()),
        description: Some("The description of the file.".to_string()),
        association_kind: AssociationKind::Supplement,
        data: data.into(),
        compress: Some(false),
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
        compress: Some(false),
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
        compress: Some(false),
        location: None,
    }
}

fn file_4() -> EmbeddedFile {
    let data = std::fs::read(ASSETS_PATH.join("images/rgb8.gif")).unwrap();

    EmbeddedFile {
        path: "rgb8.gif".to_string(),
        mime_type: Some("image/gif".to_string()),
        description: Some("A nice gif.".to_string()),
        association_kind: AssociationKind::Unspecified,
        data: data.into(),
        compress: Some(false),
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
    file.compress = Some(true);

    d.embed_file(file);
}

#[snapshot(document)]
fn embedded_file_with_auto_compression_success(d: &mut Document) {
    let mut file = file_1();
    file.compress = None;

    d.embed_file(file);
}

#[snapshot(document)]
fn embedded_file_with_auto_compression_fail(d: &mut Document) {
    let mut file = file_4();
    file.compress = None;

    d.embed_file(file);
}

#[snapshot(document)]
fn embedded_file_multiple(d: &mut Document) {
    let f1 = file_1();
    let f2 = file_2();
    let f3 = file_3();

    d.embed_file(f1);
    d.embed_file(f2);
    d.embed_file(f3);
}

pub(crate) fn embedded_file_impl(d: &mut Document) {
    let metadata = metadata_1();
    d.set_metadata(metadata);
    let f1 = file_1();
    d.embed_file(f1);
}

#[snapshot(document, settings_25)]
fn embedded_file_pdf_20(d: &mut Document) {
    // Technically PDF 2.0 supports associated files, but we only use them for PDF/A-3.
    embedded_file_impl(d)
}

#[test]
fn embedded_file_duplicate() {
    let mut d = Document::new();
    let f1 = file_1();
    let mut f2 = file_2();
    f2.path = f1.path.clone();

    assert!(d.embed_file(f1).is_some());
    assert!(d.embed_file(f2).is_none());
}

#[test]
fn embedded_file_pdf_a2() {
    let mut d = Document::new_with(settings_13());
    let metadata = metadata_1();
    d.set_metadata(metadata);
    d.set_tag_tree(TagTree::new());

    let mut f1 = file_1();
    f1.description = None;
    d.embed_file(f1);

    assert_eq!(
        d.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::EmbeddedFile(EmbedError::Existence, None),
        ]))
    )
}
