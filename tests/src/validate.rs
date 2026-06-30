use krilla::action::LinkAction;
use krilla::annotation::{Annotation, LinkAnnotation, LinkBorder, Target};
use krilla::color::{cmyk, luma, rgb, separation};
use krilla::configure::validate::VersionedFeature;
use krilla::configure::{
    Accessibility, Archival, ConfigurationBuilder, ConfigurationError, PdfVersion, Prepress,
    ValidationError,
};
use krilla::embed::EmbedError;
use krilla::error::KrillaError;
use krilla::geom::{Point, Rect, Size};
use krilla::icc::ICCProfile;
use krilla::metadata::{DateTime, Metadata};
use krilla::num::NormalizedF32;
use krilla::outline::Outline;
use krilla::page::{Page, PageSettings};
use krilla::paint::{Fill, FillRule, LinearGradient, SpreadMethod, Stop};
use krilla::tagging::{Artifact, ArtifactType, ContentTag, SpanTag, TagGroup, TagKind, TagTree};
use krilla::tagging::{ListNumbering, TableHeaderScope, Tag};
use krilla::text::{Font, TextDirection};
use krilla::text::{GlyphId, KrillaGlyph};
use krilla::ExternalOutputProfile;
use krilla_macros::snapshot;

use crate::embed::{embedded_file_impl, file_1};
use crate::{
    blue_fill, cmyk_fill, dummy_text_with_spans, green_fill, load_jpg_image, load_png_image, loc,
    metadata_1, metadata_2, pdfx_external_output_profile, rect_to_path, red_fill, settings_1,
    settings_13, settings_15, settings_17, settings_19, settings_20, settings_23, settings_24,
    settings_32, settings_33, settings_34, settings_35, settings_36, settings_37, settings_38,
    settings_40, settings_41, settings_42, settings_7, settings_8, settings_9,
    stops_with_2_solid_1, validation_errors, youtube_link, NOTO_SANS,
};
use crate::{Document, SerializeSettings};

fn pdfa_document() -> Document {
    Document::new_with(settings_7())
}

fn pdf_ua1_settings(version: PdfVersion) -> SerializeSettings {
    SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_accessibility_validator(Accessibility::UA1)
            .with_version(version)
            .finish()
            .unwrap(),
        ..settings_1()
    }
}

fn q_nesting_impl(settings: SerializeSettings) -> Document {
    let mut document = Document::new_with(settings);
    let mut page = document.start_page();
    let mut surface = page.surface();

    for _ in 0..29 {
        surface.push_clip_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), &FillRule::NonZero);
    }

    for _ in 0..29 {
        surface.pop();
    }

    surface.finish();
    page.finish();

    document
}

#[snapshot(document, settings_7)]
pub fn validate_pdf_a_q_nesting_28(document: &mut Document) {
    let mut page = document.start_page();
    let mut surface = page.surface();

    for _ in 0..28 {
        surface.push_clip_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), &FillRule::NonZero);
    }

    for _ in 0..28 {
        surface.pop();
    }
}

#[test]
pub fn validate_pdf_a_q_nesting_28() {
    let document = q_nesting_impl(settings_7());
    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::TooHighQNestingLevel]
    );
}

#[test]
pub fn validate_pdf_a_string_length() {
    let mut document = pdfa_document();
    let metadata = Metadata::new()
        .creator("A".repeat(32768))
        .creation_date(DateTime::new(2021));
    document.set_metadata(metadata);
    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::TooLongString]
    );
}

#[snapshot(settings_7)]
fn validate_pdf_a_annotation(page: &mut Page) {
    page.add_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
            Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
        )
        .into(),
    );
}

#[test]
fn validate_pdf_a_postscript() {
    let mut document = pdfa_document();
    let mut page = document.start_page();

    let gradient = LinearGradient {
        x1: 50.0,
        y1: 0.0,
        x2: 150.0,
        y2: 0.0,
        transform: Default::default(),
        spread_method: SpreadMethod::Repeat,
        stops: stops_with_2_solid_1(),
        anti_alias: false,
    };

    let fill = Fill {
        paint: gradient.into(),
        ..Default::default()
    };

    let mut surface = page.surface();

    surface.set_fill(Some(fill));
    surface.draw_path(&rect_to_path(0.0, 0.0, 100.0, 100.0));

    surface.finish();
    page.finish();

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::ContainsPostScript(None)]
    )
}

#[test]
pub fn validate_disabled_q_nesting_28() {
    let document = q_nesting_impl(SerializeSettings::default());
    assert!(document.finish().is_ok());
}

fn cmyk_document_impl(document: &mut Document) {
    let mut page = document.start_page();
    let mut surface = page.surface();

    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
    let fill = cmyk_fill(1.0);
    surface.set_fill(Some(fill));
    surface.draw_path(&path);

    surface.finish();
    page.finish();
}

#[test]
fn validate_pdf_a_missing_cmyk() {
    let mut document = pdfa_document();
    cmyk_document_impl(&mut document);

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::MissingCMYKProfile]
    )
}

#[test]
fn validate_pdf_a_existing_cmyk() {
    let mut document = Document::new_with(settings_8());
    cmyk_document_impl(&mut document);

    assert!(document.finish().is_ok())
}

#[test]
fn validate_pdf_a_notdef_glyph() {
    let mut document = pdfa_document();
    let mut page = document.start_page();
    let mut surface = page.surface();

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();

    surface.draw_text(
        Point::from_xy(0.0, 100.0),
        font.clone(),
        20.0,
        "你",
        false,
        TextDirection::Auto,
    );
    surface.finish();
    page.finish();

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::ContainsNotDefGlyph(
            font,
            None,
            "你".to_string()
        )]
    )
}

#[test]
fn validate_pdfa2u_text_with_location() {
    let mut document = Document::new_with(settings_9());
    let mut page = document.start_page();
    let mut surface = page.surface();

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();
    let (text, glyphs) = dummy_text_with_spans();

    surface.set_location(loc(2));
    surface.set_fill(Some(red_fill(0.1)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 10.0, 10.0));

    surface.draw_glyphs(
        Point::from_xy(0.0, 100.0),
        &glyphs,
        font.clone(),
        &text,
        20.0,
        false,
    );
    surface.finish();
    page.finish();

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::ContainsNotDefGlyph(
            font,
            Some(loc(4)),
            "i".to_string()
        )]
    )
}

#[test]
fn validate_pdfa1b_transparency_with_location() {
    let mut document = Document::new_with(settings_19());
    let mut page = document.start_page();
    let mut surface = page.surface();

    surface.set_location(loc(2));
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 10.0, 10.0));
    surface.set_location(loc(3));
    surface.set_fill(Some(green_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 10.0, 10.0));
    surface.set_location(loc(4));
    surface.set_fill(Some(green_fill(0.9)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 10.0, 10.0));
    surface.set_location(loc(5));
    surface.set_fill(Some(green_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 10.0, 10.0));
    surface.set_location(loc(6));
    surface.set_fill(Some(blue_fill(0.8)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 10.0, 10.0));
    surface.set_location(loc(7));
    surface.set_fill(Some(blue_fill(0.9)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 10.0, 10.0));

    surface.finish();
    page.finish();

    assert_eq!(
        validation_errors(document.finish()),
        vec![
            ValidationError::Transparency(Some(loc(4))),
            ValidationError::Transparency(Some(loc(6))),
            // Note that we don't have 7 here, even though we should in theory. The reason is
            // that since we cache graphics states, only the first time we serialize it will
            // it trigger the validation error. Not optimal, but changing that would be a pain.
        ]
    )
}

fn validate_pdf_full_example(document: &mut Document) {
    let mut page = document.start_page();
    let mut surface = page.surface();

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();

    surface.draw_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "This is some text",
        false,
        TextDirection::Auto,
    );

    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(30.0, 30.0, 70.0, 70.0));

    surface.finish();
    page.finish();

    let metadata = metadata_1();
    document.set_metadata(metadata);
}

pub(crate) fn validate_pdf_tagged_full_example(document: &mut Document) {
    let mut page = document.start_page();
    let mut surface = page.surface();

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();

    let id1 = surface.start_tagged(ContentTag::Span(SpanTag {
        lang: None,
        alt_text: Some("Alt"),
        expanded: Some("Expanded"),
        actual_text: Some("ActualText"),
    }));
    surface.draw_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "This is some text",
        false,
        TextDirection::Auto,
    );
    surface.end_tagged();

    let id2 = surface.start_tagged(ContentTag::Artifact(Artifact::with_kind(
        ArtifactType::Header,
    )));
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(30.0, 30.0, 70.0, 70.0));
    surface.end_tagged();

    surface.finish();
    page.finish();

    let mut tag_tree = TagTree::new();
    tag_tree.push(id1);
    tag_tree.push(id2);
    document.set_tag_tree(tag_tree);

    let metadata = metadata_1();
    document.set_metadata(metadata);
}

fn invalid_codepoint_impl(document: &mut Document, font: Font, text: &str) {
    let mut page = document.start_page();
    let mut surface = page.surface();

    let glyphs = vec![
        KrillaGlyph::new(GlyphId::new(3), 2048.0, 0.0, 0.0, 0.0, 0..1, None),
        KrillaGlyph::new(GlyphId::new(2), 2048.0, 0.0, 0.0, 0.0, 1..4, None),
    ];

    surface.draw_glyphs(
        Point::from_xy(0.0, 100.0),
        &glyphs,
        font.clone(),
        text,
        20.0,
        false,
    );
    surface.finish();
    page.finish();
}

#[test]
fn validate_pdfu_invalid_codepoint() {
    let mut document = Document::new_with(settings_9());
    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();
    invalid_codepoint_impl(&mut document, font.clone(), "A\u{FEFF}B");

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::InvalidCodepointMapping(
            font,
            GlyphId::new(2),
            '\u{FEFF}',
            None
        )]
    )
}

#[test]
fn validate_pdfa_no_codepoint() {
    let mut document = Document::new_with(settings_20());
    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();
    let mut page = document.start_page();
    let mut surface = page.surface();

    let glyphs = [KrillaGlyph::new(
        GlyphId::new(3),
        2048.0,
        0.0,
        0.0,
        0.0,
        0..0,
        None,
    )];

    surface.draw_glyphs(
        Point::from_xy(0.0, 100.0),
        &glyphs,
        font.clone(),
        "",
        20.0,
        false,
    );
    surface.finish();
    page.finish();

    assert!(
        validation_errors(document.finish()).contains(&ValidationError::NoCodepointMapping(
            font,
            GlyphId::new(1),
            None
        ))
    );
}

#[test]
fn validate_pdfa_private_unicode_codepoint() {
    let mut document = Document::new_with(settings_13());
    let metadata = metadata_1();
    document.set_metadata(metadata);
    document.set_tag_tree(TagTree::new());
    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();
    invalid_codepoint_impl(&mut document, font.clone(), "A\u{E022}B");

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::UnicodePrivateArea(
            font,
            GlyphId::new(2),
            '\u{E022}',
            None
        )]
    )
}

#[snapshot(document, settings_20)]
fn validate_pdf_a1_a_full_example(document: &mut Document) {
    validate_pdf_tagged_full_example(document);
}

#[snapshot(document, settings_19)]
fn validate_pdf_a1_b_full_example(document: &mut Document) {
    validate_pdf_full_example(document);
}

#[snapshot(document, settings_13)]
fn validate_pdf_a2_a_full_example(document: &mut Document) {
    validate_pdf_tagged_full_example(document);
}

#[snapshot(document, settings_7)]
fn validate_pdf_a2_b_full_example(document: &mut Document) {
    validate_pdf_full_example(document);
}

#[snapshot(document, settings_9)]
fn validate_pdf_a2_u_full_example(document: &mut Document) {
    validate_pdf_full_example(document);
}

#[snapshot(document, settings_14)]
fn validate_pdf_a3_a_full_example(document: &mut Document) {
    validate_pdf_tagged_full_example(document);
}

#[snapshot(document, settings_10)]
fn validate_pdf_a3_b_full_example(document: &mut Document) {
    validate_pdf_full_example(document);
}

#[snapshot(document, settings_11)]
fn validate_pdf_a3_u_full_example(document: &mut Document) {
    validate_pdf_full_example(document);
}

#[snapshot(document, settings_26)]
fn validate_pdf_a4_full_example(document: &mut Document) {
    validate_pdf_full_example(document);
}

#[snapshot(document, settings_27)]
fn validate_pdf_a4f_full_example(document: &mut Document) {
    validate_pdf_full_example(document);
}

#[snapshot(document, settings_28)]
fn validate_pdf_a4e_full_example(document: &mut Document) {
    validate_pdf_full_example(document);
}

#[test]
fn validate_pdf_ua1_empty_annotation_alt() {
    let mut document = Document::new_with(settings_15());
    let mut page = document.start_page();

    let annot_loc = loc(1);
    let annot = page.add_tagged_annotation(
        Annotation::new_link(
            LinkAnnotation::new(
                Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
                Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
            ),
            Some(String::new()),
        )
        .with_location(Some(annot_loc)),
    );

    page.finish();

    let div_loc = loc(2);
    let mut tag_group = TagGroup::new(Tag::Div.with_location(Some(div_loc)));
    tag_group.push(annot);

    let mut tag_tree = TagTree::new();
    tag_tree.push(tag_group);
    document.set_tag_tree(tag_tree);

    assert!(validation_errors(document.finish())
        .contains(&ValidationError::MissingAnnotationAltText(Some(annot_loc))));
}

#[test]
fn validate_pdf_ua1_empty_alt() {
    let mut document = Document::new_with(settings_15());
    let mut page = document.start_page();
    let mut surface = page.surface();

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();

    let id1 = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.draw_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "Hi",
        false,
        TextDirection::Auto,
    );
    surface.end_tagged();

    surface.finish();

    page.finish();

    let formula_loc = loc(1);
    let mut tag_group =
        TagGroup::new(Tag::Formula(Some(String::new())).with_location(Some(formula_loc)));
    tag_group.push(id1);

    let mut tag_tree = TagTree::new();
    tag_tree.push(tag_group);
    document.set_tag_tree(tag_tree);

    assert!(validation_errors(document.finish())
        .contains(&ValidationError::MissingAltText(Some(formula_loc))));
}

#[snapshot(document, settings_15)]
fn validate_pdf_ua1_full_example(document: &mut Document) {
    let mut page = document.start_page();
    let mut surface = page.surface();

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();

    let id1 = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.draw_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "This is some text",
        false,
        TextDirection::Auto,
    );
    surface.end_tagged();

    surface.finish();

    let annotation = page.add_tagged_annotation(Annotation::new_link(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
            Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
        ),
        Some("A link to youtube".to_string()),
    ));

    let mut link_group = TagGroup::new(Tag::Link);
    link_group.push(annotation);

    page.finish();

    let mut tag_tree = TagTree::new();
    tag_tree.push(id1);
    tag_tree.push(link_group);
    document.set_tag_tree(tag_tree);

    let metadata = Metadata::new()
        .language("en".to_string())
        .title("a nice title".to_string());
    document.set_metadata(metadata);

    let outline = Outline::new();
    document.set_outline(outline);
}

#[test]
fn validate_pdf_ua1_missing_requirements() {
    let mut document = Document::new_with(settings_15());
    let mut page = document.start_page();
    let mut surface = page.surface();

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();

    let id1 = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.draw_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "Hi",
        false,
        TextDirection::Auto,
    );
    surface.end_tagged();

    surface.finish();

    let annot_loc = loc(1);
    let annot = page.add_tagged_annotation(
        Annotation::new_link(
            LinkAnnotation::new(
                Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
                Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
            ),
            None,
        )
        .with_location(Some(annot_loc)),
    );

    page.finish();

    let formula_loc = loc(2);
    let mut tag_group = TagGroup::new(Tag::Formula(None).with_location(Some(formula_loc)));
    tag_group.push(id1);
    tag_group.push(annot);

    let mut tag_tree = TagTree::new();
    tag_tree.push(tag_group);
    document.set_tag_tree(tag_tree);

    assert_eq!(
        validation_errors(document.finish()),
        vec![
            ValidationError::MissingDocumentOutline,
            ValidationError::MissingAnnotationAltText(Some(annot_loc)),
            ValidationError::MissingAltText(Some(formula_loc)),
            ValidationError::NoDocumentTitle
        ]
    )
}

#[snapshot(document, settings_15)]
fn validate_pdf_ua1_attributes(document: &mut Document) {
    let mut page = document.start_page();
    let mut surface = page.surface();

    let id1 = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 100.0, 100.0));
    surface.end_tagged();

    let id2 = surface.start_tagged(ContentTag::Other);
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 100.0, 100.0));
    surface.end_tagged();

    surface.finish();
    page.finish();

    let mut tag_tree = TagTree::new();

    let mut group1 = TagGroup::new(Tag::L(ListNumbering::Circle));
    group1.push(id1);

    let mut group2 = TagGroup::new(Tag::TH(TableHeaderScope::Row));
    let mut group3 = TagGroup::new(Tag::TR);
    let mut group4 = TagGroup::new(Tag::Table);
    group2.push(id2);
    group3.push(group2);
    group4.push(group3);

    tag_tree.push(group1);
    tag_tree.push(group4);
    document.set_tag_tree(tag_tree);

    let metadata = Metadata::new()
        .language("en".to_string())
        .title("a nice title".to_string());
    document.set_metadata(metadata);

    let outline = Outline::new();
    document.set_outline(outline);
}

#[snapshot(document, settings_16)]
fn pdf_version_14_tagged(document: &mut Document) {
    validate_pdf_tagged_full_example(document);
}

#[test]
fn validate_pdf_a1_no_transparency() {
    let mut document = Document::new_with(settings_19());
    let metadata = metadata_1();
    document.set_metadata(metadata);
    let mut page = document.start_page();
    let mut surface = page.surface();
    surface.set_fill(Some(red_fill(0.5)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 100.0, 100.0));
    surface.finish();
    page.finish();

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::Transparency(None)]
    )
}

#[test]
fn validate_pdf_a1_no_image_transparency() {
    let mut document = Document::new_with(settings_19());
    let metadata = metadata_1();
    document.set_metadata(metadata);

    let image = load_png_image("rgba8.png");
    let size = Size::from_wh(image.size().0 as f32, image.size().1 as f32).unwrap();

    let mut page = document.start_page();
    let mut surface = page.surface();
    surface.draw_image(image, size);
    surface.finish();
    page.finish();

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::Transparency(None)]
    )
}

#[snapshot(document, settings_22)]
fn validate_other_version(document: &mut Document) {
    validate_pdf_full_example(document);
}

#[test]
fn validate_pdf_a1_limits() {
    let mut document = Document::new_with(settings_19());
    let mut page = document.start_page();

    // An array can only have 8191 elements, so it must not be possible to have that many.
    for _ in 0..8193 {
        page.add_annotation(youtube_link(100.0, 100.0, 100.0, 100.0));
    }

    page.add_annotation(youtube_link(66000.1, 66000.1, 100.0, 100.0));
    page.finish();

    assert_eq!(
        validation_errors(document.finish()),
        vec![
            ValidationError::TooLargeFloat,
            ValidationError::TooLongArray,
        ]
    )
}

#[test]
fn validate_pdf_a3_a_no_tag_tree() {
    let mut document = Document::new_with(settings_24());
    document.set_metadata(metadata_1().title("krilla test".to_string()));

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::MissingTagging]
    )
}

#[test]
fn validate_pdf_a3_missing_fields() {
    let mut d = Document::new_with(settings_23());
    let mut f1 = file_1();
    f1.description = None;
    f1.modification_date = None;
    d.embed_file(f1);

    assert_eq!(
        validation_errors(d.finish()),
        vec![
            ValidationError::EmbeddedFile(EmbedError::MissingDate, None),
            ValidationError::EmbeddedFile(EmbedError::MissingDescription, None)
        ]
    )
}

#[snapshot(document, settings_23)]
fn validate_pdf_a3_with_embedded_file(d: &mut Document) {
    embedded_file_impl(d)
}

#[snapshot(document, settings_27)]
fn validate_pdf_a4_f_with_embedded_file(d: &mut Document) {
    embedded_file_impl(d)
}

// See https://github.com/LaurenzV/krilla/issues/162
// Can't include this test because it would requires us to embed the font in the snapshot.
#[cfg(target_os = "macos")]
#[ignore]
fn validate_pdf_a1_b_ttc(d: &mut Document) {
    let font_data: crate::Data = std::fs::read("/System/Library/Fonts/Supplemental/Songti.ttc")
        .unwrap()
        .into();
    let font = Font::new(font_data.clone(), 3).unwrap();

    let mut page = d.start_page();
    let mut surface = page.surface();

    surface.draw_text(
        Point::from_xy(0.0, 75.0),
        font.clone(),
        20.0,
        "文",
        false,
        TextDirection::Auto,
    );
}

#[test]
fn validate_pdf_a1_b_cmyk_image_without_icc_profile() {
    let mut document = Document::new_with(settings_19());
    let mut page = document.start_page();
    let mut surface = page.surface();
    let image = load_jpg_image("cmyk.jpg");
    let size = image.size();
    surface.draw_image(
        image.clone(),
        Size::from_wh(size.0 as f32, size.1 as f32).unwrap(),
    );

    surface.finish();
    page.finish();

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::MissingCMYKProfile]
    );
}

#[snapshot(document, settings_15)]
fn validate_pdf_ua1_only_annotation(document: &mut Document) {
    let mut page = document.start_page();

    let annotation = page.add_tagged_annotation(Annotation::new_link(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
            Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
        ),
        Some("A link to youtube".to_string()),
    ));

    let mut link_group = TagGroup::new(Tag::Link);
    link_group.push(annotation);

    page.finish();

    let mut tag_tree = TagTree::new();
    tag_tree.push(link_group);
    document.set_tag_tree(tag_tree);

    let metadata = Metadata::new()
        .language("en".to_string())
        .title("a nice title".to_string());
    document.set_metadata(metadata);

    let outline = Outline::new();
    document.set_outline(outline);
}

#[test]
fn validate_deduplicate_errors() {
    let mut document = Document::new_with(settings_19());
    let mut page = document.start_page();
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(0.5)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 20.0, 20.0));
    surface.set_location(loc(2));
    surface.set_fill(Some(red_fill(0.4)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 20.0, 20.0));
    surface.reset_location();
    surface.set_fill(Some(red_fill(0.3)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 20.0, 20.0));
    surface.finish();
    page.finish();

    assert_eq!(
        validation_errors(document.finish()),
        vec![
            ValidationError::Transparency(None),
            ValidationError::Transparency(Some(loc(2)))
        ]
    );
}

#[test]
fn validate_inconsistent_separation_fallback() {
    let mut document = Document::new_with(settings_7());
    let mut page = document.start_page();
    let mut surface = page.surface();

    // First usage of "PANTONE 185 C" with red fallback
    let space1 = separation::SeparationSpace::new(
        separation::SeparationColorant::Custom("PANTONE 185 C".to_string()),
        rgb::Color::new(255, 0, 0).into(),
    );
    let color1: krilla::color::Color = separation::Color::new(255, space1).into();
    let fill1 = Fill {
        paint: color1.into(),
        opacity: NormalizedF32::ONE,
        rule: Default::default(),
    };
    surface.set_fill(Some(fill1));
    surface.draw_path(&rect_to_path(0.0, 0.0, 20.0, 20.0));

    // Second usage of "PANTONE 185 C" with DIFFERENT blue fallback
    // This should trigger a validation error
    let space2 = separation::SeparationSpace::new(
        separation::SeparationColorant::Custom("PANTONE 185 C".to_string()),
        rgb::Color::new(0, 0, 255).into(),
    );
    let color2: krilla::color::Color = separation::Color::new(255, space2).into();
    let fill2 = Fill {
        paint: color2.into(),
        opacity: NormalizedF32::ONE,
        rule: Default::default(),
    };
    surface.set_fill(Some(fill2));
    surface.draw_path(&rect_to_path(30.0, 0.0, 50.0, 20.0));

    surface.finish();
    page.finish();

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::InconsistentSeparationFallback(
            separation::SeparationColorant::Custom("PANTONE 185 C".to_string())
        )]
    );
}

fn header_footer_artifact_subtypes_impl(settings: SerializeSettings) -> Document {
    let mut document = Document::new_with(settings);
    let mut page = document.start_page();
    let mut surface = page.surface();

    let id = surface.start_tagged(ContentTag::Artifact(Artifact::with_kind(
        ArtifactType::Header,
    )));
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(30.0, 30.0, 70.0, 70.0));
    surface.end_tagged();

    surface.finish();
    page.finish();

    let mut tag_tree = TagTree::new();
    tag_tree.push(id);
    document.set_tag_tree(tag_tree);
    document.set_metadata(metadata_2());
    document.set_outline(Outline::new());

    document
}

fn validate_pdf14_ua1_header_footer_artifact_subtypes() {
    let document = header_footer_artifact_subtypes_impl(settings_33());

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::RequiresNewerPdfVersion(
            VersionedFeature::HeaderFooterArtifactSubtypes,
            None
        )]
    );
}

#[test]
fn validate_pdf17_ua1_header_footer_artifact_subtypes() {
    let document = header_footer_artifact_subtypes_impl(pdf_ua1_settings(
        VersionedFeature::HeaderFooterArtifactSubtypes.minimum_pdf_version(),
    ));

    assert!(document.finish().is_ok());
}

#[snapshot(document)]
fn no_validators_embedded_file_no_af(d: &mut Document) {
    // Embedded files are written but no AF (associated files) entry is
    // produced, because an empty validator set means `allows_associated_files`
    // is vacuously false.
    embedded_file_impl(d);
}

// A-3b + UA-1: Even though neither PDF 1.7 nor PDF/UA-1 specify associated
// files, A-3b adds them, so the AF entry should be written.
#[snapshot(document, settings_32)]
fn validate_multi_validator_embedded_file_af(d: &mut Document) {
    let metadata = Metadata::new()
        .language("en".to_string())
        .title("a nice title".to_string())
        .creation_date(DateTime::new(2001));
    d.set_metadata(metadata);
    d.set_tag_tree(TagTree::new());
    d.set_outline(Outline::new());

    d.embed_file(file_1());
}

// A-3b + UA-1: UA-1 requires an outline; A-3b does not.
#[test]
fn validate_multi_validator_ua1_prohibits_missing_outline() {
    let mut document = Document::new_with(settings_32());
    let metadata = Metadata::new()
        .language("en".to_string())
        .title("title".to_string())
        .creation_date(DateTime::new(2001));
    document.set_metadata(metadata);
    document.set_tag_tree(TagTree::new());

    let mut page = document.start_page();
    page.surface().finish();
    page.finish();

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::MissingDocumentOutline]
    );
}

#[snapshot(document, settings_32)]
fn validate_multi_validator_pdf_a3b_pdf_ua1_full_example(document: &mut Document) {
    let mut page = document.start_page();
    let mut surface = page.surface();

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();

    let id1 = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.draw_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "Hello, PDF/A-3b + PDF/UA-1",
        false,
        TextDirection::Auto,
    );
    surface.end_tagged();

    surface.finish();
    page.finish();

    let mut tag_tree = TagTree::new();
    tag_tree.push(id1);
    document.set_tag_tree(tag_tree);

    let metadata = Metadata::new()
        .language("en".to_string())
        .title("a nice title".to_string())
        .creation_date(DateTime::new(2001));
    document.set_metadata(metadata);

    document.set_outline(Outline::new());
}

fn structure_order_tabbing_impl(settings: SerializeSettings) -> Document {
    let mut document = Document::new_with(settings);
    let mut page = document.start_page();

    let annot = page.add_tagged_annotation(Annotation::new_link(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
            Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
        ),
        Some("Link to YouTube".to_string()),
    ));

    page.finish();

    let mut tag_tree = TagTree::new();
    tag_tree.push(annot);
    document.set_tag_tree(tag_tree);

    document.set_metadata(metadata_2());

    let outline = Outline::new();
    document.set_outline(outline);

    document
}

#[test]
fn validate_pdf14_ua1_structure_order_tabbing() {
    let document = structure_order_tabbing_impl(settings_33());

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::RequiresNewerPdfVersion(
            VersionedFeature::StructureOrderTabbing,
            None
        )]
    );
}

#[test]
fn validate_pdf15_ua1_structure_order_tabbing() {
    let document = structure_order_tabbing_impl(pdf_ua1_settings(
        VersionedFeature::StructureOrderTabbing.minimum_pdf_version(),
    ));

    assert!(document.finish().is_ok());
}

fn table_header_scope_impl(settings: SerializeSettings) -> Document {
    let mut document = Document::new_with(settings);
    let mut page = document.start_page();
    let mut surface = page.surface();

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();

    let text_id = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.draw_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "header",
        false,
        TextDirection::Auto,
    );
    surface.end_tagged();

    surface.finish();
    page.finish();

    let mut row = TagGroup::new(Tag::TR);
    let mut th = TagGroup::new(Tag::TH(TableHeaderScope::Row));
    th.push(text_id);
    row.push(th);

    let mut table = TagGroup::new(Tag::Table);
    table.push(row);

    let mut tag_tree = TagTree::new();
    tag_tree.push(table);
    document.set_tag_tree(tag_tree);

    document.set_metadata(metadata_2());

    let outline = Outline::new();
    document.set_outline(outline);

    document
}

#[test]
fn validate_pdf14_ua1_table_header_scope() {
    let document = table_header_scope_impl(settings_33());

    assert_eq!(
        validation_errors(document.finish()),
        vec![ValidationError::RequiresNewerPdfVersion(
            VersionedFeature::TableHeaderScope,
            None
        )]
    );
}

#[test]
fn validate_pdf15_ua1_table_header_scope() {
    let document = table_header_scope_impl(pdf_ua1_settings(
        VersionedFeature::TableHeaderScope.minimum_pdf_version(),
    ));

    assert!(document.finish().is_ok());
}

#[test]
fn validate_pdf14_tagged_annotation_no_ua() {
    // Ensure tagging + annotation + PDF 1.4 without UA validator doesn't
    // fail (see https://github.com/LaurenzV/krilla/pull/278#discussion_r3213542007).
    let mut document = Document::new_with(settings_17());
    let mut page = document.start_page();

    let annot = page.add_tagged_annotation(Annotation::new_link(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
            Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
        ),
        None,
    ));

    page.finish();

    let mut tag_tree = TagTree::new();
    tag_tree.push(annot);
    document.set_tag_tree(tag_tree);

    assert!(document.finish().is_ok());
}

fn pdfx_page_settings() -> PageSettings {
    let ps = PageSettings::from_wh(200.0, 200.0).unwrap();
    let trim = Rect::from_xywh(0.0, 0.0, 200.0, 200.0).unwrap();
    ps.with_trim_box(Some(trim))
}

/// Helper that creates a valid PDF/X document with text and a shape.
/// Uses CMYK fill for X-1a compatibility.
fn validate_pdf_x_full_example_cmyk(document: &mut Document) {
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();

    surface.draw_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "This is some text",
        false,
        TextDirection::Auto,
    );

    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(30.0, 30.0, 70.0, 70.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X Document".to_string());
    document.set_metadata(metadata);
}

/// Helper that creates a valid PDF/X document using RGB (for X-3, X-4).
fn validate_pdf_x_full_example_rgb(document: &mut Document) {
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();

    surface.draw_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "This is some text",
        false,
        TextDirection::Auto,
    );

    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(30.0, 30.0, 70.0, 70.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X Document".to_string());
    document.set_metadata(metadata);
}

/// Helper that creates a valid PDF/X document for an RGB output intent
/// (PDF/X-4p, PDF/X-6p). Every print element — including the text — uses an
/// explicit RGB fill: a DeviceGray (default-black) fill would be uncharacterized
/// under an RGB output intent, since a device colour space is permitted only
/// when it matches the intent or the intent is CMYK and the space is DeviceGray
/// (ISO 15930-7 §6.4.3.2, ISO 15930-9 §6.6.3.2).
fn validate_pdf_x_full_example_rgb_intent(document: &mut Document) {
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    // Set the RGB fill before drawing the text so the glyphs are not painted in
    // the default DeviceGray black.
    surface.set_fill(Some(red_fill(1.0)));

    let font_data = NOTO_SANS.clone();
    let font = Font::new(font_data, 0).unwrap();

    surface.draw_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "This is some text",
        false,
        TextDirection::Auto,
    );

    surface.draw_path(&rect_to_path(30.0, 30.0, 70.0, 70.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X Document".to_string());
    document.set_metadata(metadata);
}

fn pdfx_validation_document(settings: SerializeSettings) -> Document {
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    document
}

// ---- PDF/X snapshot tests ----

#[snapshot(document, settings_34)]
fn validate_pdf_x4_full_example(document: &mut Document) {
    validate_pdf_x_full_example_rgb(document);
}

#[snapshot(document, settings_35)]
fn validate_pdf_x3_full_example(document: &mut Document) {
    validate_pdf_x_full_example_rgb(document);
}

#[snapshot(document, settings_36)]
fn validate_pdf_x1a_full_example(document: &mut Document) {
    validate_pdf_x_full_example_cmyk(document);
}

#[snapshot(document, settings_37)]
fn validate_pdf_x4p_full_example(document: &mut Document) {
    // PDF/X-4p uses an RGB output intent, so all content must be RGB.
    validate_pdf_x_full_example_rgb_intent(document);
}

#[snapshot(document, settings_38)]
fn validate_pdf_x6_full_example(document: &mut Document) {
    validate_pdf_x_full_example_rgb(document);
}

#[snapshot(document, settings_42)]
fn validate_pdf_x6p_full_example(document: &mut Document) {
    // PDF/X-6p uses an RGB output intent, so all content must be RGB.
    validate_pdf_x_full_example_rgb_intent(document);
}

#[snapshot(document, settings_40)]
fn validate_pdf_a2b_x4_full_example(document: &mut Document) {
    validate_pdf_x_full_example_rgb(document);
}

#[snapshot(document, settings_41)]
fn validate_pdf_a3b_x4_full_example(document: &mut Document) {
    validate_pdf_x_full_example_rgb(document);
}

// ---- PDF/X unit tests ----

#[test]
fn validate_pdf_x1a_no_rgb() {
    let mut document = Document::new_with(settings_36());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X-1a".to_string());
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| matches!(e, ValidationError::ContainsRgb(_))));
        }
        other => panic!("expected ContainsRgb error, got {other:?}"),
    }
}

#[test]
fn validate_pdf_x1a_no_rgb_image() {
    use krilla::image::Image;

    let mut document = Document::new_with(settings_36());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    let image_data = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/images/rgb8.png"),
    )
    .unwrap();
    let image = Image::from_png(image_data.into(), false).unwrap();
    surface.draw_image(image, Size::from_wh(50.0, 50.0).unwrap());

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X-1a".to_string());
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| matches!(e, ValidationError::ContainsRgb(_))));
        }
        other => panic!("expected ContainsRgb error, got {other:?}"),
    }
}

#[test]
fn validate_pdf_x1a_luma_ok() {
    let mut document = Document::new_with(settings_36());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    let fill = Fill {
        paint: luma::Color::new(128).into(),
        opacity: NormalizedF32::ONE,
        rule: FillRule::default(),
    };
    surface.set_fill(Some(fill));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X-1a".to_string());
    document.set_metadata(metadata);

    assert!(document.finish().is_ok());
}

#[test]
fn validate_pdf_x1a_no_annotations() {
    let mut document = Document::new_with(settings_36());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);

    page.add_annotation(Annotation::new_link(
        LinkAnnotation::new(
            Rect::from_xywh(0.0, 0.0, 100.0, 20.0).unwrap(),
            Target::Action(LinkAction::new("https://example.com".to_string()).into()),
        ),
        None,
    ));

    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X-1a".to_string());
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| e == &ValidationError::ContainsAnnotation(None)));
        }
        other => panic!("expected ContainsAnnotation error, got {other:?}"),
    }
}

#[test]
fn validate_pdf_x1a_no_transparency() {
    let mut document = Document::new_with(settings_36());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(cmyk_fill(0.5)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X-1a".to_string());
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| e == &ValidationError::Transparency(None)));
        }
        other => panic!("expected Transparency error, got {other:?}"),
    }
}

#[test]
fn validate_pdf_x3_no_transparency() {
    let mut document = Document::new_with(settings_35());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(0.5)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X-3".to_string());
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| e == &ValidationError::Transparency(None)));
        }
        other => panic!("expected Transparency error, got {other:?}"),
    }
}

#[test]
fn validate_pdf_x4_transparency_ok() {
    let mut document = Document::new_with(settings_34());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(0.5)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X".to_string());
    document.set_metadata(metadata);

    assert!(document.finish().is_ok());
}

#[test]
fn validate_pdf_x_missing_trim_art_box() {
    let mut document = Document::new_with(settings_34());
    let mut page = document.start_page();
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001));
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| matches!(e, ValidationError::MissingTrimOrArtBox(0, _))));
        }
        other => panic!("expected MissingTrimOrArtBox error, got {other:?}"),
    }
}

#[test]
fn validate_pdf_x_with_trim_box() {
    let mut document = Document::new_with(settings_34());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X".to_string());
    document.set_metadata(metadata);

    assert!(document.finish().is_ok());
}

#[test]
fn validate_pdf_x_with_art_box() {
    let mut document = Document::new_with(settings_34());
    let ps = PageSettings::from_wh(200.0, 200.0).unwrap();
    let art = Rect::from_xywh(0.0, 0.0, 200.0, 200.0).unwrap();
    let page_settings = ps.with_art_box(Some(art));
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X".to_string());
    document.set_metadata(metadata);

    assert!(document.finish().is_ok());
}

#[test]
fn validate_pdf_x1a_cmyk_fill_ok() {
    let mut document = Document::new_with(settings_36());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X-1a".to_string());
    document.set_metadata(metadata);

    assert!(document.finish().is_ok());
}

#[test]
fn validate_pdf_x_missing_date() {
    let mut document = Document::new_with(settings_34());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new().language("en".to_string());
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| e == &ValidationError::MissingDocumentDate));
        }
        other => panic!("expected MissingDocumentDate error, got {other:?}"),
    }
}

#[test]
fn validate_pdf_x1a_no_title() {
    let mut document = Document::new_with(settings_36());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001));
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| e == &ValidationError::NoDocumentTitle));
        }
        other => panic!("expected NoDocumentTitle error, got {other:?}"),
    }
}

#[test]
fn validate_pdf_x3_no_title() {
    let mut document = Document::new_with(settings_35());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001));
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| e == &ValidationError::NoDocumentTitle));
        }
        other => panic!("expected NoDocumentTitle error, got {other:?}"),
    }
}

#[test]
fn validate_pdf_x_title_required_for_pdf14_16_levels_only() {
    // PDF/X-1a/-3 (Info `Title`) and PDF/X-4/-4p (ISO 15930-7: `dc:title` in the
    // mandatory metadata set) require a document title. PDF/X-6/-6p (ISO 15930-9
    // §6.11) do NOT — only xmp:CreateDate/ModifyDate/MetadataDate are mandatory.
    // X-1a/X-3 are covered by `validate_pdf_x1a_no_title` / `validate_pdf_x3_no_title`.
    let build = |settings: SerializeSettings| {
        let mut document = Document::new_with(settings);
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();
        surface.set_fill(Some(red_fill(1.0)));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.finish();
        // language + creation date, but deliberately no title.
        document.set_metadata(
            Metadata::new()
                .language("en".to_string())
                .creation_date(DateTime::new(2001)),
        );
        document.finish()
    };

    // PDF/X-4/-4p require a title.
    for (name, settings) in [("X4", settings_34()), ("X4P", settings_37())] {
        match build(settings) {
            Err(KrillaError::Validation(errors)) => assert!(
                errors
                    .iter()
                    .any(|(e, _)| e == &ValidationError::NoDocumentTitle),
                "{name}: expected NoDocumentTitle, got {errors:?}"
            ),
            other => panic!("{name}: expected NoDocumentTitle error, got {other:?}"),
        }
    }

    // PDF/X-6/-6p do not require a title: a title-less file is conformant.
    for (name, settings) in [("X6", settings_38()), ("X6P", settings_42())] {
        assert!(
            build(settings).is_ok(),
            "{name}: PDF/X-6/-6p must not require a document title"
        );
    }
}

#[test]
fn validate_pdf_x1a_separation_cmyk_fallback_ok() {
    let mut document = Document::new_with(settings_36());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    let space = separation::SeparationSpace::new(
        separation::SeparationColorant::Custom("PANTONE 185 C".to_string()),
        cmyk::Color::new(0, 255, 255, 0).into(),
    );
    let color: krilla::color::Color = separation::Color::new(255, space).into();
    let fill = Fill {
        paint: color.into(),
        opacity: NormalizedF32::ONE,
        rule: FillRule::default(),
    };
    surface.set_fill(Some(fill));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X-1a".to_string());
    document.set_metadata(metadata);

    assert!(document.finish().is_ok());
}

#[test]
fn validate_pdf_x1a_separation_rgb_fallback() {
    let mut document = Document::new_with(settings_36());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    let space = separation::SeparationSpace::new(
        separation::SeparationColorant::Custom("PANTONE 185 C".to_string()),
        rgb::Color::new(255, 0, 0).into(),
    );
    let color: krilla::color::Color = separation::Color::new(255, space).into();
    let fill = Fill {
        paint: color.into(),
        opacity: NormalizedF32::ONE,
        rule: FillRule::default(),
    };
    surface.set_fill(Some(fill));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X-1a".to_string());
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| matches!(e, ValidationError::ContainsRgb(_))));
        }
        other => panic!("expected ContainsRgb error, got {other:?}"),
    }
}

#[test]
fn validate_pdf_x1a_missing_cmyk_profile() {
    // X1A without a CMYK profile should trigger MissingCMYKProfile.
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X1A)
            .finish()
            .unwrap(),
        // No cmyk_profile provided.
        ..crate::settings_1()
    };
    let mut document = Document::new_with(settings);
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X-1a".to_string());
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| e == &ValidationError::MissingCMYKProfile));
        }
        other => panic!("expected MissingCMYKProfile error, got {other:?}"),
    }
}

#[test]
fn validate_pdfa1b_pdfx1a_writes_both_output_intents() {
    // PDF/A-1b and PDF/X-1a both permit a file to carry a GTS_PDFA1 and a
    // GTS_PDFX output intent sharing one embedded profile (ISO 19005-1 §6.2.2:
    // "if a file's OutputIntents array contains more than one entry, then all
    // entries that contain a DestOutputProfile key shall have ... the same
    // indirect object"; ISO 15930-4 §6.2.2: "Additional output intent
    // dictionaries may be present; if so, they shall use different values for
    // the S key"). The combination is valid and writes both intents.
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_archival_validator(Archival::A1_B)
            .with_prepress_validator(Prepress::X1A)
            .finish()
            .unwrap(),
        cmyk_profile: settings_36().cmyk_profile,
        ..settings_1()
    };
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/A-1b + PDF/X-1a".to_string()),
    );
    let pdf = document.finish().unwrap();
    let pdf_text = String::from_utf8_lossy(&pdf);
    assert_eq!(
        pdf_text.matches("/Type /OutputIntent").count(),
        2,
        "the combination writes both a PDF/A and a PDF/X output intent"
    );
    assert!(pdf_text.contains("/S /GTS_PDFA1"));
    assert!(pdf_text.contains("/S /GTS_PDFX"));

    // PDF/A-1a + PDF/X-3 likewise composes (config builds successfully).
    assert!(ConfigurationBuilder::new()
        .with_archival_validator(Archival::A1_A)
        .with_prepress_validator(Prepress::X3)
        .finish()
        .is_ok());
}

#[test]
fn validate_combined_pdfa2b_pdfx4_writes_both_output_intents() {
    // PDF/A pairs with the embedded-profile PDF/X-4: the file carries both a
    // GTS_PDFA1 intent (so PDF/A characterizes the DeviceCMYK content) and a
    // GTS_PDFX intent (for PDF/X), sharing one embedded profile. The
    // external-profile PDF/X-4p/X-6p cannot do this (PDF/A forbids
    // DestOutputProfileRef) and are rejected at configuration time instead
    // (see the configure module tests).
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_archival_validator(Archival::A2_B)
            .with_prepress_validator(Prepress::X4)
            .finish()
            .unwrap(),
        cmyk_profile: settings_36().cmyk_profile,
        ..settings_1()
    };
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/A-2b + PDF/X-4".to_string()),
    );

    let pdf = document.finish().unwrap();
    let pdf_text = String::from_utf8_lossy(&pdf);
    assert_eq!(
        pdf_text.matches("/Type /OutputIntent").count(),
        2,
        "the combination writes both a PDF/A and a PDF/X output intent"
    );
    assert!(
        pdf_text.contains("/S /GTS_PDFA1"),
        "PDF/A needs its own GTS_PDFA1 output intent for device colour"
    );
    assert!(
        pdf_text.contains("/S /GTS_PDFX"),
        "PDF/X-4 needs a GTS_PDFX output intent"
    );
}

#[test]
fn validate_pdf_x_variants_require_document_date_without_metadata_object() {
    for (name, settings) in [
        ("X4", settings_34()),
        ("X4P", settings_37()),
        ("X6", settings_38()),
        ("X6P", settings_42()),
        ("A2B_X4", settings_40()),
        ("A3B_X4", settings_41()),
    ] {
        let document = pdfx_validation_document(settings);

        match document.finish() {
            Err(KrillaError::Validation(errors)) => {
                assert!(
                    errors
                        .iter()
                        .any(|(e, _)| e == &ValidationError::MissingDocumentDate),
                    "{name}: expected MissingDocumentDate, got {errors:?}"
                );
            }
            other => panic!("{name}: expected MissingDocumentDate error, got {other:?}"),
        }
    }
}

#[test]
fn validate_pdf_x_embedded_output_intent_variants_require_cmyk_profile() {
    for (name, configuration) in [
        (
            "X3",
            ConfigurationBuilder::new()
                .with_prepress_validator(Prepress::X3)
                .finish()
                .unwrap(),
        ),
        (
            "X4",
            ConfigurationBuilder::new()
                .with_prepress_validator(Prepress::X4)
                .finish()
                .unwrap(),
        ),
        (
            "X6",
            ConfigurationBuilder::new()
                .with_prepress_validator(Prepress::X6)
                .finish()
                .unwrap(),
        ),
        (
            "A2B_X4",
            ConfigurationBuilder::new()
                .with_archival_validator(Archival::A2_B)
                .with_prepress_validator(Prepress::X4)
                .finish()
                .unwrap(),
        ),
        (
            "A3B_X4",
            ConfigurationBuilder::new()
                .with_archival_validator(Archival::A3_B)
                .with_prepress_validator(Prepress::X4)
                .finish()
                .unwrap(),
        ),
    ] {
        let settings = SerializeSettings {
            configuration,
            ..crate::settings_1()
        };
        let mut document = Document::new_with(settings);
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();

        surface.set_fill(Some(red_fill(1.0)));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.finish();

        document.set_metadata(
            Metadata::new()
                .language("en".to_string())
                .creation_date(DateTime::new(2001))
                .title(name.to_string()),
        );

        match document.finish() {
            Err(KrillaError::Validation(errors)) => {
                assert!(
                    errors
                        .iter()
                        .any(|(e, _)| e == &ValidationError::MissingCMYKProfile),
                    "{name}: expected MissingCMYKProfile, got {errors:?}"
                );
            }
            other => panic!("{name}: expected MissingCMYKProfile error, got {other:?}"),
        }
    }
}

#[test]
fn validate_pdf_x_rejects_nonconforming_page_boxes() {
    let full = Rect::from_xywh(0.0, 0.0, 200.0, 200.0).unwrap();
    let run = |ps: PageSettings| {
        let mut document = Document::new_with(settings_34());
        let mut page = document.start_page_with(ps);
        let mut surface = page.surface();
        surface.set_fill(Some(red_fill(1.0)));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.finish();
        document.set_metadata(metadata_1().title("krilla test".to_string()));
        validation_errors(document.finish())
    };

    // A page must carry exactly one of TrimBox/ArtBox, never both.
    assert!(run(PageSettings::from_wh(200.0, 200.0)
        .unwrap()
        .with_trim_box(Some(full))
        .with_art_box(Some(full)))
    .iter()
    .any(|e| matches!(e, ValidationError::BothTrimAndArtBox(0, _))));

    // The MediaBox must contain every other box.
    assert!(run(PageSettings::from_wh(200.0, 200.0)
        .unwrap()
        .with_trim_box(Some(Rect::from_xywh(0.0, 0.0, 500.0, 500.0).unwrap())))
    .iter()
    .any(|e| matches!(e, ValidationError::PageBoxNotNested(0, _))));

    // A BleedBox must contain the TrimBox/ArtBox.
    assert!(run(PageSettings::from_wh(200.0, 200.0)
        .unwrap()
        .with_trim_box(Some(full))
        .with_bleed_box(Some(Rect::from_xywh(80.0, 80.0, 40.0, 40.0).unwrap())))
    .iter()
    .any(|e| matches!(e, ValidationError::PageBoxNotNested(0, _))));
}

#[test]
fn validate_pdf_x_bleed_box_must_lie_within_crop_box() {
    // ISO 15930-4 §6.8 / ISO 15930-7 §6.12: "If the CropBox is present, none of
    // the ArtBox, the TrimBox, or the BleedBox shall extend beyond the
    // boundaries of the CropBox." A BleedBox larger than the CropBox is rejected.
    let ps = PageSettings::from_wh(200.0, 200.0)
        .unwrap()
        .with_crop_box(Some(Rect::from_xywh(10.0, 10.0, 180.0, 180.0).unwrap()))
        .with_bleed_box(Some(Rect::from_xywh(0.0, 0.0, 200.0, 200.0).unwrap()))
        .with_trim_box(Some(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap()));
    let mut document = Document::new_with(settings_34());
    let mut page = document.start_page_with(ps);
    let mut surface = page.surface();
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(30.0, 30.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::PageBoxNotNested(0, _))),
        "a BleedBox extending beyond the CropBox must be rejected, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x3_rejects_embedded_file() {
    // PDF/X-3 (PDF 1.4 blind exchange) forbids embedded files, like PDF/X-1a.
    let mut document = Document::new_with(settings_35());
    document.embed_file(file_1());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X-3".to_string()),
    );
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::EmbeddedFile(EmbedError::Existence, _))),
        "expected EmbeddedFile(Existence), got {errors:?}"
    );
}

#[test]
fn validate_pdf_x4p_cmyk_content_requires_cmyk_output_intent() {
    // settings_37 is PDF/X-4p with an RGB external output profile; DeviceCMYK
    // content is not characterized by it.
    let mut document = Document::new_with(settings_37());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::OutputIntentColorSpaceMismatch(_))),
        "expected OutputIntentColorSpaceMismatch, got {errors:?}"
    );
}

#[test]
fn validate_pdf_a_with_external_profile_pdfx_is_rejected() {
    // PDF/A requires its output profile to be embedded, while PDF/X-4p/X-6p
    // reference it externally via DestOutputProfileRef, which PDF/A forbids. The
    // combination is rejected when building the configuration rather than
    // emitting a file whose GTS_PDFX intent violates PDF/A.
    for (archival, prepress) in [
        (Archival::A2_B, Prepress::X4P),
        (Archival::A3_B, Prepress::X4P),
        (Archival::A4, Prepress::X6P),
    ] {
        assert!(
            matches!(
                ConfigurationBuilder::new()
                    .with_archival_validator(archival)
                    .with_prepress_validator(prepress)
                    .finish(),
                Err(ConfigurationError::IncompatibleOutputIntents(_))
            ),
            "{archival:?} + {prepress:?} must be rejected"
        );
    }
}

#[test]
fn validate_pdf_x_rejects_annotation_inside_print_area() {
    // PDF/X (except X-1a, which forbids annotations) requires annotations to lie
    // outside the print area; this link is inside the full-page TrimBox.
    let mut document = Document::new_with(settings_34());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.add_annotation(youtube_link(50.0, 50.0, 50.0, 20.0));
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::AnnotationInsidePrintArea(_))),
        "expected AnnotationInsidePrintArea, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x_rejects_rgb_annotation_color() {
    // An annotation's /C color array cannot be ICC-wrapped, so an RGB border is
    // uncharacterized DeviceRGB under the CMYK output intent.
    let mut document = Document::new_with(settings_34());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.add_annotation(Annotation::from(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 50.0, 50.0, 20.0).unwrap(),
            Target::Action(LinkAction::new("https://example.com".to_string()).into()),
        )
        .with_border(LinkBorder::new(1.0, rgb::Color::new(255, 0, 0).into())),
    ));
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::AnnotationContainsRgb(_))),
        "expected AnnotationContainsRgb, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x1a_image_with_icc_is_device_not_iccbased() {
    // PDF/X-1a forbids ICCBased color spaces, so a gray/CMYK image carrying an
    // embedded ICC profile must serialize as DeviceGray/DeviceCMYK instead.
    for image in [
        load_png_image("luma8_icc.png"),
        load_jpg_image("ccyk_icc.jpg"),
    ] {
        let mut document = Document::new_with(settings_36());
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();
        surface.draw_image(image, Size::from_wh(50.0, 50.0).unwrap());
        surface.finish();
        page.finish();
        document.set_metadata(
            Metadata::new()
                .language("en".to_string())
                .creation_date(DateTime::new(2001))
                .title("PDF/X-1a".to_string()),
        );
        let pdf = document.finish().unwrap();
        assert!(
            !String::from_utf8_lossy(&pdf).contains("ICCBased"),
            "PDF/X-1a image color space must be device, not ICCBased"
        );
    }
}

#[test]
fn validate_pdf_x1a_rejects_icc_v4_output_profile() {
    // PDF/X-1a is PDF 1.4, which admits only ICC v2 output profiles.
    let mut icc = std::fs::read(crate::ASSETS_PATH.join("icc/krilla-generic-cmyk-v2.icc")).unwrap();
    icc[8] = 4; // bump the ICC major version to 4
    let configuration = ConfigurationBuilder::new()
        .with_prepress_validator(Prepress::X1A)
        .finish()
        .unwrap();
    let settings = SerializeSettings {
        configuration,
        cmyk_profile: Some(ICCProfile::<4>::new(&icc).unwrap()),
        ..settings_1()
    };
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X-1a".to_string()),
    );
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::IncompatibleOutputProfileVersion(_))),
        "expected IncompatibleOutputProfileVersion, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x4p_cmyk_image_requires_cmyk_output_intent() {
    // settings_37 is PDF/X-4p with an RGB output intent; a DeviceCMYK image
    // (cmyk.jpg carries no embedded ICC) is not characterized by it.
    let mut document = Document::new_with(settings_37());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.draw_image(
        load_jpg_image("cmyk.jpg"),
        Size::from_wh(50.0, 50.0).unwrap(),
    );
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::OutputIntentColorSpaceMismatch(_))),
        "expected OutputIntentColorSpaceMismatch, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x1a_rejects_inconsistent_separation_fallback() {
    // A colorant name must map to a single tint transform, even under X-1a.
    let mut document = Document::new_with(settings_36());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    for (i, fallback) in [
        cmyk::Color::new(0, 255, 255, 0),
        cmyk::Color::new(0, 200, 100, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let space = separation::SeparationSpace::new(
            separation::SeparationColorant::Custom("PANTONE 185 C".to_string()),
            fallback.into(),
        );
        let color: krilla::color::Color = separation::Color::new(255, space).into();
        surface.set_fill(Some(Fill {
            paint: color.into(),
            ..Default::default()
        }));
        surface.draw_path(&rect_to_path(i as f32 * 60.0, 0.0, 50.0, 50.0));
    }
    surface.finish();
    page.finish();
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X-1a".to_string()),
    );
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::InconsistentSeparationFallback(_))),
        "expected InconsistentSeparationFallback, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x_rejects_degenerate_page_box() {
    // A zero-area page box is not a valid page region.
    let mut document = Document::new_with(settings_34());
    let mut page = document.start_page_with(
        PageSettings::from_wh(200.0, 200.0)
            .unwrap()
            .with_trim_box(Some(Rect::from_xywh(10.0, 10.0, 0.0, 50.0).unwrap())),
    );
    let mut surface = page.surface();
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::DegeneratePageBox(0, _))),
        "expected DegeneratePageBox, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x4p_annotation_color_matches_output_intent() {
    // settings_37 is PDF/X-4p with an RGB output intent, so an RGB annotation
    // border is characterized but a CMYK one is not. The annotation sits outside
    // the print area to isolate the color check. The page fill is RGB (ICC-
    // wrapped to match the RGB intent); a DeviceGray fill would itself be
    // uncharacterized under an RGB intent.
    let run = |border: krilla::color::Color| {
        let mut document = Document::new_with(settings_37());
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();
        surface.set_fill(Some(Fill {
            paint: rgb::Color::new(0, 0, 255).into(),
            ..Default::default()
        }));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.add_annotation(Annotation::from(
            LinkAnnotation::new(
                Rect::from_xywh(250.0, 250.0, 50.0, 20.0).unwrap(),
                // An internal destination, not an Action: PDF/X-4p forbids
                // Actions, so the target must be a destination to isolate the
                // annotation-border colour check.
                Target::Destination(
                    krilla::destination::XyzDestination::new(0, Point::from_xy(10.0, 10.0)).into(),
                ),
            )
            .with_border(LinkBorder::new(1.0, border)),
        ));
        page.finish();
        document.set_metadata(metadata_1().title("krilla test".to_string()));
        document.finish()
    };

    assert!(run(rgb::Color::new(255, 0, 0).into()).is_ok());
    assert!(
        validation_errors(run(cmyk::Color::new(0, 0, 0, 255).into()))
            .iter()
            .any(|e| matches!(e, ValidationError::OutputIntentColorSpaceMismatch(_)))
    );
}

#[test]
fn validate_pdf_x4_output_profile_icc_version_cap() {
    // PDF/X-4 is PDF 1.6, whose output-intent ICC profile is capped at v4.2
    // (ISO 15930-7 Table 1 / ISO 32000-1 Table 66). v4.2 is accepted; v4.3 is
    // rejected.
    let build = |minor: u8| {
        let mut icc =
            std::fs::read(crate::ASSETS_PATH.join("icc/krilla-generic-cmyk-v2.icc")).unwrap();
        icc[8] = 4; // ICC major version 4
        icc[9] = minor; // ICC minor/bugfix nibbles
        let configuration = ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X4)
            .finish()
            .unwrap();
        let settings = SerializeSettings {
            configuration,
            cmyk_profile: Some(ICCProfile::<4>::new(&icc).unwrap()),
            ..settings_1()
        };
        let mut document = Document::new_with(settings);
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();
        surface.set_fill(Some(cmyk_fill(1.0)));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.finish();
        document.set_metadata(metadata_1().title("krilla test".to_string()));
        document.finish()
    };

    // v4.2 sits at the cap and is accepted.
    assert!(build(0x20).is_ok());
    // v4.3 exceeds the cap and is rejected.
    let errors = validation_errors(build(0x30));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::IncompatibleOutputProfileVersion(_))),
        "expected IncompatibleOutputProfileVersion, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x_action_rules_by_level() {
    // PDF/X-3/-4/-4p forbid interactive actions (ISO 15930-6 §6.14, ISO 15930-7
    // §6.18: "shall not include Actions or JavaScripts"). PDF/X-6/-6p (ISO
    // 15930-9 §6.14) permit GoTo/URI actions — the only kinds krilla emits.
    // (PDF/X-1a forbids annotations outright; covered separately.) The link sits
    // outside the 200×200 page to keep clear of the positional checks; the page
    // fill matches each level's output intent (CMYK or RGB).
    let build = |settings: SerializeSettings, fill: krilla::color::Color| {
        let mut document = Document::new_with(settings);
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();
        surface.set_fill(Some(Fill {
            paint: fill.into(),
            ..Default::default()
        }));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.add_annotation(Annotation::from(LinkAnnotation::new(
            Rect::from_xywh(250.0, 250.0, 50.0, 20.0).unwrap(),
            Target::Action(LinkAction::new("https://example.com".to_string()).into()),
        )));
        page.finish();
        document.set_metadata(
            Metadata::new()
                .language("en".to_string())
                .creation_date(DateTime::new(2001))
                .title("PDF/X".to_string()),
        );
        document.finish()
    };

    // PDF/X-3/-4 (CMYK intent) and PDF/X-4p (RGB intent) reject the link action.
    for (name, settings, fill) in [
        ("X3", settings_35(), cmyk::Color::new(0, 0, 0, 255).into()),
        ("X4", settings_34(), cmyk::Color::new(0, 0, 0, 255).into()),
        ("X4P", settings_37(), rgb::Color::new(0, 0, 255).into()),
    ] {
        assert!(
            validation_errors(build(settings, fill))
                .iter()
                .any(|e| matches!(e, ValidationError::ContainsAction(_))),
            "{name}: must forbid the link action"
        );
    }

    // PDF/X-6 (CMYK intent) and PDF/X-6p (RGB intent) permit the URI link
    // action, producing a fully conformant file.
    for (name, settings, fill) in [
        ("X6", settings_38(), cmyk::Color::new(0, 0, 0, 255).into()),
        ("X6P", settings_42(), rgb::Color::new(0, 0, 255).into()),
    ] {
        assert!(
            build(settings, fill).is_ok(),
            "{name}: must permit the URI link action"
        );
    }
}

#[test]
fn validate_pdf_x6_requires_trim_box() {
    // ISO 15930-9 §6.9: a PDF/X-6 page must include a TrimBox; an ArtBox is not
    // an acceptable substitute, but a coexisting ArtBox is permitted (there is
    // no "but not both" rule, unlike the earlier levels).
    let build = |ps: PageSettings| {
        let mut document = Document::new_with(settings_38());
        let mut page = document.start_page_with(ps);
        let mut surface = page.surface();
        surface.set_fill(Some(cmyk_fill(1.0)));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.finish();
        document.set_metadata(metadata_1().title("krilla test".to_string()));
        document.finish()
    };

    let full = Rect::from_xywh(0.0, 0.0, 200.0, 200.0).unwrap();

    // ArtBox but no TrimBox → rejected.
    let art_only = PageSettings::from_wh(200.0, 200.0)
        .unwrap()
        .with_art_box(Some(full));
    assert!(
        validation_errors(build(art_only))
            .iter()
            .any(|e| matches!(e, ValidationError::MissingTrimBox(0, _))),
        "PDF/X-6 must reject an ArtBox-only page"
    );

    // TrimBox and ArtBox together → fully conformant (no MissingTrimBox, no
    // BothTrimAndArtBox).
    let trim_and_art = PageSettings::from_wh(200.0, 200.0)
        .unwrap()
        .with_trim_box(Some(full))
        .with_art_box(Some(full));
    assert!(
        build(trim_and_art).is_ok(),
        "PDF/X-6 permits a coexisting TrimBox and ArtBox"
    );
}

#[test]
fn validate_pdf_x6_permits_annotations_in_print_area() {
    // ISO 15930-9 §6.12: PDF/X-6 permits annotations inside the visible area,
    // unlike PDF/X-3/-4/-4p. The link sits well inside the 200×200 TrimBox.
    let mut document = Document::new_with(settings_38());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.add_annotation(Annotation::from(LinkAnnotation::new(
        Rect::from_xywh(50.0, 50.0, 50.0, 20.0).unwrap(),
        Target::Destination(
            krilla::destination::XyzDestination::new(0, Point::from_xy(10.0, 10.0)).into(),
        ),
    )));
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    assert!(
        document.finish().is_ok(),
        "PDF/X-6 permits annotations in the print area"
    );
}

#[test]
fn validate_pdf_x_embedded_non_cmyk_profile_rejected() {
    // ISO 15930-7 §6.4.1 / ISO 15930-9 §6.6.1: the embedded output-intent
    // profile must have a GRAY/RGB/CMYK data colour space. A 4-channel '4CLR'
    // DeviceN profile is rejected even when the document has no DeviceCMYK
    // content (so no separate OutputIntentColorSpaceMismatch fires).
    let mut icc = std::fs::read(crate::ASSETS_PATH.join("icc/krilla-generic-cmyk-v2.icc")).unwrap();
    icc[16..20].copy_from_slice(b"4CLR"); // data colour space: 4CLR, not CMYK
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X4)
            .finish()
            .unwrap(),
        cmyk_profile: Some(ICCProfile::<4>::new(&icc).unwrap()),
        ..settings_1()
    };
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(Fill {
        paint: luma::Color::new(0).into(),
        ..Default::default()
    }));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidOutputProfileColorSpace(_))),
        "expected InvalidOutputProfileColorSpace, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x4p_device_gray_under_rgb_intent_rejected() {
    // ISO 15930-7 §6.4.3.2: a DeviceGray space is permitted under a CMYK or
    // grayscale output intent, but not under an RGB one (it would need a
    // DefaultGray colour space krilla does not emit). settings_37 is PDF/X-4p
    // with an RGB output intent.
    let mut document = Document::new_with(settings_37());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(Fill {
        paint: luma::Color::new(128).into(),
        ..Default::default()
    }));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::OutputIntentColorSpaceMismatch(_))),
        "DeviceGray under an RGB output intent must be flagged, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x_prohibits_verdicts_by_level() {
    use krilla::embed::EmbedError;
    // Exercise Prepress::prohibits for every level directly through the public
    // Validators::prohibits. Columns are the verdicts for X1A, X3, X4, X4P, X6,
    // X6P — `true` meaning the error is a conformance violation at that level.
    let font = Font::new(NOTO_SANS.clone(), 0).unwrap();
    let cases: Vec<(ValidationError, [bool; 6])> = vec![
        // String/name/indirect/q-nesting limits survive into PDF 1.6 but are
        // dropped by PDF 2.0.
        (
            ValidationError::TooLongString,
            [true, true, true, true, false, false],
        ),
        (
            ValidationError::TooLongName,
            [true, true, true, true, false, false],
        ),
        (
            ValidationError::TooManyIndirectObjects,
            [true, true, true, true, false, false],
        ),
        (
            ValidationError::TooHighQNestingLevel,
            [true, true, true, true, false, false],
        ),
        // Array/dictionary/float limits are PDF 1.4-only.
        (
            ValidationError::TooLongArray,
            [true, true, false, false, false, false],
        ),
        (
            ValidationError::TooLongDictionary,
            [true, true, false, false, false, false],
        ),
        (
            ValidationError::TooLargeFloat,
            [true, true, false, false, false, false],
        ),
        // PostScript calculator functions and transparency: PDF 1.4 levels only.
        (
            ValidationError::ContainsPostScript(None),
            [true, true, false, false, false, false],
        ),
        (
            ValidationError::Transparency(None),
            [true, true, false, false, false, false],
        ),
        // Embedded files: forbidden at the PDF 1.4 levels, permitted later.
        (
            ValidationError::EmbeddedFile(EmbedError::Existence, None),
            [true, true, false, false, false, false],
        ),
        // Interactive actions: forbidden except for the PDF 2.0 levels.
        (
            ValidationError::ContainsAction(None),
            [true, true, true, true, false, false],
        ),
        // Annotations inside the print area: enforced by X-3/X-4/X-4p only.
        (
            ValidationError::AnnotationInsidePrintArea(None),
            [false, true, true, true, false, false],
        ),
        // A document title: required except for X-6/X-6p.
        (
            ValidationError::NoDocumentTitle,
            [true, true, true, true, false, false],
        ),
        // A TrimBox specifically: required only by X-6/X-6p.
        (
            ValidationError::MissingTrimBox(0, None),
            [false, false, false, false, true, true],
        ),
        // "A TrimBox or an ArtBox, but not both": PDF 1.4/1.6 levels only.
        (
            ValidationError::BothTrimAndArtBox(0, None),
            [true, true, true, true, false, false],
        ),
        // Forbidden by every PDF/X level.
        (
            ValidationError::RestrictedLicense(font.clone()),
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::EmbeddedPDF(None),
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::InvalidOutputProfileColorSpace(None),
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::InvalidOutputProfileDeviceClass(None),
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::MissingCMYKProfile,
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::ContainsNotDefGlyph(font.clone(), None, String::new()),
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::MissingDocumentDate,
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::MixedGradientColorSpaces(None),
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::MissingTrimOrArtBox(0, None),
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::PageBoxNotNested(0, None),
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::DegeneratePageBox(0, None),
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::OutputIntentColorSpaceMismatch(None),
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::IncompatibleOutputProfileVersion(None),
            [true, true, true, true, true, true],
        ),
        (
            ValidationError::InconsistentSeparationFallback(
                separation::SeparationColorant::Custom("Spot".to_string()),
            ),
            [true, true, true, true, true, true],
        ),
        // PDF/X-1a only: RGB content and annotations are forbidden.
        (
            ValidationError::ContainsRgb(None),
            [true, false, false, false, false, false],
        ),
        (
            ValidationError::ContainsAnnotation(None),
            [true, false, false, false, false, false],
        ),
        // The 14400-unit page-size cap is a PDF 1.4 limit (X-1a/X-3 only).
        (
            ValidationError::PageBoxTooLarge(0, None),
            [true, true, false, false, false, false],
        ),
        // A non-Existence embedded-file error is never a PDF/X violation.
        (
            ValidationError::EmbeddedFile(EmbedError::MissingDate, None),
            [false, false, false, false, false, false],
        ),
        // An annotation border colour must be characterized at every level that
        // permits annotations (i.e. all but X-1a).
        (
            ValidationError::AnnotationContainsRgb(None),
            [false, true, true, true, true, true],
        ),
        // The external output profile is required by the -p variants only ...
        (
            ValidationError::MissingExternalOutputProfile,
            [false, false, false, true, false, true],
        ),
        // ... and unsupported by every non-p variant.
        (
            ValidationError::ExternalOutputProfileUnsupportedByValidator,
            [true, true, true, false, true, false],
        ),
        // Image interpolation is permitted by every PDF/X level (a representative
        // of the "allowed by every PDF/X standard" arm).
        (
            ValidationError::ImageInterpolation(None),
            [false, false, false, false, false, false],
        ),
    ];

    let levels = [
        ("X1A", Prepress::X1A),
        ("X3", Prepress::X3),
        ("X4", Prepress::X4),
        ("X4P", Prepress::X4P),
        ("X6", Prepress::X6),
        ("X6P", Prepress::X6P),
    ];

    for (error, expected) in &cases {
        for (i, (name, level)) in levels.iter().enumerate() {
            let validators = ConfigurationBuilder::new()
                .with_prepress_validator(*level)
                .finish()
                .unwrap()
                .validators();
            assert_eq!(
                validators.prohibits(error).is_some(),
                expected[i],
                "{name}: prohibits({error:?}) should be {}",
                expected[i]
            );
        }
    }
}

#[test]
fn validate_pdf_x4p_gray_annotation_border_under_rgb_intent_rejected() {
    // The annotation analogue of the gray-content rule: a DeviceGray annotation
    // border is not characterized by an RGB output intent (ISO 15930-7 §6.4.3.2).
    // settings_37 is PDF/X-4p with an RGB output intent.
    let mut document = Document::new_with(settings_37());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(Fill {
        paint: rgb::Color::new(0, 0, 255).into(),
        ..Default::default()
    }));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.add_annotation(Annotation::from(
        LinkAnnotation::new(
            Rect::from_xywh(250.0, 250.0, 50.0, 20.0).unwrap(),
            Target::Destination(
                krilla::destination::XyzDestination::new(0, Point::from_xy(10.0, 10.0)).into(),
            ),
        )
        .with_border(LinkBorder::new(1.0, luma::Color::new(128).into())),
    ));
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::OutputIntentColorSpaceMismatch(_))),
        "a DeviceGray annotation border under an RGB output intent must be flagged, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x4p_external_profile_rejects_non_prtr_device_class() {
    // ISO 15930-7 §6.4.2.1 (Annex A.1 extends it to the external profile): the
    // output-intent profile must be an Output Device Profile (Device Class
    // `prtr`). A display (`mntr`) external profile is rejected.
    let mut bytes =
        std::fs::read(crate::WORKSPACE_PATH.join("crates/krilla/icc/sRGB-v4.icc")).unwrap();
    bytes[12..16].copy_from_slice(b"mntr"); // display profile, not an output device
    let external = ExternalOutputProfile::rgb(
        ICCProfile::new(&bytes).unwrap(),
        vec!["https://example.com/p.icc".to_string()],
        "Custom".to_string(),
        "sRGB display profile".to_string(),
    )
    .unwrap();
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X4P)
            .finish()
            .unwrap(),
        external_output_profile: Some(external),
        ..settings_1()
    };
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(Fill {
        paint: rgb::Color::new(0, 0, 255).into(),
        ..Default::default()
    }));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidOutputProfileDeviceClass(_))),
        "expected InvalidOutputProfileDeviceClass for an mntr external profile, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x4p_external_profile_rejects_too_new_icc_version() {
    // PDF/X-4p is PDF 1.6, capped at ICC v4.2. An external v4.3 profile is
    // rejected (this exercises the external version-cap call site, distinct from
    // the embedded one).
    let mut bytes =
        std::fs::read(crate::WORKSPACE_PATH.join("crates/krilla/icc/sRGB-v4.icc")).unwrap();
    bytes[12..16].copy_from_slice(b"prtr"); // output device (the fixture ships as mntr)
    bytes[8] = 4;
    bytes[9] = 0x30; // ICC v4.3 — too new for PDF 1.6
    let external = ExternalOutputProfile::rgb(
        ICCProfile::new(&bytes).unwrap(),
        vec!["https://example.com/p.icc".to_string()],
        "Custom".to_string(),
        "sRGB v4.3 profile".to_string(),
    )
    .unwrap();
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X4P)
            .finish()
            .unwrap(),
        external_output_profile: Some(external),
        ..settings_1()
    };
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(Fill {
        paint: rgb::Color::new(0, 0, 255).into(),
        ..Default::default()
    }));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::IncompatibleOutputProfileVersion(_))),
        "expected IncompatibleOutputProfileVersion for a v4.3 external profile under X-4p, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x6_output_profile_icc_version_cap() {
    // PDF/X-6 is PDF 2.0, whose output-intent ICC profile is capped at v4.3
    // (ISO 15930-9, citing ISO 15076-1:2010). v4.3 is accepted; v4.4 is rejected.
    let build = |minor: u8| {
        let mut icc =
            std::fs::read(crate::ASSETS_PATH.join("icc/krilla-generic-cmyk-v2.icc")).unwrap();
        icc[8] = 4; // ICC major version 4
        icc[9] = minor;
        let settings = SerializeSettings {
            configuration: ConfigurationBuilder::new()
                .with_prepress_validator(Prepress::X6)
                .finish()
                .unwrap(),
            cmyk_profile: Some(ICCProfile::<4>::new(&icc).unwrap()),
            ..settings_1()
        };
        let mut document = Document::new_with(settings);
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();
        surface.set_fill(Some(cmyk_fill(1.0)));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.finish();
        document.set_metadata(metadata_1().title("krilla test".to_string()));
        document.finish()
    };

    // v4.3 sits at the cap and is accepted.
    assert!(build(0x30).is_ok());
    // v4.4 exceeds the cap and is rejected.
    let errors = validation_errors(build(0x40));
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::IncompatibleOutputProfileVersion(_))),
        "expected IncompatibleOutputProfileVersion for v4.4 under X-6, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x4p_device_gray_image_under_rgb_intent_rejected() {
    // A bare DeviceGray image (no embedded ICC) is not characterized by an RGB
    // output intent (ISO 15930-7 §6.4.3.2), mirroring the gray-fill rule on the
    // image path.
    use krilla::image::Image;
    let mut document = Document::new_with(settings_37());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    let image_data = std::fs::read(crate::ASSETS_PATH.join("images/luma8.png")).unwrap();
    let image = Image::from_png(image_data.into(), false).unwrap();
    surface.draw_image(image, Size::from_wh(50.0, 50.0).unwrap());
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::OutputIntentColorSpaceMismatch(_))),
        "a bare DeviceGray image under an RGB output intent must be flagged, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x4p_grayscale_output_intent() {
    // A grayscale (GRAY) external output intent, built via
    // `ExternalOutputProfile::luma`. ISO 15930-7 §6.4.3.2: a device colour space
    // may be used only if it matches the output intent (or the intent is CMYK
    // and the space is DeviceGray). So DeviceGray content matches and is
    // accepted, while DeviceCMYK content is not characterized and is flagged.
    let build = |fill: krilla::color::Color| {
        let mut bytes =
            std::fs::read(crate::WORKSPACE_PATH.join("crates/krilla/icc/sGrey-v2-magic.icc"))
                .unwrap();
        bytes[12..16].copy_from_slice(b"prtr"); // output device (the fixture ships as mntr)
        let external = ExternalOutputProfile::luma(
            ICCProfile::new(&bytes).unwrap(),
            vec!["https://example.com/grey.icc".to_string()],
            "Custom".to_string(),
            "sGrey output profile".to_string(),
        )
        .unwrap();
        let settings = SerializeSettings {
            configuration: ConfigurationBuilder::new()
                .with_prepress_validator(Prepress::X4P)
                .finish()
                .unwrap(),
            external_output_profile: Some(external),
            ..settings_1()
        };
        let mut document = Document::new_with(settings);
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();
        surface.set_fill(Some(Fill {
            paint: fill.into(),
            ..Default::default()
        }));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.finish();
        document.set_metadata(metadata_1().title("krilla test".to_string()));
        document.finish()
    };

    // DeviceGray content matches the grayscale intent and is accepted.
    assert!(
        build(luma::Color::new(128).into()).is_ok(),
        "DeviceGray content under a grayscale output intent must be accepted"
    );
    // DeviceCMYK content is not characterized by a grayscale intent and is flagged.
    assert!(
        validation_errors(build(cmyk::Color::new(0, 0, 0, 255).into()))
            .iter()
            .any(|e| matches!(e, ValidationError::OutputIntentColorSpaceMismatch(_))),
        "DeviceCMYK content under a grayscale output intent must be flagged"
    );
}

#[test]
fn validate_pdf_x4p_external_profile_writes_registry_name() {
    // The optional RegistryName is written into the external GTS_PDFX output
    // intent when set (ISO 15930-7 §6.4.2.1 permits it for a registry-defined
    // condition), and the builder trims/round-trips the value.
    let mut bytes =
        std::fs::read(crate::WORKSPACE_PATH.join("crates/krilla/icc/sRGB-v4.icc")).unwrap();
    bytes[12..16].copy_from_slice(b"prtr"); // output device (the fixture ships as mntr)
    let external = ExternalOutputProfile::rgb(
        ICCProfile::new(&bytes).unwrap(),
        vec!["https://example.com/p.icc".to_string()],
        "FOGRA51".to_string(),
        "RGB output profile".to_string(),
    )
    .unwrap()
    .with_registry_name("  http://www.color.org  ".to_string());
    assert_eq!(external.registry_name(), Some("http://www.color.org"));
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X4P)
            .finish()
            .unwrap(),
        external_output_profile: Some(external),
        ..settings_1()
    };
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(Fill {
        paint: rgb::Color::new(0, 0, 255).into(),
        ..Default::default()
    }));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let pdf = document.finish().unwrap();
    let text = String::from_utf8_lossy(&pdf);
    assert!(
        text.contains("/RegistryName (http://www.color.org)"),
        "the external output intent must write the RegistryName when set"
    );
}

#[test]
fn validate_pdfx_trapped_true_is_written() {
    use krilla::metadata::Trapping;
    // The affirmative Trapping::Trapped path. PDF/X-4 writes /Trapped /True in
    // the Info dict and pdf:Trapped=True in XMP; PDF/X-6 omits the Info dict and
    // carries it in XMP only.
    let build = |settings: SerializeSettings| {
        let mut document = pdfx_validation_document(settings);
        document.set_metadata(
            Metadata::new()
                .language("en".to_string())
                .creation_date(DateTime::new(2001))
                .title("PDF/X".to_string())
                .trapped(Trapping::Trapped),
        );
        String::from_utf8_lossy(&document.finish().unwrap()).into_owned()
    };

    let x4 = build(settings_34());
    assert!(
        x4.contains("/Trapped /True"),
        "PDF/X-4 with Trapping::Trapped must write /Trapped /True in the Info dict"
    );
    assert!(x4.contains("<pdf:Trapped>True</pdf:Trapped>"));

    let x6 = build(settings_38());
    assert!(
        !x6.contains("/Trapped"),
        "PDF/X-6 writes no Info-dict /Trapped entry"
    );
    assert!(x6.contains("<pdf:Trapped>True</pdf:Trapped>"));
}

#[test]
fn validate_pdf_x_annotation_inside_print_area_rejected_x3_x4p() {
    // ISO 15930-6 §6.13 / ISO 15930-7 §6.17: X-3/X-4/X-4p require annotations to
    // lie wholly outside the print area. A destination (not action) target keeps
    // clear of the action prohibition so the positional rule is isolated.
    let build = |settings: SerializeSettings, fill: krilla::color::Color| {
        let mut document = Document::new_with(settings);
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();
        surface.set_fill(Some(Fill {
            paint: fill.into(),
            ..Default::default()
        }));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        // The link overlaps the 200×200 TrimBox (the print area).
        page.add_annotation(Annotation::from(LinkAnnotation::new(
            Rect::from_xywh(50.0, 50.0, 50.0, 20.0).unwrap(),
            Target::Destination(
                krilla::destination::XyzDestination::new(0, Point::from_xy(10.0, 10.0)).into(),
            ),
        )));
        page.finish();
        document.set_metadata(metadata_1().title("krilla test".to_string()));
        document.finish()
    };

    // X-3 (CMYK intent) and X-4p (RGB intent) both reject the in-print-area link.
    for (name, settings, fill) in [
        ("X3", settings_35(), cmyk::Color::new(0, 0, 0, 255).into()),
        ("X4P", settings_37(), rgb::Color::new(0, 0, 255).into()),
    ] {
        assert!(
            validation_errors(build(settings, fill))
                .iter()
                .any(|e| matches!(e, ValidationError::AnnotationInsidePrintArea(_))),
            "{name}: an annotation inside the print area must be rejected"
        );
    }
}

#[test]
fn validate_pdf_x_annotation_edge_touching_print_area_is_outside() {
    // ISO 15930-7 §6.17: "A Rect shall be regarded as completely outside a
    // bounding box if all of the coordinates of the Rect lie either outside the
    // bounding box or on its edge, and the intersection of the two rectangles is
    // zero." So an annotation that merely touches the print-area edge is
    // accepted, while a one-unit overlap is inside and rejected. The print area
    // is the 200×200 TrimBox.
    let build = |rect: Rect| {
        let mut document = Document::new_with(settings_34());
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();
        surface.set_fill(Some(cmyk_fill(1.0)));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.add_annotation(Annotation::from(LinkAnnotation::new(
            rect,
            Target::Destination(
                krilla::destination::XyzDestination::new(0, Point::from_xy(10.0, 10.0)).into(),
            ),
        )));
        page.finish();
        document.set_metadata(metadata_1().title("krilla test".to_string()));
        document.finish()
    };

    // Left edge of the annotation coincides with the right edge of the print
    // area (annotation x ∈ [200, 250], print area x ∈ [0, 200]) → zero-area
    // intersection → outside → accepted.
    let touching = Rect::from_xywh(200.0, 50.0, 50.0, 20.0).unwrap();
    assert!(
        build(touching).is_ok(),
        "an edge-touching annotation must be treated as outside the print area"
    );

    // One unit of overlap (annotation x ∈ [199, 249]) → inside → rejected.
    let overlapping = Rect::from_xywh(199.0, 50.0, 50.0, 20.0).unwrap();
    assert!(
        validation_errors(build(overlapping))
            .iter()
            .any(|e| matches!(e, ValidationError::AnnotationInsidePrintArea(_))),
        "a one-unit-overlapping annotation must be inside the print area"
    );
}

#[test]
fn validate_pdf_a4_x6_writes_both_output_intents() {
    // PDF/A-4 + PDF/X-6 (both PDF 2.0) compose: the file carries both a
    // GTS_PDFA1 and a GTS_PDFX output intent sharing one embedded profile.
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_archival_validator(Archival::A4)
            .with_prepress_validator(Prepress::X6)
            .finish()
            .unwrap(),
        cmyk_profile: settings_38().cmyk_profile,
        ..settings_1()
    };
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/A-4 + PDF/X-6".to_string()),
    );
    let pdf = document.finish().unwrap();
    let text = String::from_utf8_lossy(&pdf);
    assert_eq!(
        text.matches("/Type /OutputIntent").count(),
        2,
        "PDF/A-4 + PDF/X-6 writes both output intents"
    );
    assert!(text.contains("/S /GTS_PDFA1"));
    assert!(text.contains("/S /GTS_PDFX"));
}

#[test]
fn validate_pdf_x_rejects_non_output_device_profile() {
    // The output intent must be an output-rendering (`prtr`) profile, not a
    // device-link/abstract/etc. transform profile (and not a `mntr` display one).
    let mut icc = std::fs::read(crate::ASSETS_PATH.join("icc/krilla-generic-cmyk-v2.icc")).unwrap();
    icc[12..16].copy_from_slice(b"link"); // device-link class
    let configuration = ConfigurationBuilder::new()
        .with_prepress_validator(Prepress::X4)
        .finish()
        .unwrap();
    let settings = SerializeSettings {
        configuration,
        cmyk_profile: Some(ICCProfile::<4>::new(&icc).unwrap()),
        ..settings_1()
    };
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::InvalidOutputProfileDeviceClass(_))),
        "expected InvalidOutputProfileDeviceClass, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x1a_rejects_oversized_page() {
    // PDF/X-1a is PDF 1.4, which caps page dimensions at 14400 units per side.
    let big = Rect::from_xywh(0.0, 0.0, 15000.0, 200.0).unwrap();
    let ps = PageSettings::from_wh(15000.0, 200.0)
        .unwrap()
        .with_trim_box(Some(big));
    let mut document = Document::new_with(settings_36());
    let mut page = document.start_page_with(ps);
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X-1a".to_string()),
    );
    let errors = validation_errors(document.finish());
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, ValidationError::PageBoxTooLarge(0, _))),
        "expected PageBoxTooLarge, got {errors:?}"
    );
}

#[test]
fn validate_pdf_x4p_writes_profile_name_for_unnamed_profile() {
    // `DestOutputProfileRef` requires `/ProfileName`. When the referenced profile
    // has no parseable description tag, krilla falls back to the output-condition
    // info so the key is always present.
    let mut icc = vec![0u8; 132];
    icc[8] = 0x02; // ICC v2.0
    icc[12..16].copy_from_slice(b"prtr"); // output device class
    icc[16..20].copy_from_slice(b"CMYK"); // CMYK data colour space
                                          // bytes 128..132 (tag count) stay zero -> no `desc` tag
    let external = ExternalOutputProfile::cmyk(
        ICCProfile::<4>::new(&icc).unwrap(),
        vec!["https://example.com/p.icc".to_string()],
        "Custom".to_string(),
        "Fallback profile name".to_string(),
    )
    .unwrap();
    let configuration = ConfigurationBuilder::new()
        .with_prepress_validator(Prepress::X4P)
        .finish()
        .unwrap();
    let settings = SerializeSettings {
        configuration,
        external_output_profile: Some(external),
        ..settings_1()
    };
    let mut document = Document::new_with(settings);
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(metadata_1().title("krilla test".to_string()));
    let pdf = document.finish().unwrap();
    let text = String::from_utf8_lossy(&pdf);
    assert!(
        text.contains("/ProfileName"),
        "DestOutputProfileRef must include /ProfileName"
    );
    assert!(
        text.contains("Fallback profile name"),
        "ProfileName should fall back to the output-condition info"
    );
}

#[test]
fn validate_pdf_x4p_external_cmyk_constructor_rejects_4clr_profile() {
    // A 4-channel non-CMYK ('4CLR' DeviceN) profile has the right channel count
    // for ExternalOutputProfile::cmyk but the wrong ICC data colour space. A
    // PDF/X output-intent profile must have a GRAY/RGB/CMYK data colour space
    // (ISO 15930-7 §6.4.1, Annex A.2), so the profile is rejected eagerly at
    // construction rather than producing a non-characterizing output intent.
    let mut icc = vec![0u8; 132];
    icc[8] = 0x02; // ICC v2
    icc[12..16].copy_from_slice(b"prtr"); // output device class
    icc[16..20].copy_from_slice(b"4CLR"); // 4-colorant, NOT 'CMYK'
    assert!(matches!(
        ExternalOutputProfile::cmyk(
            ICCProfile::<4>::new(&icc).unwrap(),
            vec!["https://example.com/p.icc".to_string()],
            "Custom".to_string(),
            "4CLR profile".to_string(),
        ),
        Err(krilla::ExternalOutputProfileError::WrongColorSpace)
    ));
}

#[test]
fn validate_date_info_and_xmp_offset_agree() {
    // The Info-dict date and its XMP counterpart must encode the same instant,
    // including a minutes-only UTC offset.
    let mut document = Document::new_with(settings_34());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();
    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .title("Offset".to_string())
            .creation_date(DateTime::new(2001).utc_offset_minute(30)),
    );
    let pdf = document.finish().unwrap();
    let text = String::from_utf8_lossy(&pdf);
    // Info dict uses pdf-writer's `D:...+HH'mm` form; XMP uses ISO-8601 `+HH:mm`.
    assert!(
        text.contains("+00'30"),
        "Info date should carry the +00:30 offset"
    );
    assert!(
        text.contains("+00:30"),
        "XMP date should carry the same +00:30 offset"
    );
}

#[test]
fn validate_pdf_x4_writes_required_xmp_fields() {
    let mut document = Document::new_with(settings_34());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(0.5)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
    surface.finish();
    page.finish();

    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X-4".to_string()),
    );

    let pdf = document.finish().unwrap();
    let pdf_text = String::from_utf8_lossy(&pdf);

    assert!(pdf_text.starts_with("%PDF-1.6"));
    assert!(pdf_text.contains("<xmp:MetadataDate>2001-01-01T00:00:00Z</xmp:MetadataDate>"));
    assert!(pdf_text.contains("<xmpMM:VersionID>1</xmpMM:VersionID>"));
}

#[test]
fn validate_pdf_x1a_gradient_checks_every_stop() {
    let mut document = Document::new_with(settings_36());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();

    let gradient = LinearGradient {
        x1: 0.0,
        y1: 0.0,
        x2: 100.0,
        y2: 0.0,
        transform: Default::default(),
        spread_method: SpreadMethod::Pad,
        stops: vec![
            Stop {
                offset: NormalizedF32::ZERO,
                color: cmyk::Color::new(255, 0, 0, 0).into(),
                opacity: NormalizedF32::ONE,
            },
            Stop {
                offset: NormalizedF32::ONE,
                color: rgb::Color::new(255, 0, 0).into(),
                opacity: NormalizedF32::ONE,
            },
        ],
        anti_alias: false,
    };

    surface.set_fill(Some(Fill {
        paint: gradient.into(),
        opacity: NormalizedF32::ONE,
        rule: FillRule::default(),
    }));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X-1a".to_string()),
    );

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(
                errors
                    .iter()
                    .any(|(e, _)| matches!(e, ValidationError::ContainsRgb(_))),
                "expected ContainsRgb, got {errors:?}"
            );
            assert!(
                errors
                    .iter()
                    .any(|(e, _)| matches!(e, ValidationError::MixedGradientColorSpaces(_))),
                "expected MixedGradientColorSpaces, got {errors:?}"
            );
        }
        other => panic!("expected gradient validation errors, got {other:?}"),
    }
}

#[test]
fn validate_mixed_gradient_stop_spaces_fail_cleanly_without_a_validator() {
    let mut document = pdfx_validation_document(crate::settings_1());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();

    let gradient = LinearGradient {
        x1: 0.0,
        y1: 0.0,
        x2: 100.0,
        y2: 0.0,
        transform: Default::default(),
        spread_method: SpreadMethod::Pad,
        stops: vec![
            Stop {
                offset: NormalizedF32::ZERO,
                color: cmyk::Color::new(255, 0, 0, 0).into(),
                opacity: NormalizedF32::ONE,
            },
            Stop {
                offset: NormalizedF32::ONE,
                color: luma::Color::new(0).into(),
                opacity: NormalizedF32::ONE,
            },
        ],
        anti_alias: false,
    };

    surface.set_fill(Some(Fill {
        paint: gradient.into(),
        opacity: NormalizedF32::ONE,
        rule: FillRule::default(),
    }));
    surface.draw_path(&rect_to_path(60.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    // Without a validator, mixed gradient color spaces are silently normalized
    // to the first stop's color space and no validation error is raised — the
    // validator-gated error is exercised by the PDF/X gradient tests above.
    assert!(
        document.finish().is_ok(),
        "expected a cleanly normalized document without a validator"
    );
}

#[test]
fn validate_pdf_x4p_requires_external_output_profile() {
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X4P)
            .finish()
            .unwrap(),
        ..crate::settings_1()
    };
    let mut document = pdfx_validation_document(settings);

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001));
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| e == &ValidationError::MissingExternalOutputProfile));
        }
        other => panic!("expected MissingExternalOutputProfile error, got {other:?}"),
    }
}

#[test]
fn external_output_profile_rejects_invalid_input() {
    use krilla::icc::ICCProfile;
    use krilla::{ExternalOutputProfile, ExternalOutputProfileError};

    let profile_bytes =
        std::fs::read(crate::WORKSPACE_PATH.join("crates/krilla/icc/sRGB-v4.icc")).unwrap();
    let profile = ICCProfile::<3>::new(&profile_bytes).unwrap();

    assert_eq!(
        ExternalOutputProfile::rgb(
            profile.clone(),
            vec![],
            "Custom".to_string(),
            "info".to_string(),
        )
        .err(),
        Some(ExternalOutputProfileError::EmptyUrls)
    );

    assert_eq!(
        ExternalOutputProfile::rgb(
            profile.clone(),
            vec!["   ".to_string()],
            "Custom".to_string(),
            "info".to_string(),
        )
        .err(),
        Some(ExternalOutputProfileError::EmptyUrls)
    );

    assert_eq!(
        ExternalOutputProfile::rgb(
            profile.clone(),
            vec!["https://example.com/profile.icc".to_string()],
            "   ".to_string(),
            "info".to_string(),
        )
        .err(),
        Some(ExternalOutputProfileError::EmptyIdentifier)
    );

    assert_eq!(
        ExternalOutputProfile::rgb(
            profile,
            vec!["https://example.com/profile.icc".to_string()],
            "Custom".to_string(),
            "   ".to_string(),
        )
        .err(),
        Some(ExternalOutputProfileError::EmptyInfo)
    );
}

#[test]
fn validate_x4_rejects_external_output_profile() {
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X4)
            .finish()
            .unwrap(),
        external_output_profile: Some(pdfx_external_output_profile()),
        ..crate::settings_1()
    };
    let mut document = pdfx_validation_document(settings);
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X".to_string()),
    );

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| e == &ValidationError::ExternalOutputProfileUnsupportedByValidator));
        }
        other => {
            panic!("expected ExternalOutputProfileUnsupportedByValidator error, got {other:?}")
        }
    }
}

#[test]
fn validate_pdf_x4p_with_external_profile_reference() {
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X4P)
            .finish()
            .unwrap(),
        external_output_profile: Some(pdfx_external_output_profile()),
        ..crate::settings_1()
    };
    let mut document = pdfx_validation_document(settings);
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X".to_string()),
    );

    let pdf = document.finish().unwrap();
    let pdf_text = String::from_utf8_lossy(&pdf);

    assert!(pdf_text.contains("/DestOutputProfileRef <<"));
    assert!(pdf_text.contains("/URLs ["));
    assert!(pdf_text.contains("/FS /URL"));
    assert!(pdf_text.contains("/F (https://example.com/profiles/sRGB-v4.icc)"));
    assert!(pdf_text.contains("/OutputConditionIdentifier (Custom)"));
    assert!(pdf_text.contains("/OutputCondition (sRGB)"));
    assert!(pdf_text.contains("/Info (sRGB v4 ICC profile)"));
    assert!(pdf_text.contains("/CheckSum <"));
    assert!(pdf_text.contains("/ICCVersion ("));
    assert!(pdf_text.contains("/ProfileCS ("));
    assert!(!pdf_text.contains("/DestOutputProfile "));
}

fn output_profile_refs(pdf_text: &str) -> Vec<&str> {
    let mut refs = Vec::new();
    let mut remainder = pdf_text;

    while let Some(start) = remainder.find("/DestOutputProfile ") {
        let tail = &remainder[start + "/DestOutputProfile ".len()..];
        let end = tail.find(" R").unwrap();
        refs.push(&tail[..end]);
        remainder = &tail[end + 2..];
    }

    refs
}

#[test]
fn validate_combined_pdfa_pdfx_declares_pdfx_extension_schema() {
    for (settings, use_cmyk) in [(settings_40(), false), (settings_41(), false)] {
        let mut document = Document::new_with(settings);
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();

        surface.set_fill(Some(if use_cmyk {
            cmyk_fill(1.0)
        } else {
            red_fill(1.0)
        }));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.finish();

        document.set_metadata(
            Metadata::new()
                .language("en".to_string())
                .creation_date(DateTime::new(2001))
                .title("Combined".to_string()),
        );

        let pdf = document.finish().unwrap();
        let pdf_text = String::from_utf8_lossy(&pdf);

        assert!(
            pdf_text.contains("xmlns:pdfxid=\"http://www.npes.org/pdfx/ns/id/\""),
            "missing pdfxid namespace declaration"
        );
        assert!(
            pdf_text.contains("<pdfaSchema:namespaceURI>http://www.npes.org/pdfx/ns/id/</pdfaSchema:namespaceURI>"),
            "missing PDF/A extension schema for pdfxid"
        );
        assert!(
            pdf_text.contains("<pdfaProperty:name>GTS_PDFXVersion</pdfaProperty:name>"),
            "missing PDF/A extension property declaration for GTS_PDFXVersion"
        );
    }
}

#[test]
fn validate_a2b_x4_uses_one_output_profile_for_both_intents() {
    // PDF/X-4 permits multiple output intents, so the combined PDF/A-2b +
    // PDF/X-4 export writes a GTS_PDFA1 and a GTS_PDFX intent, both of which
    // must reference the same embedded ICC profile.
    let mut document = Document::new_with(settings_40());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();

    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/A-2b + PDF/X-4".to_string()),
    );

    let pdf = document.finish().unwrap();
    let pdf_text = String::from_utf8_lossy(&pdf);
    let refs = output_profile_refs(&pdf_text);

    assert_eq!(refs.len(), 2, "expected two output intents");
    assert_eq!(
        refs[0], refs[1],
        "combined output intents must share one ICC profile"
    );
}

#[test]
fn validate_pdfx_device_content_is_not_iccbased() {
    // DeviceCMYK and DeviceGray page content under any PDF/X validator is
    // characterized by the CMYK output intent, so it must stay device — never
    // an ICCBased color space (an ICCBased CMYK identical to the output intent
    // is outright disallowed, and ICC-wrapping device colors the output intent
    // already covers is incorrect). This covers PDF/X-3 (settings_35), PDF/X-4
    // (settings_34) and PDF/X-6 (settings_38), all of which force
    // `no_device_cs` (which must still hold for RGB, but not CMYK/gray).
    for (label, settings) in [
        ("PDF/X-3", settings_35()),
        ("PDF/X-4", settings_34()),
        ("PDF/X-6", settings_38()),
    ] {
        let mut document = Document::new_with(settings);
        let mut page = document.start_page_with(pdfx_page_settings());
        let mut surface = page.surface();
        surface.set_fill(Some(cmyk_fill(1.0)));
        surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));
        surface.set_fill(Some(Fill {
            paint: luma::Color::new(64).into(),
            ..Default::default()
        }));
        surface.draw_path(&rect_to_path(60.0, 0.0, 50.0, 50.0));
        surface.finish();
        page.finish();
        document.set_metadata(
            Metadata::new()
                .language("en".to_string())
                .creation_date(DateTime::new(2001))
                .title(label.to_string()),
        );

        let pdf = document.finish().unwrap();
        let pdf_text = String::from_utf8_lossy(&pdf);
        assert!(
            !pdf_text.contains(" scn\n"),
            "{label}: device CMYK/gray content must use device operators, not ICCBased scn"
        );
        assert!(
            !pdf_text.contains("/ColorSpace <<"),
            "{label}: page resources must not declare an ICCBased alias for device content"
        );
    }
}

#[test]
fn validate_pdf_x1a_uses_device_cmyk_for_page_content() {
    let mut document = Document::new_with(settings_36());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();

    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X-1a".to_string()),
    );

    let pdf = document.finish().unwrap();
    let pdf_text = String::from_utf8_lossy(&pdf);

    assert!(
        !pdf_text.contains("/ColorSpace <<"),
        "PDF/X-1a page resources must not declare ICCBased aliases for page content"
    );
    assert!(
        !pdf_text.contains(" scn\n"),
        "PDF/X-1a page content should use device operators instead of ICCBased scn painting"
    );
}

#[test]
fn validate_a2b_x4_uses_device_cmyk_for_page_content() {
    // Even though PDF/A-2b would normally ICC-wrap CMYK, the active PDF/X-4
    // output intent characterizes DeviceCMYK, so CMYK page content must be
    // emitted as DeviceCMYK (never an ICCBased space duplicating the output
    // intent).
    let mut document = Document::new_with(settings_40());
    let mut page = document.start_page_with(pdfx_page_settings());
    let mut surface = page.surface();

    surface.set_fill(Some(cmyk_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/A-2b + PDF/X-4".to_string()),
    );

    let pdf = document.finish().unwrap();
    let pdf_text = String::from_utf8_lossy(&pdf);

    assert!(
        !pdf_text.contains("/ColorSpace <<"),
        "PDF/A-2b + PDF/X-4 page resources must not declare ICCBased aliases for page content"
    );
    assert!(
        !pdf_text.contains(" scn\n"),
        "PDF/A-2b + PDF/X-4 page content should use device operators instead of ICCBased scn painting"
    );
}

#[test]
fn validate_pdfx_downgrades_unknown_trapping_to_not_trapped() {
    use krilla::metadata::Trapping;
    // PDF/X-4 (PDF 1.6) writes /Trapped in both the Info dict and the XMP
    // metadata, so it exercises the Unknown→NotTrapped downgrade in both forms.
    let mut document = pdfx_validation_document(settings_34());
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X".to_string())
            .trapped(Trapping::Unknown),
    );

    let pdf = document.finish().unwrap();
    let pdf_text = String::from_utf8_lossy(&pdf);

    // PDF/X forbids the Unknown trapping state; krilla downgrades it to
    // NotTrapped in both the Info dict and the XMP metadata.
    assert!(
        pdf_text.contains("/Trapped /False"),
        "PDF/X-4 with Trapping::Unknown must still write /Trapped /False"
    );
    assert!(pdf_text.contains("<pdf:Trapped>False</pdf:Trapped>"));
    assert!(!pdf_text.contains("/Trapped /Unknown"));
}

#[test]
fn validate_pdf_x6_omits_info_dict_and_uses_xmp_metadata() {
    // ISO 15930-9 §6.5.2: the Info key shall not be present in a PDF/X-6 file
    // unless a PieceInfo entry exists (krilla never writes one), so X-6 carries
    // trapping and the PDF/X version in XMP, not the Info dict. §6.11.3
    // additionally requires the pdfxid:rev property.
    let mut document = pdfx_validation_document(settings_38());
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X".to_string()),
    );

    let pdf = document.finish().unwrap();
    let pdf_text = String::from_utf8_lossy(&pdf);

    // The Info-dict keys must NOT appear for X-6 (the dict is suppressed).
    assert!(
        !pdf_text.contains("/Trapped"),
        "PDF/X-6 must not write /Trapped in an Info dict"
    );
    assert!(
        !pdf_text.contains("/GTS_PDFXVersion"),
        "PDF/X-6 must not write /GTS_PDFXVersion in an Info dict"
    );
    // The metadata is carried in XMP instead.
    assert!(pdf_text.contains("<pdf:Trapped>False</pdf:Trapped>"));
    assert!(pdf_text.contains("<pdfxid:GTS_PDFXVersion>PDF/X-6</pdfxid:GTS_PDFXVersion>"));
    assert!(
        pdf_text.contains("<pdfxid:rev>2020</pdfxid:rev>"),
        "PDF/X-6 must write the required pdfxid:rev property"
    );
}

#[test]
fn validate_pdf_x6p_requires_external_output_profile() {
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X6P)
            .finish()
            .unwrap(),
        ..crate::settings_1()
    };
    let mut document = pdfx_validation_document(settings);

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001));
    document.set_metadata(metadata);

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| e == &ValidationError::MissingExternalOutputProfile));
        }
        other => panic!("expected MissingExternalOutputProfile error, got {other:?}"),
    }
}

#[test]
fn validate_pdf_x6p_transparency_ok() {
    let mut document = Document::new_with(settings_42());
    let page_settings = pdfx_page_settings();
    let mut page = document.start_page_with(page_settings);
    let mut surface = page.surface();

    surface.set_fill(Some(red_fill(0.5)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 50.0, 50.0));

    surface.finish();
    page.finish();

    let metadata = Metadata::new()
        .language("en".to_string())
        .creation_date(DateTime::new(2001))
        .title("PDF/X".to_string());
    document.set_metadata(metadata);

    assert!(document.finish().is_ok());
}

#[test]
fn validate_pdf_x6p_with_external_profile_reference() {
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X6P)
            .finish()
            .unwrap(),
        external_output_profile: Some(pdfx_external_output_profile()),
        ..crate::settings_1()
    };
    let mut document = pdfx_validation_document(settings);
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X".to_string()),
    );

    let pdf = document.finish().unwrap();
    let pdf_text = String::from_utf8_lossy(&pdf);

    assert!(pdf_text.starts_with("%PDF-2.0"));
    assert!(pdf_text.contains("/DestOutputProfileRef <<"));
    assert!(pdf_text.contains("/S /GTS_PDFX"));
    assert!(!pdf_text.contains("/DestOutputProfile "));
    assert!(pdf_text.contains("GTS_PDFXVersion"));
    assert!(pdf_text.contains("PDF/X-6p"));
}

#[test]
fn validate_x6_rejects_external_output_profile() {
    // X6 (not X6P) should reject external output profile.
    let settings = SerializeSettings {
        configuration: ConfigurationBuilder::new()
            .with_prepress_validator(Prepress::X6)
            .finish()
            .unwrap(),
        external_output_profile: Some(pdfx_external_output_profile()),
        ..crate::settings_1()
    };
    let mut document = pdfx_validation_document(settings);
    document.set_metadata(
        Metadata::new()
            .language("en".to_string())
            .creation_date(DateTime::new(2001))
            .title("PDF/X".to_string()),
    );

    match document.finish() {
        Err(KrillaError::Validation(errors)) => {
            assert!(errors
                .iter()
                .any(|(e, _)| e == &ValidationError::ExternalOutputProfileUnsupportedByValidator));
        }
        other => {
            panic!("expected ExternalOutputProfileUnsupportedByValidator error, got {other:?}")
        }
    }
}
