//! Building outlines for the navigation of the document.
//!
//! An outline is a tree-like structure that stores the hierarchical structure of a document.
//! In particular, in most cases it is used to define the hierarchy of headings in the document.
//! For example, your document might consist of two first-level headings, which in turn have
//! more nested headings, which in turn might be nested even further, and so on. The [`Outline`]
//! allows you to encode this behavior, and makes it easier to navigate the document in PDF
//! viewers that support this feature.
//!
//! The intended usage is to create a new, empty outline in the very beginning. It represents
//! the root of the tree. Then, you can create new outline nodes, which you can recursively
//! nest in any way you wish while processing your document. In the end, you need to push
//! the first layer of children to the [`Outline`] object.
//!
//! Finally, once you are done building your outline tree, you can use the [`Document::set_outline`]
//! function of [`Document`] to store the outline in the document.
//!
//! [`Document`]: crate::Document
//! [`Document::set_outline`]: crate::Document::set_outline

use pdf_writer::writers::OutlineItem;
use pdf_writer::{Chunk, Finish, Name, Ref, TextStr};

use crate::error::KrillaResult;
use crate::object::destination::XyzDestination;
use crate::serialize::SerializeContext;

/// An outline.
///
/// This represents the root of the outline tree.
#[derive(Debug, Clone)]
pub struct Outline {
    children: Vec<OutlineNode>,
}

impl Outline {
    pub(crate) fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl Default for Outline {
    fn default() -> Self {
        Self::new()
    }
}

trait Outlineable {
    fn first(&mut self, item: Ref) -> &mut Self;
    fn last(&mut self, item: Ref) -> &mut Self;
    fn count(&mut self, count: i32) -> &mut Self;
}

impl Outlineable for pdf_writer::writers::Outline<'_> {
    fn first(&mut self, item: Ref) -> &mut Self {
        self.first(item)
    }

    fn last(&mut self, item: Ref) -> &mut Self {
        self.last(item)
    }

    fn count(&mut self, count: i32) -> &mut Self {
        self.count(count)
    }
}

impl Outlineable for OutlineItem<'_> {
    fn first(&mut self, item: Ref) -> &mut Self {
        self.first(item)
    }

    fn last(&mut self, item: Ref) -> &mut Self {
        self.last(item)
    }

    fn count(&mut self, count: i32) -> &mut Self {
        self.count(count)
    }
}

impl Outline {
    /// Create a new, empty outline.
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    /// Push a new child (which may in turn contain other children) to the outline.
    pub fn push_child(&mut self, node: OutlineNode) {
        self.children.push(node)
    }

    pub(crate) fn serialize(&self, sc: &mut SerializeContext, root: Ref) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let mut sub_chunks = vec![];

        let mut outline = chunk.outline(root);
        serialize_children(
            &self.children,
            root,
            &mut sub_chunks,
            sc,
            &mut outline,
            false,
        )?;
        outline.finish();

        for sub_chunk in sub_chunks {
            chunk.extend(&sub_chunk);
        }

        Ok(chunk)
    }
}

/// An outline node.
///
/// This represents either an intermediate node in the outline tree, or a leaf node
/// if it does not contain any further children itself.
#[derive(Debug, Clone)]
pub struct OutlineNode {
    /// The children of the outline node.
    children: Vec<OutlineNode>,
    /// The text of the outline entry.
    text: String,
    /// The destination of the outline entry.
    destination: XyzDestination,
}

impl OutlineNode {
    /// Create a new outline node.
    ///
    /// `text` is the string that should be displayed in the outline tree, and
    /// `destination` is the destination that should be jumped to when clicking on
    /// the outline entry.
    pub fn new(text: String, destination: XyzDestination) -> Self {
        Self {
            children: vec![],
            text,
            destination,
        }
    }

    /// Add a new child to the outline node.
    pub fn push_child(&mut self, node: OutlineNode) {
        self.children.push(node)
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializeContext,
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

        serialize_children(
            &self.children,
            root,
            &mut sub_chunks,
            sc,
            &mut outline_entry,
            true,
        )?;

        if !self.text.is_empty() {
            outline_entry.title(TextStr(&self.text));
        }

        let dest_ref = sc.register_xyz_destination(self.destination.clone());
        outline_entry.pair(Name(b"Dest"), dest_ref);

        outline_entry.finish();

        for sub_chunk in sub_chunks {
            chunk.extend(&sub_chunk);
        }

        Ok(chunk)
    }
}

fn serialize_children(
    children: &[OutlineNode],
    root: Ref,
    sub_chunks: &mut Vec<Chunk>,
    sc: &mut SerializeContext,
    outlineable: &mut impl Outlineable,
    negate_count: bool,
) -> KrillaResult<()> {
    if !children.is_empty() {
        let first = sc.new_ref();
        let mut last = first;

        let mut prev = None;
        let mut cur = Some(first);

        for i in 0..children.len() {
            let next = if i < children.len() - 1 {
                Some(sc.new_ref())
            } else {
                None
            };

            last = cur.unwrap();

            sub_chunks.push(children[i].serialize(sc, root, last, next, prev)?);

            prev = cur;
            cur = next;
        }

        outlineable.first(first);
        outlineable.last(last);

        let mut count = i32::try_from(children.len()).unwrap();
        if negate_count {
            count = -count;
        }
        outlineable.count(count);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use krilla_macros::snapshot;
    use tiny_skia_path::Point;

    use crate::destination::XyzDestination;
    use crate::document::{Document, PageSettings};
    use crate::object::outline::{Outline, OutlineNode};
    use crate::tests::{blue_fill, green_fill, rect_to_path, red_fill};

    #[snapshot(document)]
    fn outline_simple(d: &mut Document) {
        let fills = [red_fill(1.0), green_fill(1.0), blue_fill(1.0)];
        for (index, fill) in fills.into_iter().enumerate() {
            let factor = index as f32 * 50.0;
            let path = rect_to_path(factor, factor, 100.0 + factor, 100.0 + factor);
            let mut page = d.start_page_with(PageSettings::new(200.0, 200.0));
            let mut surface = page.surface();
            surface.fill_path(&path, fill);
            surface.finish();
            page.finish();
        }
        let mut outline = Outline::new();

        let mut child1 = OutlineNode::new(
            "Heading 1".to_string(),
            XyzDestination::new(0, Point::from_xy(0.0, 0.0)),
        );
        child1.push_child(OutlineNode::new(
            "Heading 1.1".to_string(),
            XyzDestination::new(1, Point::from_xy(50.0, 50.0)),
        ));

        let child2 = OutlineNode::new(
            "Heading 2".to_string(),
            XyzDestination::new(2, Point::from_xy(100.0, 100.0)),
        );

        outline.push_child(child1);
        outline.push_child(child2);

        d.set_outline(outline);
    }
}
