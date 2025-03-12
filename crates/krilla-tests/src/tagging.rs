use krilla::action::{Action, LinkAction};
use krilla::annotation::{LinkAnnotation, Target};
use krilla::error::KrillaError;
use krilla::path::Fill;
use krilla::surface::{Surface, TextDirection};
use krilla::tagging::{ArtifactType, ContentTag, Tag, TagGroup, TagTree};
use krilla::{Document, Font};
use krilla_macros::snapshot;
use krilla_svg::{SurfaceExt, SvgSettings};
use tiny_skia_path::{Rect, Size, Transform};

use crate::{green_fill, load_png_image, rect_to_path, NOTO_SANS, SVGS_PATH};

pub trait SurfaceTaggingExt {
    fn fill_text_(&mut self, y: f32, content: &str);
}

impl SurfaceTaggingExt for Surface<'_> {
    fn fill_text_(&mut self, y: f32, content: &str) {
        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0, true).unwrap();

        self.fill_text(
            tiny_skia_path::Point::from_xy(0.0, y),
            Fill::default(),
            font,
            20.0,
            content,
            false,
            TextDirection::Auto,
        );
    }
}

#[snapshot(document)]
fn tagging_empty(document: &mut Document) {
    let tag_root = TagTree::new();
    document.set_tag_tree(tag_root);
}

fn tagging_simple_impl(document: &mut Document) {
    let mut tag_tree = TagTree::new();
    let mut par = TagGroup::new(Tag::P);

    let mut page = document.start_page();
    let mut surface = page.surface();
    let id = surface.start_tagged(ContentTag::Span(
        "en",
        Some("an alt text"),
        Some("expanded"),
        Some("actual text"),
    ));
    surface.fill_text_(25.0, "a paragraph");
    surface.end_tagged();

    surface.finish();
    page.finish();

    par.push(id);
    tag_tree.push(par);

    document.set_tag_tree(tag_tree);
}

fn tagging_simple_with_link_impl(document: &mut Document) {
    let mut tag_tree = TagTree::new();
    let mut par = TagGroup::new(Tag::P);
    let mut link = TagGroup::new(Tag::Link);

    let mut page = document.start_page();
    let mut surface = page.surface();
    let id = surface.start_tagged(ContentTag::Span("", None, None, None));
    surface.fill_text_(25.0, "a paragraph");
    surface.end_tagged();

    surface.finish();

    let link_id = page.add_tagged_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(0.0, 0.0, 100.0, 25.0).unwrap(),
            None,
            Target::Action(Action::Link(LinkAction::new("www.youtube.com".to_string()))),
        )
        .into(),
    );

    page.finish();

    link.push(link_id);
    link.push(id);
    par.push(link);
    tag_tree.push(par);

    document.set_tag_tree(tag_tree);
}

#[snapshot(document)]
fn tagging_simple(document: &mut Document) {
    tagging_simple_impl(document);
}

#[snapshot(document)]
fn tagging_simple_with_link(document: &mut Document) {
    tagging_simple_with_link_impl(document);
}

#[snapshot(document, settings_12)]
fn tagging_disabled(document: &mut Document) {
    tagging_simple_impl(document);
}

#[snapshot(document, settings_12)]
fn tagging_disabled_2(document: &mut Document) {
    tagging_simple_with_link_impl(document);
}

pub(crate) fn sample_svg() -> usvg::Tree {
    let data = std::fs::read(SVGS_PATH.join("resvg_shapes_rect_simple_case.svg")).unwrap();
    usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap()
}

#[snapshot(document)]
fn tagging_image_with_alt(document: &mut Document) {
    let mut tag_tree = TagTree::new();
    let mut image_group =
        TagGroup::new(Tag::Figure(Some("This is the alternate text.".to_string())));

    let mut page = document.start_page();
    let mut surface = page.surface();

    let id = surface.start_tagged(ContentTag::Other);
    let tree = sample_svg();
    surface.draw_svg(&tree, tree.size(), SvgSettings::default());
    surface.end_tagged();

    surface.finish();
    page.finish();

    image_group.push(id);
    tag_tree.push(image_group);

    document.set_tag_tree(tag_tree);
}

#[snapshot(document)]
fn tagging_multiple_content_tags(document: &mut Document) {
    let mut tag_tree = TagTree::new();

    let mut page = document.start_page();
    let mut surface = page.surface();
    let id1 = surface.start_tagged(ContentTag::Span("", None, None, None));
    surface.fill_text_(25.0, "a span");
    surface.end_tagged();
    let id2 = surface.start_tagged(ContentTag::Artifact(ArtifactType::Header));
    surface.fill_text_(50.0, "a header artifact");
    surface.end_tagged();
    let id3 = surface.start_tagged(ContentTag::Other);
    surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), Fill::default());
    surface.end_tagged();

    let id4 = surface.start_tagged(ContentTag::Other);
    let tree = sample_svg();
    surface.push_transform(&Transform::from_translate(100.0, 100.0));
    surface.draw_svg(&tree, tree.size(), SvgSettings::default());
    surface.pop();
    surface.end_tagged();

    let id5 = surface.start_tagged(ContentTag::Other);
    let image = load_png_image("rgb8.png");
    let image_size = Size::from_wh(image.size().0 as f32, image.size().1 as f32).unwrap();
    surface.push_transform(&Transform::from_translate(100.0, 300.0));
    surface.draw_image(image, image_size);
    surface.pop();
    surface.end_tagged();

    let id6 = surface.start_tagged(ContentTag::Artifact(ArtifactType::Other));
    surface.fill_text_(75.0, "a different type of artifact");
    surface.end_tagged();

    surface.finish();
    page.finish();

    tag_tree.push(id1);
    tag_tree.push(id2);
    tag_tree.push(id3);
    tag_tree.push(id4);
    tag_tree.push(id5);
    tag_tree.push(id6);

    document.set_tag_tree(tag_tree);
}

#[snapshot(document)]
fn tagging_multiple_pages(document: &mut Document) {
    let mut tag_tree = TagTree::new();
    let mut par_1 = TagGroup::new(Tag::P);
    let mut par_2 = TagGroup::new(Tag::P);
    let mut heading_1 = TagGroup::new(Tag::H1(Some("first heading".to_string())));
    let mut heading_2 = TagGroup::new(Tag::H1(Some("second heading".to_string())));

    let mut page = document.start_page();
    let mut surface = page.surface();
    let h1 = surface.start_tagged(ContentTag::Span("", None, None, None));
    surface.fill_text_(25.0, "a heading");
    surface.end_tagged();
    let p1 = surface.start_tagged(ContentTag::Span("", None, None, None));
    surface.fill_text_(50.0, "a paragraph");
    surface.end_tagged();
    surface.finish();
    page.finish();

    let mut page = document.start_page();
    let mut surface = page.surface();
    let p2 = surface.start_tagged(ContentTag::Span("", None, None, None));
    surface.fill_text_(75.0, "a second paragraph");
    surface.end_tagged();
    surface.finish();
    page.finish();

    let mut page = document.start_page();
    let mut surface = page.surface();
    let h2 = surface.start_tagged(ContentTag::Span("", None, None, None));
    surface.fill_text_(25.0, "another heading");
    surface.end_tagged();
    let p3 = surface.start_tagged(ContentTag::Span("", None, None, None));
    surface.fill_text_(50.0, "another paragraph");
    surface.end_tagged();
    surface.finish();
    page.finish();

    heading_1.push(h1);
    par_1.push(p1);
    par_1.push(p2);

    heading_2.push(h2);
    par_2.push(p3);

    let mut sect1 = TagGroup::new(Tag::Section);
    sect1.push(heading_1);
    sect1.push(par_1);
    let mut sect2 = TagGroup::new(Tag::Section);
    sect2.push(heading_2);
    sect2.push(par_2);

    tag_tree.push(sect1);
    tag_tree.push(sect2);

    document.set_tag_tree(tag_tree);
}

#[snapshot(document)]
fn tagging_two_footnotes(document: &mut Document) {
    let mut tag_tree = TagTree::new();
    let mut fn_group_1 = TagGroup::new(Tag::Note);
    let mut fn_group_2 = TagGroup::new(Tag::Note);

    let mut page = document.start_page();
    let mut surface = page.surface();

    let id1 = surface.start_tagged(ContentTag::Other);
    surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), green_fill(1.0));
    surface.end_tagged();

    let id2 = surface.start_tagged(ContentTag::Other);
    surface.fill_path(&rect_to_path(100.0, 100.0, 150.0, 150.0), green_fill(1.0));
    surface.end_tagged();

    surface.finish();
    page.finish();

    fn_group_1.push(id1);
    fn_group_2.push(id2);
    tag_tree.push(fn_group_1);
    tag_tree.push(fn_group_2);

    document.set_tag_tree(tag_tree);
}

#[test]
fn tagging_page_identifer_appears_twice() {
    let mut document = Document::new();
    let mut tag_tree = TagTree::new();
    let mut fn_group_1 = TagGroup::new(Tag::P);
    let mut fn_group_2 = TagGroup::new(Tag::P);

    let mut page = document.start_page();
    let mut surface = page.surface();

    let id1 = surface.start_tagged(ContentTag::Other);
    surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), green_fill(1.0));
    surface.end_tagged();

    surface.finish();
    page.finish();

    fn_group_1.push(id1);
    fn_group_2.push(id1);
    tag_tree.push(fn_group_1);
    tag_tree.push(fn_group_2);

    document.set_tag_tree(tag_tree);

    assert!(matches!(document.finish(), Err(KrillaError::UserError(_))))
}

#[test]
fn tagging_annotation_identifer_appears_twice() {
    let mut document = Document::new();
    let mut tag_tree = TagTree::new();
    let mut fn_group_1 = TagGroup::new(Tag::P);
    let mut fn_group_2 = TagGroup::new(Tag::P);

    let mut page = document.start_page();
    let link_id = page.add_tagged_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(0.0, 0.0, 100.0, 25.0).unwrap(),
            None,
            Target::Action(Action::Link(LinkAction::new("www.youtube.com".to_string()))),
        )
        .into(),
    );
    page.finish();

    fn_group_1.push(link_id);
    fn_group_2.push(link_id);
    tag_tree.push(fn_group_1);
    tag_tree.push(fn_group_2);

    document.set_tag_tree(tag_tree);

    assert!(matches!(document.finish(), Err(KrillaError::UserError(_))))
}

#[test]
fn tagging_missing_identifier_in_tree() {
    let mut document = Document::new();
    let tag_tree = TagTree::new();

    let mut page = document.start_page();
    let mut surface = page.surface();

    let _ = surface.start_tagged(ContentTag::Other);
    surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), green_fill(1.0));
    surface.end_tagged();

    surface.finish();
    page.finish();

    document.set_tag_tree(tag_tree);

    assert!(matches!(document.finish(), Err(KrillaError::UserError(_))))
}
