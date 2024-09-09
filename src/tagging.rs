use crate::serialize::SerializerContext;
use pdf_writer::types::StructRole;
use pdf_writer::writers::StructElement;
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::collections::{BTreeMap, HashMap};

pub enum ContentTag {
    Span,
    Figure,
}

impl ContentTag {
    pub fn name(&self) -> Name {
        match self {
            ContentTag::Span => Name(b"Span"),
            ContentTag::Figure => Name(b"Figure"),
        }
    }
}

#[derive(Copy, Clone, Eq, Hash, PartialEq, Debug, Ord, PartialOrd)]
pub(crate) struct RealContentIdentifier(pub usize, pub i32);

#[derive(Copy, Clone)]
pub(crate) enum ContentIdentifierEnum {
    Real(RealContentIdentifier),
    Dummy,
}

#[derive(Copy, Clone)]
pub struct ContentIdentifier(pub(crate) ContentIdentifierEnum);

impl ContentIdentifierEnum {
    pub fn new(page_index: usize) -> Self {
        Self::Real(RealContentIdentifier(page_index, 0))
    }

    pub fn new_dummy() -> Self {
        Self::Dummy
    }

    pub fn bump(&mut self) -> ContentIdentifierEnum {
        let old = *self;

        match self {
            ContentIdentifierEnum::Real(RealContentIdentifier(_, num)) => {
                *num = num.checked_add(1).unwrap();
            }
            ContentIdentifierEnum::Dummy => {}
        }

        old
    }
}

#[derive(Debug, Clone, Copy)]
pub enum StructureTag {
    Paragraph,
    TOC,
    List,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl From<StructureTag> for StructRole {
    fn from(value: StructureTag) -> Self {
        match value {
            StructureTag::Paragraph => StructRole::P,
            StructureTag::TOC => StructRole::TOC,
            StructureTag::H1 => StructRole::H1,
            StructureTag::H2 => StructRole::H2,
            StructureTag::H3 => StructRole::H3,
            StructureTag::H4 => StructRole::H4,
            StructureTag::H5 => StructRole::H5,
            StructureTag::H6 => StructRole::H6,
            StructureTag::List => StructRole::L,
        }
    }
}

pub enum Node {
    Group(StructureGroup),
    ContentIdentifier(ContentIdentifier),
}

impl Node {
    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        parent_tree_map: &mut BTreeMap<RealContentIdentifier, Ref>,
        parent: Ref,
        struct_elems: &mut Vec<Chunk>,
    ) -> Option<Reference> {
        match self {
            Node::Group(g) => Some(g.serialize(sc, parent_tree_map, parent, struct_elems)),
            Node::ContentIdentifier(ci) => match ci.0 {
                ContentIdentifierEnum::Real(rci) => Some(Reference::ContentIdentifier(rci)),
                ContentIdentifierEnum::Dummy => None,
            },
        }
    }
}

impl From<StructureGroup> for Node {
    fn from(value: StructureGroup) -> Self {
        Node::Group(value)
    }
}

impl From<ContentIdentifier> for Node {
    fn from(value: ContentIdentifier) -> Self {
        Node::ContentIdentifier(value)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Reference {
    Ref(Ref),
    ContentIdentifier(RealContentIdentifier),
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
        parent_tree_map: &mut BTreeMap<RealContentIdentifier, Ref>,
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
        let mut struct_element = chunk.struct_element(root_ref);
        struct_element.kind(self.tag.into());
        struct_element.parent(parent);
        let mut struct_children = struct_element.children();

        for child in children_refs {
            match child {
                Reference::Ref(r) => {
                    struct_children.struct_element(r);
                }
                Reference::ContentIdentifier(rci) => {
                    parent_tree_map.insert(rci, root_ref);
                    struct_children
                        .marked_content_ref()
                        .marked_content_id(rci.1)
                        // TODO: Error handling
                        .page(sc.page_infos()[rci.0].ref_);
                }
            }
        }

        struct_children.finish();
        struct_element.finish();
        struct_elems.push(chunk);

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
        parent_tree_map: &mut BTreeMap<RealContentIdentifier, Ref>,
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
        let mut struct_children = struct_elem.children();

        for child in children_refs {
            match child {
                Reference::Ref(r) => {
                    struct_children.struct_element(r);
                }
                Reference::ContentIdentifier(rci) => {
                    parent_tree_map.insert(rci, root_ref);
                    struct_children
                        .marked_content_ref()
                        .marked_content_id(rci.1)
                        // TODO: Error handling
                        .page(sc.page_infos()[rci.0].ref_);
                }
            }
        }

        struct_children.finish();
        struct_elem.finish();
        struct_elems.push(chunk);

        // Not strictly necessary, but it's nicer to have them in DFS-order instead
        // of in reverse.
        struct_elems = struct_elems.into_iter().rev().collect::<Vec<_>>();

        (root_ref, struct_elems)
    }
}
