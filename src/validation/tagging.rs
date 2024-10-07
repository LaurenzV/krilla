use crate::serialize::SerializerContext;
use pdf_writer::types::{ArtifactAttachment, ArtifactSubtype, StructRole};
use pdf_writer::writers::{PropertyList, StructChildren, StructElement};
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::cmp::PartialEq;
use std::collections::HashMap;
use tiny_skia_path::Rect;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ArtifactType {
    /// The header of a page.
    Header,
    /// The footer of the page.
    Footer,
    /// Page artifacts, such as for example cut marks or color bars.
    Page,
    /// The background of a page, which might for example include a watermark.
    /// The rectangle should delimit the bounding box of the visible content of the
    /// content to be delimited as the background of the page.
    Background(Rect),
}

/// A language identifier as specified in RFC 3066. It will not be validated, so
/// it's on the user of the library to ensure the tag is valid.
pub type Lang<'a> = &'a str;
pub type Alt<'a> = &'a str;

#[derive(Clone, Copy, Debug)]
pub enum ContentTag<'a> {
    Artifact(ArtifactType),
    Span,
    Image(Option<Alt<'a>>),
    Other,
}

impl ContentTag<'_> {
    pub(crate) fn name(&self) -> Name {
        match self {
            ContentTag::Artifact(_) => Name(b"Artifact"),
            ContentTag::Span => Name(b"Span"),
            // There isn't really anything better to encode it with, so we use P as a fallback
            // alternative. Word seems to do the same.
            ContentTag::Image(_) => Name(b"P"),
            ContentTag::Other => Name(b"P"),
        }
    }

    pub(crate) fn write_properties(
        &self,
        sc: &mut SerializerContext,
        mut properties: PropertyList,
    ) {
        match self {
            ContentTag::Artifact(at) => {
                let mut artifact = properties.artifact();

                let artifact_type = match at {
                    ArtifactType::Header => pdf_writer::types::ArtifactType::Pagination,
                    ArtifactType::Footer => pdf_writer::types::ArtifactType::Pagination,
                    ArtifactType::Page => pdf_writer::types::ArtifactType::Page,
                    // TODO: Handle bbox.
                    ArtifactType::Background(_) => pdf_writer::types::ArtifactType::Background,
                };

                if *at == ArtifactType::Header {
                    artifact.attached([ArtifactAttachment::Top]);
                    artifact.subtype(ArtifactSubtype::Header);
                }

                if *at == ArtifactType::Footer {
                    artifact.attached([ArtifactAttachment::Bottom]);
                    artifact.subtype(ArtifactSubtype::Footer);
                }

                artifact.kind(artifact_type);
            }
            ContentTag::Span => {}
            ContentTag::Image(alt) => {
                // TODO Alt text cannot be written like that.
                if let Some(alt) = alt {
                    properties.pair(Name(b"Alt"), sc.new_text_str(alt));
                }
            }
            ContentTag::Other => {}
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PageTagIdentifier {
    pub(crate) page_index: usize,
    pub(crate) mcid: i32,
}

impl From<PageTagIdentifier> for IdentifierType {
    fn from(value: PageTagIdentifier) -> Self {
        IdentifierType::PageIdentifier(value)
    }
}

impl From<PageTagIdentifier> for Identifier {
    fn from(value: PageTagIdentifier) -> Self {
        Identifier(IdentifierInner::Real(value.into()))
    }
}

impl PageTagIdentifier {
    pub fn new(page_index: usize, mcid: i32) -> Self {
        Self { page_index, mcid }
    }

    pub fn bump(&mut self) -> PageTagIdentifier {
        let old = *self;

        self.mcid = self.mcid.checked_add(1).unwrap();

        old
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct AnnotationIdentifier {
    page_index: usize,
    annot_index: usize,
}

impl From<AnnotationIdentifier> for IdentifierType {
    fn from(value: AnnotationIdentifier) -> Self {
        IdentifierType::AnnotationIdentifier(value)
    }
}

impl From<AnnotationIdentifier> for Identifier {
    fn from(value: AnnotationIdentifier) -> Self {
        Identifier(IdentifierInner::Real(value.into()))
    }
}

impl AnnotationIdentifier {
    pub fn new(page_index: usize, annot_index: usize) -> Self {
        Self {
            page_index,
            annot_index,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum IdentifierType {
    PageIdentifier(PageTagIdentifier),
    AnnotationIdentifier(AnnotationIdentifier),
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum IdentifierInner {
    Real(IdentifierType),
    Dummy,
}

#[derive(Copy, Clone)]
pub struct Identifier(pub(crate) IdentifierInner);

impl Identifier {
    pub(crate) fn new_annotation(page_index: usize, annot_index: usize) -> Self {
        AnnotationIdentifier::new(page_index, annot_index).into()
    }

    pub(crate) fn dummy() -> Self {
        Self(IdentifierInner::Dummy)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Tag {
    /// A part of a document that may contain multiple articles or sections.
    Part,
    /// An article with largely self-contained content.
    Article,
    /// Section of a larger document.
    Section,
    /// A paragraph-level quote.
    BlockQuote,
    /// An image or figure caption.
    ///
    /// **Best Practice**: In the tag tree, this should appear
    /// as a sibling after the image (or other) content it describes.
    Caption,
    /// Table of contents.
    ///
    /// **Best Practice**: Should consist of TOCIs or other nested TOCs.
    TOC,
    /// Item in the table of contents.
    ///
    /// **Best Practice**: Should only appear within a TOC. Should only consist of
    /// labels, references, paragraphs and TOCs.
    TOCI,
    /// Index of the key terms in the document.
    ///
    /// **Best Practice**: Should contain a sequence of text accompanied by
    /// reference elements pointing to their occurrence in the text.
    Index,
    /// A paragraph.
    P,
    /// First-level heading.
    H1,
    /// Second-level heading.
    H2,
    /// Third-level heading.
    H3,
    /// Fourth-level heading.
    H4,
    /// Fifth-level heading.
    H5,
    /// Sixth-level heading.
    H6,
    /// A list.
    ///
    /// **Best practice**: Should consist of an optional caption followed by
    /// list items.
    L,
    /// A list item.
    ///
    /// **Best practice**: Should consist of one or more list labels and/or list bodies.
    LI,
    /// Label for a list item.
    Lbl,
    /// Description of the list item.
    LBody,
    /// A table.
    ///
    /// **Best practice**: Should consist of an optional table header row,
    /// one or more table body elements and an optional table footer. Can have
    /// caption as the first or last child.
    Table,
    /// A table row.
    ///
    /// **Best practice**: May contain table headers cells and table data cells.
    TR,
    /// A table header cell.
    TH,
    /// A table data cell.
    TD,
    /// A table header row group.
    THead,
    /// A table data row group.
    TBody,
    /// A table footer row group.
    TFoot,
    /// An inline quotation.
    InlineQuote,
    /// A foot- or endnote, potentially referred to from within the text.
    ///
    /// **Best practice**: It may have a label as a child.
    Note,
    /// A reference to elsewhere in the document.
    Reference,
    /// A reference to the external source of some cited document.
    ///
    /// **Best practice**: It may have a label as a child.
    BibEntry,
    /// Computer code.
    Code,
    /// A link.
    Link,
    /// An association between an annotation and the content it belongs to. PDF
    /// 1.5+
    Annot,
    /// Item of graphical content.
    Figure,
    /// A mathematical formula.
    Formula,
}

impl From<Tag> for StructRole {
    fn from(value: Tag) -> Self {
        match value {
            Tag::Part => StructRole::Part,
            Tag::Article => StructRole::Art,
            Tag::Section => StructRole::Sect,
            Tag::BlockQuote => StructRole::BlockQuote,
            Tag::Caption => StructRole::Caption,
            Tag::TOC => StructRole::TOC,
            Tag::TOCI => StructRole::TOCI,
            Tag::Index => StructRole::Index,
            Tag::P => StructRole::P,
            Tag::H1 => StructRole::H1,
            Tag::H2 => StructRole::H2,
            Tag::H3 => StructRole::H3,
            Tag::H4 => StructRole::H4,
            Tag::H5 => StructRole::H5,
            Tag::H6 => StructRole::H6,
            Tag::L => StructRole::L,
            Tag::LI => StructRole::LI,
            Tag::Lbl => StructRole::Lbl,
            Tag::LBody => StructRole::LBody,
            Tag::Table => StructRole::Table,
            Tag::TR => StructRole::TR,
            Tag::TH => StructRole::TH,
            Tag::TD => StructRole::TD,
            Tag::THead => StructRole::THead,
            Tag::TBody => StructRole::TBody,
            Tag::TFoot => StructRole::TFoot,
            Tag::InlineQuote => StructRole::Quote,
            Tag::Note => StructRole::Note,
            Tag::Reference => StructRole::Reference,
            Tag::BibEntry => StructRole::BibEntry,
            Tag::Code => StructRole::Code,
            Tag::Link => StructRole::Link,
            Tag::Annot => StructRole::Annot,
            Tag::Figure => StructRole::Figure,
            Tag::Formula => StructRole::Formula,
        }
    }
}

pub enum Node {
    Group(TagGroup),
    Leaf(Identifier),
}

impl Node {
    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        parent_tree_map: &mut HashMap<IdentifierType, Ref>,
        parent: Ref,
        struct_elems: &mut Vec<Chunk>,
    ) -> Option<Reference> {
        match self {
            Node::Group(g) => Some(g.serialize(sc, parent_tree_map, parent, struct_elems)),
            Node::Leaf(ci) => match ci.0 {
                IdentifierInner::Real(rci) => Some(Reference::ContentIdentifier(rci)),
                IdentifierInner::Dummy => None,
            },
        }
    }
}

impl From<TagGroup> for Node {
    fn from(value: TagGroup) -> Self {
        Node::Group(value)
    }
}

impl From<Identifier> for Node {
    fn from(value: Identifier) -> Self {
        Node::Leaf(value)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Reference {
    Ref(Ref),
    ContentIdentifier(IdentifierType),
}

pub struct TagGroup {
    tag: Tag,
    children: Vec<Node>,
}

impl TagGroup {
    pub fn new(tag: Tag) -> Self {
        Self {
            tag,
            children: vec![],
        }
    }

    pub fn push(&mut self, child: impl Into<Node>) {
        self.children.push(child.into())
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        parent_tree_map: &mut HashMap<IdentifierType, Ref>,
        parent: Ref,
        struct_elems: &mut Vec<Chunk>,
    ) -> Reference {
        let root_ref = sc.new_ref();
        let children_refs = self
            .children
            .iter()
            .flat_map(|n| n.serialize(sc, parent_tree_map, parent, struct_elems))
            .collect::<Vec<_>>();

        let mut chunk = Chunk::new();
        let mut struct_elem = chunk.struct_element(root_ref);
        struct_elem.kind(self.tag.into());
        struct_elem.parent(parent);
        let struct_children = struct_elem.children();
        serialize_children(
            sc,
            root_ref,
            children_refs,
            parent_tree_map,
            struct_children,
        );
        struct_elem.finish();
        struct_elems.push(chunk);

        Reference::Ref(root_ref)
    }
}

pub struct TagTree {
    children: Vec<Node>,
}

impl TagTree {
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    pub fn push(&mut self, child: impl Into<Node>) {
        self.children.push(child.into())
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        parent_tree_map: &mut HashMap<IdentifierType, Ref>,
        struct_tree_ref: Ref,
    ) -> (Ref, Vec<Chunk>) {
        let root_ref = sc.new_ref();
        let mut struct_elems = vec![];

        let children_refs = self
            .children
            .iter()
            .flat_map(|n| n.serialize(sc, parent_tree_map, root_ref, &mut struct_elems))
            .collect::<Vec<_>>();

        let mut chunk = Chunk::new();
        let mut struct_elem = chunk.indirect(root_ref).start::<StructElement>();
        struct_elem.kind(StructRole::Document);
        struct_elem.parent(struct_tree_ref);
        let struct_children = struct_elem.children();
        serialize_children(
            sc,
            root_ref,
            children_refs,
            parent_tree_map,
            struct_children,
        );

        struct_elem.finish();
        struct_elems.push(chunk);

        // Not strictly necessary, but it's nicer to have them in DFS-order instead
        // of in reverse.
        struct_elems = struct_elems.into_iter().rev().collect::<Vec<_>>();

        (root_ref, struct_elems)
    }
}

fn serialize_children(
    sc: &mut SerializerContext,
    root_ref: Ref,
    children_refs: Vec<Reference>,
    parent_tree_map: &mut HashMap<IdentifierType, Ref>,
    mut struct_children: StructChildren,
) {
    for child in children_refs {
        match child {
            Reference::Ref(r) => {
                struct_children.struct_element(r);
            }
            Reference::ContentIdentifier(it) => {
                match it {
                    IdentifierType::PageIdentifier(pi) => {
                        // TODO: Ensure that pi doesn't already have a parent.
                        parent_tree_map.insert(pi.into(), root_ref);

                        struct_children
                            .marked_content_ref()
                            .marked_content_id(pi.mcid)
                            // TODO: Error handling
                            .page(sc.page_infos()[pi.page_index].ref_);
                    }
                    IdentifierType::AnnotationIdentifier(_) => {
                        unimplemented!()
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::font::Font;
    use crate::path::Fill;
    use crate::surface::{Surface, TextDirection};
    use crate::tests::{load_png_image, rect_to_path, NOTO_SANS, SVGS_PATH};
    use crate::validation::tagging::{ArtifactType, ContentTag, Tag, TagGroup, TagTree};
    use crate::{Document, SvgSettings};
    use krilla_macros::snapshot;
    use tiny_skia_path::Transform;

    pub trait SurfaceExt {
        fn fill_text_(&mut self, y: f32, content: &str);
    }

    impl SurfaceExt for Surface<'_> {
        fn fill_text_(&mut self, y: f32, content: &str) {
            let font_data = NOTO_SANS.clone();
            let font = Font::new(font_data, 0, vec![]).unwrap();

            self.fill_text(
                tiny_skia_path::Point::from_xy(0.0, y),
                Fill::default(),
                font,
                20.0,
                &[],
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
        let id = surface.start_tagged(ContentTag::Span);
        surface.fill_text_(25.0, "a paragraph");
        surface.end_tagged();

        surface.finish();
        page.finish();

        par.push(id);
        tag_tree.push(par);

        document.set_tag_tree(tag_tree);
    }

    #[snapshot(document)]
    fn tagging_simple(document: &mut Document) {
        tagging_simple_impl(document);
    }

    #[snapshot(document, settings_12)]
    fn tagging_disabled(document: &mut Document) {
        tagging_simple_impl(document);
    }

    pub(crate) fn sample_svg() -> usvg::Tree {
        let data = std::fs::read(SVGS_PATH.join("resvg_shapes_rect_simple_case.svg")).unwrap();
        usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap()
    }

    #[snapshot(document)]
    fn tagging_multiple_content_tags(document: &mut Document) {
        let mut tag_tree = TagTree::new();

        let mut page = document.start_page();
        let mut surface = page.surface();
        let id1 = surface.start_tagged(ContentTag::Span);
        surface.fill_text_(25.0, "a span");
        surface.end_tagged();
        let id2 = surface.start_tagged(ContentTag::Artifact(ArtifactType::Header));
        surface.fill_text_(50.0, "a header artifact");
        surface.end_tagged();
        let id3 = surface.start_tagged(ContentTag::Other);
        surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), Fill::default());
        surface.end_tagged();

        let id4 = surface.start_tagged(ContentTag::Image(Some("A super cool SVG image.")));
        let tree = sample_svg();
        surface.push_transform(&Transform::from_translate(100.0, 100.0));
        surface.draw_svg(&tree, tree.size(), SvgSettings::default());
        surface.pop();
        surface.end_tagged();

        let id5 = surface.start_tagged(ContentTag::Image(Some("A super cool bitmap image.")));
        let image = load_png_image("rgb8.png");
        surface.push_transform(&Transform::from_translate(100.0, 300.0));
        surface.draw_image(image.clone(), image.size());
        surface.pop();
        surface.end_tagged();

        surface.finish();
        page.finish();

        tag_tree.push(id1);
        tag_tree.push(id2);
        tag_tree.push(id3);
        tag_tree.push(id4);
        tag_tree.push(id5);

        document.set_tag_tree(tag_tree);
    }

    #[snapshot(document)]
    fn tagging_multiple_pages(document: &mut Document) {
        let mut tag_tree = TagTree::new();
        let mut par_1 = TagGroup::new(Tag::P);
        let mut par_2 = TagGroup::new(Tag::P);
        let mut heading_1 = TagGroup::new(Tag::H1);
        let mut heading_2 = TagGroup::new(Tag::H1);

        let mut page = document.start_page();
        let mut surface = page.surface();
        let h1 = surface.start_tagged(ContentTag::Span);
        surface.fill_text_(25.0, "a heading");
        surface.end_tagged();
        let p1 = surface.start_tagged(ContentTag::Span);
        surface.fill_text_(50.0, "a paragraph");
        surface.end_tagged();
        surface.finish();
        page.finish();

        let mut page = document.start_page();
        let mut surface = page.surface();
        let p2 = surface.start_tagged(ContentTag::Span);
        surface.fill_text_(75.0, "a second paragraph");
        surface.end_tagged();
        surface.finish();
        page.finish();

        let mut page = document.start_page();
        let mut surface = page.surface();
        let h2 = surface.start_tagged(ContentTag::Span);
        surface.fill_text_(25.0, "another heading");
        surface.end_tagged();
        let p3 = surface.start_tagged(ContentTag::Span);
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
}
