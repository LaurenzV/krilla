//! Writing the objects of a document into a sink that the caller supplies.
//!
//! [`Document::finish`](crate::Document::finish) writes a complete PDF file.
//! [`Document::finish_into`](crate::Document::finish_into) instead hands the
//! same indirect objects to an [`ObjectSink`], numbered with object numbers
//! that the sink allocates, so that an application can add them to a
//! document it already holds.

/// Receives the indirect objects of a document.
pub trait ObjectSink {
    /// Allocates the number of a new indirect object in the receiving
    /// document. Numbers must be positive and unique.
    fn allocate(&mut self) -> i32;

    /// Receives a sequence of complete indirect objects (`N 0 obj ...
    /// endobj`), each numbered with a number from [`ObjectSink::allocate`].
    fn write(&mut self, objects: &[u8]);
}

/// The objects that a document catalog would reference, as numbers the sink
/// allocated. The sink owns the catalog, the document information, and the
/// XMP metadata, which krilla does not write into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkOutput {
    /// The root of the page tree.
    pub pages: i32,
    /// The interactive form dictionary.
    pub acro_form: Option<i32>,
    /// The page label number tree.
    pub page_labels: Option<i32>,
    /// The output intents array.
    pub output_intents: Option<i32>,
    /// The structure tree root.
    pub struct_tree_root: Option<i32>,
    /// The outline dictionary.
    pub outlines: Option<i32>,
    /// The named destinations, sorted by name, for the `Dests` name tree.
    pub named_destinations: Vec<(Vec<u8>, i32)>,
    /// The embedded files, sorted by name, for the `EmbeddedFiles` name tree
    /// and the catalog's associated files.
    pub embedded_files: Vec<(Vec<u8>, i32)>,
}

#[cfg(test)]
mod tests {
    use super::ObjectSink;
    use crate::geom::{PathBuilder, Rect};
    use crate::page::PageSettings;
    use crate::Document;

    struct Collector {
        next: i32,
        allocated: Vec<i32>,
        bytes: Vec<u8>,
    }

    impl ObjectSink for Collector {
        fn allocate(&mut self) -> i32 {
            let number = self.next;
            self.next += 1;
            self.allocated.push(number);
            number
        }

        fn write(&mut self, objects: &[u8]) {
            self.bytes.extend_from_slice(objects);
        }
    }

    fn document() -> Document {
        let mut document = Document::new();
        let mut page = document.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());
        let mut surface = page.surface();
        let mut builder = PathBuilder::new();
        builder.push_rect(Rect::from_xywh(20.0, 20.0, 50.0, 50.0).unwrap());
        surface.draw_path(&builder.finish().unwrap());
        surface.finish();
        page.finish();
        document
    }

    fn count(bytes: &[u8], needle: &[u8]) -> usize {
        bytes.windows(needle.len()).filter(|w| *w == needle).count()
    }

    #[test]
    fn sink_receives_every_object_with_its_own_numbers() {
        let mut sink = Collector {
            next: 100,
            allocated: vec![],
            bytes: vec![],
        };
        let output = document().finish_into(&mut sink).unwrap();

        assert!(sink.allocated.contains(&output.pages));
        assert_eq!(count(&sink.bytes, b" 0 obj"), sink.allocated.len());
        for number in &sink.allocated {
            let header = format!("{number} 0 obj");
            assert_eq!(count(&sink.bytes, header.as_bytes()), 1, "{header}");
        }
        assert!(!sink.bytes.windows(4).any(|w| w == b"/Cat"));
    }

    #[test]
    fn the_file_holds_the_sink_objects_and_a_catalog() {
        let mut sink = Collector {
            next: 1,
            allocated: vec![],
            bytes: vec![],
        };
        document().finish_into(&mut sink).unwrap();
        let pdf = document().finish().unwrap();

        // The file adds the catalog and the document information.
        assert_eq!(count(&pdf, b" 0 obj"), sink.allocated.len() + 2);
    }
}
