use crate::serialize::{Object, SerializerContext};
use pdf_writer::{Chunk, Finish, Ref, TextStr};
use tiny_skia_path::{Point, Transform};

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

    pub fn serialize_into(
        &self,
        sc: &mut SerializerContext,
        parent: Ref,
        root: Ref,
        next: Option<Ref>,
        prev: Option<Ref>,
    ) -> Chunk {
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

                sub_chunks.push(self.children[i].serialize_into(sc, root, last, next, prev));

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

        let page_ref = sc.page_infos()[self.page_index as usize].ref_;
        let page_size = sc.page_infos()[self.page_index as usize].media_box.height();
        let mut mapped_point = self.pos;
        let invert_transform = Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, page_size);
        invert_transform.map_point(&mut mapped_point);

        outline_entry
            .dest()
            .page(page_ref)
            .xyz(mapped_point.x, mapped_point.y, None);

        outline_entry.finish();

        for sub_chunk in sub_chunks {
            chunk.extend(&sub_chunk);
        }

        chunk
    }
}

impl Object for Outline {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
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

                sub_chunks.push(self.children[i].serialize_into(sc, root_ref, last, next, prev));

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

        chunk
    }
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::object::outline::{Outline, OutlineNode};
    use crate::rgb::Rgb;
    use crate::serialize::SerializeSettings;
    use crate::test_utils::check_snapshot;
    use crate::Fill;
    use tiny_skia_path::{PathBuilder, Point, Rect, Size};

    #[test]
    fn simple() {
        let mut builder = PathBuilder::new();
        builder.push_rect(Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap());
        let path = builder.finish().unwrap();

        let mut db = Document::new(SerializeSettings::default_test());
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

        check_snapshot("outline/simple", &db.finish());
    }
}
