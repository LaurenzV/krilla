use crate::serialize::{Object, RegisterableObject, SerializerContext};
use crate::stream::Stream;
use crate::util::RectExt;
use pdf_writer::{Chunk, Finish, Ref};
use tiny_skia_path::{Rect, Size};

/// A page.
#[derive(Debug, Hash, Eq, PartialEq)]
pub(crate) struct Page {
    /// The stream of the page.
    pub stream: Stream,
    /// The media box of the page.
    pub media_box: Rect,
}

impl Page {
    /// Create a new page.
    pub fn new(size: Size, stream: Stream) -> Self {
        Self {
            stream,
            media_box: size.to_rect(0.0, 0.0).unwrap(),
        }
    }
}

impl Object for Page {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let stream_ref = sc.new_ref();

        let mut chunk = Chunk::new();

        let mut page = chunk.page(root_ref);
        self.stream
            .resource_dictionary
            .to_pdf_resources(sc, &mut page);

        page.media_box(self.media_box.to_pdf_rect());
        page.parent(sc.page_tree_ref());
        page.contents(stream_ref);
        page.finish();

        let (stream, filter) = sc.get_content_stream(&self.stream.content);

        let mut stream = chunk.stream(stream_ref, &stream);

        if let Some(filter) = filter {
            stream.filter(filter);
        }

        stream.finish();

        sc.add_page_ref(root_ref);

        chunk
    }
}

impl RegisterableObject for Page {}

#[cfg(test)]
mod tests {
    use crate::object::page::Page;
    use crate::rgb::Rgb;
    use crate::serialize::{SerializeSettings, SerializerContext};
    use crate::surface::StreamBuilder;
    use crate::test_utils::check_snapshot;
    use crate::Fill;
    use tiny_skia_path::{PathBuilder, Rect, Size};

    #[test]
    fn simple_page() {
        let mut sc = SerializerContext::new(SerializeSettings::default_test());

        let mut stream_builder = StreamBuilder::new(&mut sc);
        let mut surface = stream_builder.surface();

        let mut builder = PathBuilder::new();
        builder.push_rect(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap());
        let path = builder.finish().unwrap();

        surface.fill_path(&path, Fill::<Rgb>::default());
        surface.finish();
        let page = Page::new(
            Size::from_wh(200.0, 200.0).unwrap(),
            stream_builder.finish(),
        );
        sc.add(page);

        check_snapshot("page/simple_page", sc.finish().as_bytes());
    }

    #[test]
    fn page_with_resources() {
        let mut sc = SerializerContext::new(SerializeSettings {
            no_device_cs: true,
            ..SerializeSettings::default_test()
        });

        let mut stream_builder = StreamBuilder::new(&mut sc);
        let mut surface = stream_builder.surface();

        let mut builder = PathBuilder::new();
        builder.push_rect(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap());
        let path = builder.finish().unwrap();

        surface.fill_path(&path, Fill::<Rgb>::default());
        surface.finish();
        let page = Page::new(
            Size::from_wh(200.0, 200.0).unwrap(),
            stream_builder.finish(),
        );
        sc.add(page);

        check_snapshot("page/page_with_resources", sc.finish().as_bytes());
    }
}
