use crate::serialize::SerializerContext;
use pdf_writer::{Chunk, Name};

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
pub enum ContentIdentifier {
    Normal(usize, i32),
    Dummy,
}

impl ContentIdentifier {
    pub fn new(page_index: usize) -> Self {
        Self::Normal(page_index, 0)
    }

    pub fn new_dummy() -> Self {
        Self::Dummy
    }

    pub fn bump(&mut self) -> ContentIdentifier {
        let old = *self;

        match self {
            ContentIdentifier::Normal(_, num) => {
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
}

pub enum Node {
    Group(StructureGroup),
    ContentIdentifier(ContentIdentifier),
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

    pub fn serialize(&self, sc: &mut SerializerContext) -> Option<Chunk> {
        if !sc.serialize_settings.enable_tagging {
            return None;
        }
    }
}
