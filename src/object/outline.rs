use pdf_writer::{Chunk, Ref};
use tiny_skia_path::Point;
use crate::serialize::{Object, SerializerContext};

#[derive(Debug)]
pub struct Outline {
    children: Vec<OutlineNode>,
}

impl Outline {
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    pub fn push_child(&mut self, node: OutlineNode) {
        self.children.push(node)
    }
}

#[derive(Debug)]
struct OutlineNode {
    children: Vec<Box<OutlineNode>>,
    text: String,
    page_index: u32,
    pos: Point,
}

impl OutlineNode {
    pub fn new(text: String, page_index: u32, pos: Point) -> Self {
        Self {
            children: vec![],
            text,
            page_index,
            pos,
        }
    }

    pub fn push_child(&mut self, node: OutlineNode) {
        self.children.push(Box::new(node))
    }


}

impl Object for Outline {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        todo!()
    }
}