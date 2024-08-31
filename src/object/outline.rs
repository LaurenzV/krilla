use crate::error::KrillaResult;
use crate::object::destination::XyzDestination;
use crate::serialize::{Object, SerializerContext};
use pdf_writer::{Chunk, Finish, Name, Ref, TextStr};
use tiny_skia_path::Point;

#[derive(Debug, Clone)]
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

    pub(crate) fn serialize_into(
        &self,
        sc: &mut SerializerContext,
        root_ref: Ref,
    ) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let mut sub_chunks = vec![];

        let mut outline = chunk.outline(root_ref);

        if !self.children.is_empty() {
            let first = sc.new_ref();
            let mut last = first;

            let mut prev = None;
            let mut cur = Some(first);

            for i in 0..self.children.len() {
                let next = if i < self.children.len() - 1 {
                    Some(sc.new_ref())
                } else {
                    None
                };

                last = cur.unwrap();

                sub_chunks.push(self.children[i].serialize_into(sc, root_ref, last, next, prev)?);

                prev = cur;
                cur = next;
            }

            outline.first(first);
            outline.last(last);
            outline.count(i32::try_from(self.children.len()).unwrap());
        }

        outline.finish();

        for sub_chunk in sub_chunks {
            chunk.extend(&sub_chunk);
        }

        Ok(chunk)
    }
}

#[derive(Debug, Clone)]
pub struct OutlineNode {
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

    pub(crate) fn serialize_into(
        &self,
        sc: &mut SerializerContext,
        parent: Ref,
        root: Ref,
        next: Option<Ref>,
        prev: Option<Ref>,
    ) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let mut sub_chunks = vec![];

        let mut outline_entry = chunk.outline_item(root);
        outline_entry.parent(parent);

        if let Some(next) = next {
            outline_entry.next(next);
        }

        if let Some(prev) = prev {
            outline_entry.prev(prev);
        }

        if !self.children.is_empty() {
            let first = sc.new_ref();
            let mut last = first;

            let mut prev = None;
            let mut cur = Some(first);

            for i in 0..self.children.len() {
                let next = if i < self.children.len() - 1 {
                    Some(sc.new_ref())
                } else {
                    None
                };

                last = cur.unwrap();

                sub_chunks.push(self.children[i].serialize_into(sc, root, last, next, prev)?);

                prev = cur;
                cur = next;
            }

            outline_entry.first(first);
            outline_entry.last(last);
            outline_entry.count(-i32::try_from(self.children.len()).unwrap());
        }

        if !self.text.is_empty() {
            outline_entry.title(TextStr(&self.text));
        }

        let dest = XyzDestination::new(self.page_index as usize, self.pos);
        let dest_ref = sc.new_ref();
        sub_chunks.push(dest.serialize_into(sc, dest_ref)?);

        outline_entry.pair(Name(b"Dest"), dest_ref);

        outline_entry.finish();

        for sub_chunk in sub_chunks {
            chunk.extend(&sub_chunk);
        }

        Ok(chunk)
    }
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::object::outline::{Outline, OutlineNode};
    use crate::rgb::Rgb;

    use crate::Fill;
    use krilla_macros::snapshot;
    use tiny_skia_path::{PathBuilder, Point, Rect, Size};

    #[snapshot(document)]
    fn outline_simple(db: &mut Document) {
        let mut builder = PathBuilder::new();
        builder.push_rect(Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap());
        let path = builder.finish().unwrap();

        let mut page = db.start_page(Size::from_wh(200.0, 200.0).unwrap());
        let mut surface = page.surface();
        surface.fill_path(&path, Fill::<Rgb>::default());
        surface.finish();
        page.finish();

        db.start_page(Size::from_wh(200.0, 500.0).unwrap());
        db.start_page(Size::from_wh(250.0, 700.0).unwrap());

        let mut outline = Outline::new();

        let mut child1 = OutlineNode::new("Level 1".to_string(), 0, Point::from_xy(50.0, 50.0));
        child1.push_child(OutlineNode::new(
            "Level 2".to_string(),
            0,
            Point::from_xy(50.0, 150.0),
        ));

        let child2 = OutlineNode::new("Level 1 try 2".to_string(), 1, Point::from_xy(75.0, 150.0));

        outline.push_child(child1);
        outline.push_child(child2);

        db.set_outline(outline);
    }
}
