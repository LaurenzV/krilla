use crate::serialize::{Object, RegisterableObject, SerializerContext};
use crate::stream::Stream;
use crate::util::RectExt;
use pdf_writer::types::NumberingStyle;
use pdf_writer::{Chunk, Finish, Ref, TextStr};
use std::num::{NonZeroI32, NonZeroU32};
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

// TODO: Make sure that page 0 is always included.

/// A page label.
#[derive(Debug, Hash, Eq, PartialEq)]
pub(crate) struct PageLabel {
    /// The numbering style of the page label.
    style: NumberingStyle,
    /// The prefix of the page label.
    prefix: Option<String>,
    /// The start numeric value of the page label.
    offset: NonZeroI32,
}

impl PageLabel {
    pub fn new(style: NumberingStyle, prefix: Option<String>, offset: NonZeroI32) -> Self {
        Self {
            style,
            prefix,
            offset,
        }
    }
}

impl Object for PageLabel {
    fn serialize_into(self, _: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();
        let mut label = chunk
            .indirect(root_ref)
            .start::<pdf_writer::writers::PageLabel>();
        label.style(self.style);

        if let Some(prefix) = &self.prefix {
            label.prefix(TextStr(prefix));
        }

        label.offset(self.offset.get());

        label.finish();

        chunk
    }
}

impl RegisterableObject for PageLabel {}

#[cfg(test)]
mod tests {
    use crate::object::page::{Page, PageLabel};
    use crate::rgb::Rgb;
    use crate::serialize::{SerializeSettings, SerializerContext};
    use crate::surface::StreamBuilder;
    use crate::test_utils::check_snapshot;
    use crate::Fill;
    use pdf_writer::types::NumberingStyle;
    use std::num::NonZeroI32;
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

    #[test]
    fn page_label() {
        let mut sc = SerializerContext::new(SerializeSettings::default_test());

        let page_label = PageLabel::new(
            NumberingStyle::Arabic,
            Some("P".to_string()),
            NonZeroI32::new(2).unwrap(),
        );

        sc.add(page_label);

        check_snapshot("page/page_label", sc.finish().as_bytes());
    }
}
