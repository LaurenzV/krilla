use krilla::action::LinkAction;
use krilla::annotation::{Annotation, LinkAnnotation, Target};
use krilla::configure::ValidationError;
use krilla::embed::EmbedError;
use krilla::error::KrillaError;
use krilla::geom::{Point, Rect, Size};
use krilla::metadata::{DateTime, Metadata};
use krilla::outline::Outline;
use krilla::page::Page;
use krilla::paint::{Fill, FillRule, LinearGradient, SpreadMethod};
use krilla::tagging::{ArtifactType, ContentTag, SpanTag, TagGroup, TagTree};
use krilla::tagging::{ListNumbering, TableHeaderScope, Tag};
use krilla::text::{Font, TextDirection};
use krilla::text::{GlyphId, KrillaGlyph};
use krilla_macros::snapshot;

use crate::embed::{embedded_file_impl, file_1};
use crate::{
    blue_fill, cmyk_fill, dummy_text_with_spans, green_fill, load_jpg_image, load_png_image, loc,
    metadata_1, rect_to_path, red_fill, settings_13, settings_15, settings_19, settings_23,
    settings_24, settings_7, settings_8, settings_9, stops_with_2_solid_1, youtube_link, NOTO_SANS,
};
use crate::{Document, SerializeSettings};

fn pdfa_document() -> Document {
    Document::new_with(settings_7())
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::TooHighQNestingLevel
        ]))
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::TooLongString
        ]))
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::ContainsPostScript(None)
        ]))
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::MissingCMYKProfile
        ]))
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::ContainsNotDefGlyph(font, None, "你".to_string())
        ]))
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::ContainsNotDefGlyph(font, Some(loc(4)), "i".to_string())
        ]))
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::Transparency(Some(loc(4))),
            ValidationError::Transparency(Some(loc(6))),
            // Note that we don't have 7 here, even though we should in theory. The reason is
            // that since we cache graphics states, only the first time we serialize it will
            // it trigger the validation error. Not optimal, but changing that would be a pain.
        ]))
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

    let id2 = surface.start_tagged(ContentTag::Artifact(ArtifactType::Header));
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::InvalidCodepointMapping(font, GlyphId::new(2), Some('\u{FEFF}'), None)
        ]))
    )
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::UnicodePrivateArea(font, GlyphId::new(2), '\u{E022}', None)
        ]))
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

    page.finish();

    let mut tag_tree = TagTree::new();
    tag_tree.push(id1);
    tag_tree.push(annotation);
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::MissingDocumentOutline,
            ValidationError::MissingAnnotationAltText(Some(annot_loc)),
            ValidationError::MissingAltText(Some(formula_loc)),
            ValidationError::NoDocumentTitle
        ]))
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::Transparency(None)
        ]))
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::Transparency(None)
        ]))
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::TooLargeFloat,
            ValidationError::TooLongArray,
        ]))
    )
}

#[test]
fn validate_pdf_a3_a_no_tag_tree() {
    let mut document = Document::new_with(settings_24());
    document.set_metadata(metadata_1());

    assert_eq!(
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::MissingTagging
        ]))
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
        d.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::EmbeddedFile(EmbedError::MissingDate, None),
            ValidationError::EmbeddedFile(EmbedError::MissingDescription, None)
        ]))
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::MissingCMYKProfile
        ]))
    );
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
        document.finish(),
        Err(KrillaError::Validation(vec![
            ValidationError::Transparency(None),
            ValidationError::Transparency(Some(loc(2)))
        ]))
    );
}
