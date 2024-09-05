use crate::serialize::SerializerContext;
use pdf_writer::types::StructRole;
use pdf_writer::writers::StructElement;
use pdf_writer::{Chunk, Finish, Name, Ref};

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

#[derive(Copy, Clone)]
pub(crate) struct RealContentIdentifier(pub usize, pub i32);

#[derive(Copy, Clone)]
pub enum ContentIdentifier {
    Real(RealContentIdentifier),
    Dummy,
}

impl ContentIdentifier {
    pub fn new(page_index: usize) -> Self {
        Self::Real(RealContentIdentifier(page_index, 0))
    }

    pub fn new_dummy() -> Self {
        Self::Dummy
    }

    pub fn bump(&mut self) -> ContentIdentifier {
        let old = *self;

        match self {
            ContentIdentifier::Real(RealContentIdentifier(_, num)) => {
                *num = num.checked_add(1).unwrap();
            }
            ContentIdentifier::Dummy => {}
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
    ContentIdentifier(ContentIdentifier),
}

impl Node {
    pub fn serialize(
        &self,
        sc: &mut SerializerContext,
        parent: Ref,
        struct_elems: &mut Vec<Chunk>,
    ) -> Option<Reference> {
        todo!()
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
enum Reference {
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

    pub fn serialize(
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
        let mut struct_element = chunk.struct_element(root_ref);
        struct_element.kind(self.tag.into());
        let mut struct_children = struct_element.children();

        for child in children_refs {
            match child {
                Reference::Ref(r) => {
                    struct_children.struct_element(r);
                }
                Reference::ContentIdentifier(rci) => {
                    struct_children
                        .marked_content_ref()
                        .marked_content_id(rci.1)
                        // TODO: Error handling
                        .page(sc.page_infos()[rci.0].ref_);
                }
            }
        }

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

    pub fn serialize(&self, sc: &mut SerializerContext) -> Option<Vec<Chunk>> {
        if !sc.serialize_settings.enable_tagging {
            return None;
        }

        let root_ref = sc.new_ref();
        let mut chunk = Chunk::new();
        let mut struct_elem = chunk.indirect(root_ref).start::<StructElement>();
        struct_elem.kind(StructRole::Document);
        struct_elem.finish();

        let mut struct_elems = vec![];
        struct_elems.push(chunk);

        // Not strictly necessary, but it's nicer to have them in DFS-order instead
        // of in reverse.
        struct_elems = struct_elems.into_iter().rev().collect::<Vec<_>>();

        Some(struct_elems)
    }
}
