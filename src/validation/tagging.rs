use crate::serialize::SerializerContext;
use pdf_writer::types::StructRole;
use pdf_writer::writers::{StructChildren, StructElement};
use pdf_writer::{Chunk, Finish, Name, Ref};
use tiny_skia_path::Rect;

#[derive(Copy, Clone, Debug)]
pub enum Attachment {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Copy, Clone, Debug)]
pub enum ArtifactType {
    Pagination,
    Page,
    Layout,
    Background(Rect),
}

#[derive(Copy, Clone, Debug)]
pub enum ContentTag {
    Artifact(ArtifactType),
    Span,
    Other,
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum IdentifierType {
    PageIdentifier(usize, i32),
    AnnotationIdentifier(usize, usize),
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum IdentifierInner {
    Real(IdentifierType),
    Dummy,
}

#[derive(Copy, Clone)]
pub struct Identifier(pub(crate) IdentifierInner);

impl Identifier {
    pub(crate) fn new_page(page_index: usize) -> Self {
        Self(IdentifierInner::Real(IdentifierType::PageIdentifier(
            page_index, 0,
        )))
    }

    pub(crate) fn new_annotation(page_index: usize) -> Self {
        Self(IdentifierInner::Real(IdentifierType::AnnotationIdentifier(
            page_index, 0,
        )))
    }

    pub(crate) fn new_dummy() -> Self {
        Self(IdentifierInner::Dummy)
    }

    pub fn bump(&mut self) -> Identifier {
        let old = *self;

        match &mut self.0 {
            IdentifierInner::Real(rc) => match rc {
                IdentifierType::PageIdentifier(_, i) => {
                    *i = i.checked_add(1).unwrap();
                }
                IdentifierType::AnnotationIdentifier(_, i) => {
                    *i = i.checked_add(1).unwrap();
                }
            },
            IdentifierInner::Dummy => {}
        }

        old
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StructureTag {
    Paragraph,
}

impl From<StructureTag> for StructRole {
    fn from(value: StructureTag) -> Self {
        match value {
            StructureTag::Paragraph => StructRole::P,
        }
    }
}

pub enum Node {
    Group(StructureGroup),
    Leaf(Identifier),
}

impl Node {
    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        parent: Ref,
        struct_elems: &mut Vec<Chunk>,
    ) -> Option<Reference> {
        match self {
            Node::Group(g) => Some(g.serialize(sc, parent, struct_elems)),
            Node::Leaf(ci) => match ci.0 {
                IdentifierInner::Real(rci) => Some(Reference::ContentIdentifier(rci)),
                IdentifierInner::Dummy => None,
            },
        }
    }
}

impl From<StructureGroup> for Node {
    fn from(value: StructureGroup) -> Self {
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

pub struct StructureGroup {
    tag: StructureTag,
    children: Vec<Node>,
}

impl StructureGroup {
    pub fn new(tag: StructureTag) -> Self {
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
        parent: Ref,
        struct_elems: &mut Vec<Chunk>,
    ) -> Reference {
        let root_ref = sc.new_ref();
        let children_refs = self
            .children
            .iter()
            .flat_map(|n| n.serialize(sc, parent, struct_elems))
            .collect::<Vec<_>>();

        let mut chunk = Chunk::new();
        let mut struct_elem = chunk.struct_element(root_ref);
        struct_elem.kind(self.tag.into());
        struct_elem.parent(parent);
        let struct_children = struct_elem.children();
        serialize_children(sc, children_refs, struct_children);

        Reference::Ref(root_ref)
    }
}

pub struct StructureRoot {
    children: Vec<Node>,
}

impl StructureRoot {
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    pub fn push(&mut self, child: impl Into<Node>) {
        self.children.push(child.into())
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        struct_tree_ref: Ref,
    ) -> Option<Vec<Chunk>> {
        if !sc.serialize_settings.enable_tagging {
            return None;
        }

        let root_ref = sc.new_ref();
        let mut struct_elems = vec![];

        let children_refs = self
            .children
            .iter()
            .flat_map(|n| n.serialize(sc, root_ref, &mut struct_elems))
            .collect::<Vec<_>>();

        let mut chunk = Chunk::new();
        let mut struct_elem = chunk.indirect(root_ref).start::<StructElement>();
        struct_elem.kind(StructRole::Document);
        struct_elem.parent(struct_tree_ref);
        let struct_children = struct_elem.children();
        serialize_children(sc, children_refs, struct_children);

        struct_elem.finish();
        struct_elems.push(chunk);

        // Not strictly necessary, but it's nicer to have them in DFS-order instead
        // of in reverse.
        struct_elems = struct_elems.into_iter().rev().collect::<Vec<_>>();

        Some(struct_elems)
    }
}

fn serialize_children(
    sc: &mut SerializerContext,
    children_refs: Vec<Reference>,
    mut struct_children: StructChildren,
) {
    for child in children_refs {
        match child {
            Reference::Ref(r) => {
                struct_children.struct_element(r);
            }
            Reference::ContentIdentifier(it) => {
                match it {
                    IdentifierType::PageIdentifier(pi, ci) => {
                        struct_children
                            .marked_content_ref()
                            .marked_content_id(ci)
                            // TODO: Error handling
                            .page(sc.page_infos()[pi].ref_);
                    }
                    IdentifierType::AnnotationIdentifier(_, _) => {
                        unimplemented!()
                    }
                }
            }
        }
    }
}
