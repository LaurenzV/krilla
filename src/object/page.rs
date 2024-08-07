use crate::serialize::{Object, RegisterableObject, SerializerContext};
use crate::stream::Stream;
use crate::util::RectExt;
use pdf_writer::{Chunk, Finish, Ref};
use tiny_skia_path::{Rect, Size};

#[derive(Debug, Hash, Eq, PartialEq)]
pub(crate) struct Page {
    pub stream: Stream,
    pub media_box: Rect,
}

impl Page {
    pub fn new(size: Size, stream: Stream) -> Self {
        Self {
            stream,
            media_box: size.to_rect(0.0, 0.0).unwrap(),
        }
    }
}

impl Object for Page {
    fn serialize_into(self, sc: &mut SerializerContext) -> (Ref, Chunk) {
        let root_ref = sc.new_ref();
        let stream_ref = sc.new_ref();

        let mut chunk = Chunk::new();

        let mut page = chunk.page(root_ref);
        self.stream
            .resource_dictionary
            .to_pdf_resources(sc, &mut page.resources());

        page.media_box(self.media_box.to_pdf_rect());
        page.parent(sc.page_tree_ref());
        page.contents(stream_ref);
        page.finish();

        chunk.stream(stream_ref, &self.stream.content);

        sc.add_page_ref(root_ref);

        (root_ref, chunk)
    }
}

impl RegisterableObject for Page {}
