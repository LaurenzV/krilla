use std::num::NonZeroU32;

use krilla::action::{Action, LinkAction};
use krilla::annotation::{LinkAnnotation, Target};
use krilla::error::KrillaError;
use krilla::geom::{PathBuilder, Point, Rect, Size, Transform};
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke};
use krilla::surface::Surface;
use krilla::tagging::tag::{ListNumbering, TableHeaderScope, Tag, TagId};
use krilla::tagging::{ArtifactType, ContentTag, Node, SpanTag, TagGroup, TagTree};
use krilla::text::{Font, TextDirection};
use krilla::Document;
use krilla_macros::snapshot;
use krilla_svg::{SurfaceExt, SvgSettings};

use crate::{green_fill, load_png_image, rect_to_path, red_stroke, NOTO_SANS, SVGS_PATH};

pub trait SurfaceTaggingExt {
    fn fill_text_(&mut self, y: f32, content: &str);
    fn outline_text_(&mut self, y: f32, content: &str);
}

impl SurfaceTaggingExt for Surface<'_> {
    fn fill_text_(&mut self, y: f32, content: &str) {
        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0).unwrap();

        self.draw_text(
            Point::from_xy(0.0, y),
            font,
            20.0,
            content,
            false,
            TextDirection::Auto,
        );
    }

    fn outline_text_(&mut self, y: f32, content: &str) {
        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0).unwrap();

        self.draw_text(
            Point::from_xy(0.0, y),
            font,
            20.0,
            content,
            true,
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
    let id = surface.start_tagged(ContentTag::Span(SpanTag {
        lang: Some("en"),
        alt_text: Some("an alt text"),
        expanded: Some("expanded"),
        actual_text: Some("actual text"),
    }));
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
    let id = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.fill_text_(25.0, "a paragraph");
    surface.end_tagged();

    surface.finish();

    let link_id = page.add_tagged_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(0.0, 0.0, 100.0, 25.0).unwrap(),
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
    surface.draw_svg(
        &tree,
        Size::from_wh(tree.size().width(), tree.size().height()).unwrap(),
        SvgSettings::default(),
    );
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
    let id1 = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.fill_text_(25.0, "a span");
    surface.end_tagged();
    let id2 = surface.start_tagged(ContentTag::Artifact(ArtifactType::Header));
    surface.fill_text_(50.0, "a header artifact");
    surface.end_tagged();
    let id3 = surface.start_tagged(ContentTag::Other);
    surface.draw_path(&rect_to_path(50.0, 50.0, 100.0, 100.0));
    surface.end_tagged();

    let id4 = surface.start_tagged(ContentTag::Other);
    let tree = sample_svg();
    surface.push_transform(&Transform::from_translate(100.0, 100.0));
    surface.draw_svg(
        &tree,
        Size::from_wh(tree.size().width(), tree.size().height()).unwrap(),
        SvgSettings::default(),
    );
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
    let mut heading_1 = TagGroup::new(Tag::Hn(
        NonZeroU32::new(1).unwrap(),
        Some("first heading".into()),
    ));
    let mut heading_2 = TagGroup::new(Tag::Hn(
        NonZeroU32::new(1).unwrap(),
        Some("second heading".into()),
    ));

    let mut page = document.start_page();
    let mut surface = page.surface();
    let h1 = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.fill_text_(25.0, "a heading");
    surface.end_tagged();
    let p1 = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.fill_text_(50.0, "a paragraph");
    surface.end_tagged();
    surface.finish();
    page.finish();

    let mut page = document.start_page();
    let mut surface = page.surface();
    let p2 = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.fill_text_(75.0, "a second paragraph");
    surface.end_tagged();
    surface.finish();
    page.finish();

    let mut page = document.start_page();
    let mut surface = page.surface();
    let h2 = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
    surface.fill_text_(25.0, "another heading");
    surface.end_tagged();
    let p3 = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
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
fn tagging_heading_level_7_and_8_pdf_17(document: &mut Document) {
    tagging_heading_level_7_and_8_impl(document);
}

#[snapshot(document, settings_25)]
fn tagging_heading_level_7_and_8_pdf_20(document: &mut Document) {
    tagging_heading_level_7_and_8_impl(document);
}

fn tagging_heading_level_7_and_8_impl(document: &mut Document) {
    let mut tag_tree = TagTree::new();
    let mut page = document.start_page();
    let mut surface = page.surface();
    let mut offset = 25.0;

    let mut new_heading = |level, name| {
        let hn = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
        surface.fill_text_(offset, name);
        offset += 25.0;
        surface.end_tagged();

        let level = NonZeroU32::new(level).unwrap();
        let mut heading = TagGroup::new(Tag::Hn(level, Some(name.into())));
        heading.push(hn);

        let mut sect = TagGroup::new(Tag::Section);
        sect.push(heading);

        sect
    };

    let mut sect_1 = new_heading(1, "first heading");
    let mut sect_2 = new_heading(2, "second heading");
    let mut sect_3 = new_heading(3, "third heading");
    let mut sect_4 = new_heading(4, "fourth heading");
    let mut sect_5 = new_heading(5, "fifth heading");
    let mut sect_6 = new_heading(6, "sixth heading");
    let mut sect_7 = new_heading(7, "senventh heading");
    let sect_8 = new_heading(8, "eigth heading");

    surface.finish();
    page.finish();

    sect_7.push(sect_8);
    sect_6.push(sect_7);
    sect_5.push(sect_6);
    sect_4.push(sect_5);
    sect_3.push(sect_4);
    sect_2.push(sect_3);
    sect_1.push(sect_2);

    tag_tree.push(sect_1);

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
    surface.set_fill(Some(green_fill(1.0)));
    surface.draw_path(&rect_to_path(50.0, 50.0, 100.0, 100.0));
    surface.end_tagged();

    let id2 = surface.start_tagged(ContentTag::Other);
    surface.set_fill(Some(green_fill(1.0)));
    surface.draw_path(&rect_to_path(100.0, 100.0, 150.0, 150.0));
    surface.end_tagged();

    surface.finish();
    page.finish();

    fn_group_1.push(id1);
    fn_group_2.push(id2);
    tag_tree.push(fn_group_1);
    tag_tree.push(fn_group_2);

    document.set_tag_tree(tag_tree);
}

#[snapshot(document)]
fn tagging_table_header_and_footer(document: &mut Document) {
    let mut tag_tree = TagTree::new();
    let mut page = document.start_page();
    let mut surface = page.surface();

    let header_id = |x: usize| TagId::from(format!("Header {x}").into_bytes());
    let cell_text = |surface: &mut Surface, x: usize, y: usize, content: &str| {
        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0).unwrap();

        surface.draw_text(
            Point::from_xy(x as f32 * 200.0, y as f32 * 100.0 + 50.0),
            font,
            20.0,
            content,
            false,
            TextDirection::Auto,
        );
    };

    let header = {
        let mut row = TagGroup::new(Tag::TR);
        for x in 0..3 {
            let text = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
            cell_text(&mut surface, x, 0, &format!("heading {}", x + 1));
            surface.end_tagged();

            let tag = Tag::TH(TableHeaderScope::Column).with_id(Some(header_id(x)));
            row.push(TagGroup::with_children(tag, vec![Node::Leaf(text)]));
        }
        TagGroup::with_children(Tag::THead, vec![Node::Group(row)])
    };

    let mut body = TagGroup::new(Tag::TBody);
    for y in 1..4 {
        let mut row = TagGroup::new(Tag::TR);
        for x in 0..3 {
            let text = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
            cell_text(&mut surface, x, y, &format!("body {} {}", x + 1, y + 1));
            surface.end_tagged();

            let headers = [header_id(x)];
            let tag = Tag::TD.with_headers(headers);
            row.push(TagGroup::with_children(tag, vec![Node::Leaf(text)]));
        }
        body.push(row);
    }

    let footer = {
        let text = surface.start_tagged(ContentTag::Span(SpanTag::empty()));
        cell_text(&mut surface, 1, 4, "footer");
        surface.end_tagged();

        let cell = Tag::TD
            .with_row_span(Some(NonZeroU32::new(2).unwrap()))
            .with_col_span(Some(NonZeroU32::new(3).unwrap()))
            .with_headers((0..3).map(header_id));
        let cell = TagGroup::with_children(cell, vec![Node::Leaf(text)]);

        let row = TagGroup::with_children(Tag::TR, vec![Node::Group(cell)]);
        // Empty row to ensure proper table structure because of the rowspan.
        let empty_row = TagGroup::new(Tag::TR);
        TagGroup::with_children(Tag::TFoot, vec![row.into(), empty_row.into()])
    };

    surface.finish();
    page.finish();

    let mut table = TagGroup::new(Tag::Table.with_summary(Some("table summary".into())));
    table.push(header);
    table.push(body);
    table.push(footer);

    tag_tree.push(table);

    document.set_tag_tree(tag_tree);
}

#[snapshot(document)]
fn tagging_tag_attributes(document: &mut Document) {
    let mut tag_tree = TagTree::new();
    let mut page = document.start_page();
    let mut surface = page.surface();

    let logo = surface.start_tagged(ContentTag::Artifact(ArtifactType::Other));
    surface.outline_text_(100.0, "NASA");
    surface.end_tagged();

    surface.finish();
    page.finish();

    let figure = Tag::Figure(Some("The NASA logo".into()))
        .with_actual_text(Some("NASA".into()))
        .with_expanded(Some("National Aeronautics and Space Administration".into()))
        .with_lang(Some("en".into()));

    tag_tree.push(TagGroup::with_children(figure, vec![Node::Leaf(logo)]));

    document.set_tag_tree(tag_tree);
}

#[test]
#[should_panic]
fn tagging_page_identifer_appears_twice() {
    let mut document = Document::new();
    let mut tag_tree = TagTree::new();
    let mut fn_group_1 = TagGroup::new(Tag::P);
    let mut fn_group_2 = TagGroup::new(Tag::P);

    let mut page = document.start_page();
    let mut surface = page.surface();

    let id1 = surface.start_tagged(ContentTag::Other);
    surface.set_fill(Some(green_fill(1.0)));
    surface.draw_path(&rect_to_path(50.0, 50.0, 100.0, 100.0));
    surface.end_tagged();

    surface.finish();
    page.finish();

    fn_group_1.push(id1);
    fn_group_2.push(id1);
    tag_tree.push(fn_group_1);
    tag_tree.push(fn_group_2);

    document.set_tag_tree(tag_tree);

    let _ = document.finish();
}

#[test]
fn tagging_id_appears_twice() {
    let mut document = Document::new();
    let mut tag_tree = TagTree::new();

    let id = TagId::from(*b"one");
    let loc_1 = 1;
    let loc_2 = 2;
    let group_1 = TagGroup::new(Tag::P.with_id(Some(id.clone())).with_location(Some(loc_1)));
    let group_2 = TagGroup::new(Tag::P.with_id(Some(id.clone())).with_location(Some(loc_2)));

    tag_tree.push(group_1);
    tag_tree.push(group_2);

    document.set_tag_tree(tag_tree);

    assert_eq!(
        document.finish(),
        Err(KrillaError::DuplicateTagId(id, Some(loc_2)))
    );
}

#[test]
fn tagging_unknown_header_tag_id() {
    let mut document = Document::new();
    let mut tag_tree = TagTree::new();

    let id = TagId::from(*b"one");
    let loc_1 = 1;
    let group_1 = TagGroup::new(
        Tag::TD
            .with_headers([id.clone()])
            .with_location(Some(loc_1)),
    );

    tag_tree.push(group_1);

    document.set_tag_tree(tag_tree);

    assert_eq!(
        document.finish(),
        Err(KrillaError::UnknownTagId(id, Some(loc_1)))
    );
}

#[test]
#[should_panic]
fn tagging_annotation_identifer_appears_twice() {
    let mut document = Document::new();
    let mut tag_tree = TagTree::new();
    let mut fn_group_1 = TagGroup::new(Tag::P);
    let mut fn_group_2 = TagGroup::new(Tag::P);

    let mut page = document.start_page();
    let link_id = page.add_tagged_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(0.0, 0.0, 100.0, 25.0).unwrap(),
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

    let _ = document.finish();
}

#[test]
#[should_panic]
fn tagging_missing_identifier_in_tree() {
    let mut document = Document::new();
    let tag_tree = TagTree::new();

    let mut page = document.start_page();
    let mut surface = page.surface();

    let _ = surface.start_tagged(ContentTag::Other);
    surface.set_fill(Some(green_fill(1.0)));
    surface.draw_path(&rect_to_path(50.0, 50.0, 100.0, 100.0));
    surface.end_tagged();

    surface.finish();
    page.finish();

    document.set_tag_tree(tag_tree);

    let _ = document.finish();
}
