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
    /// The rect should delimit the bounding box of the visible content of the
    /// content to be delimited as the background of the page.
    Background(Rect),
}

/// A language identifier as specified in RFC 3066. It will not be validated, so
/// it's on the user of the library to ensure the tag is valid.
pub type Lang<'a> = &'a str;

#[derive(Copy, Clone, Debug)]
pub enum ContentTag {
    Artifact(ArtifactType),
    Span,
    Other,
}

impl ContentTag {
    pub(crate) fn name(&self) -> Name {
        match self {
            ContentTag::Artifact(_) => Name(b"Artifact"),
            ContentTag::Span => Name(b"Span"),
            ContentTag::Other => Name(b"P"),
        }
    }

    pub(crate) fn write_properties(&self, _: &mut SerializerContext, mut properties: PropertyList) {
        match self {
            ContentTag::Artifact(at) => {
                let mut artifact = properties.artifact();

                let artifact_type = match at {
                    ArtifactType::Header => pdf_writer::types::ArtifactType::Pagination,
                    ArtifactType::Footer => pdf_writer::types::ArtifactType::Pagination,
                    ArtifactType::Page => pdf_writer::types::ArtifactType::Page,
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
            ContentTag::Other => {}
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PageIdentifier {
    pub(crate) page_index: usize,
    pub(crate) mcid: i32,
}

impl From<PageIdentifier> for IdentifierType {
    fn from(value: PageIdentifier) -> Self {
        IdentifierType::PageIdentifier(value)
    }
}

impl From<PageIdentifier> for Identifier {
    fn from(value: PageIdentifier) -> Self {
        Identifier(IdentifierInner::Real(value.into()))
    }
}

impl PageIdentifier {
    pub fn new(page_index: usize, mcid: i32) -> Self {
        Self { page_index, mcid }
    }

    pub fn bump(&mut self) -> PageIdentifier {
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
    PageIdentifier(PageIdentifier),
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
    Paragraph,
}

impl From<Tag> for StructRole {
    fn from(value: Tag) -> Self {
        match value {
            Tag::Paragraph => StructRole::P,
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

pub struct TagRoot {
    children: Vec<Node>,
}

impl TagRoot {
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
    use crate::tests::NOTO_SANS;
    use crate::validation::tagging::{ContentTag, Tag, TagGroup, TagRoot};
    use crate::Document;
    use krilla_macros::snapshot;

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

    #[snapshot(document, settings_12)]
    fn tagging_simple(document: &mut Document) {
        let mut tag_root = TagRoot::new();
        let mut par = TagGroup::new(Tag::Paragraph);

        let mut page = document.start_page();
        let mut surface = page.surface();
        let id = surface.start_tagged(ContentTag::Span);
        surface.fill_text_(25.0, "a paragraph");
        surface.end_tagged();

        surface.finish();
        page.finish();

        par.push(id);
        tag_root.push(par);

        document.set_tag_tree(tag_root);
    }
}
