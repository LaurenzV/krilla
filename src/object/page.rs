use crate::serialize::{Object, RegisterableObject, SerializerContext};
use crate::stream::Stream;
use crate::util::RectExt;
use pdf_writer::types::NumberingStyle;
use pdf_writer::writers::NumberTree;
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
    /// The label of the page.
    pub page_label: PageLabel,
}

impl Page {
    /// Create a new page.
    pub fn new(size: Size, stream: Stream, page_label: PageLabel) -> Self {
        Self {
            stream,
            media_box: size.to_rect(0.0, 0.0).unwrap(),
            page_label,
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

        sc.add_page_info(root_ref, self.page_label);

        chunk
    }
}

impl RegisterableObject for Page {}

/// A page label.
#[derive(Debug, Hash, Eq, PartialEq, Default, Clone)]
pub struct PageLabel {
    /// The numbering style of the page label.
    pub style: Option<NumberingStyle>,
    /// The prefix of the page label.
    pub prefix: Option<String>,
    /// The numeric value of the page label.
    pub offset: Option<NonZeroU32>,
}

impl PageLabel {
    pub fn new(
        style: Option<NumberingStyle>,
        prefix: Option<String>,
        offset: NonZeroU32,
    ) -> Self {
        Self {
            style,
            prefix,
            offset: Some(offset),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.style.is_none() && self.prefix.is_none() && self.offset.is_none()
    }
}

impl Object for PageLabel {
    fn serialize_into(self, _: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();
        let mut label = chunk
            .indirect(root_ref)
            .start::<pdf_writer::writers::PageLabel>();
        if let Some(style) = self.style {
            label.style(style);
        }

        if let Some(prefix) = &self.prefix {
            label.prefix(TextStr(prefix));
        }

        if let Some(offset) = self.offset {
            label.offset(i32::try_from(offset.get()).unwrap());
        }

        label.finish();

        chunk
    }
}

impl RegisterableObject for PageLabel {}

#[derive(Hash)]
pub struct PageLabelContainer {
    labels: Vec<PageLabel>,
}

impl PageLabelContainer {
    pub fn new(labels: Vec<PageLabel>) -> Option<Self> {
        return if labels.iter().all(|f| f.is_empty()) {
            None
        } else {
            Some(PageLabelContainer { labels })
        };
    }
}

impl Object for PageLabelContainer {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        // Will always contain at least one entry, since we ensured that a PageLabelContainer cannot
        // be empty
        let mut filtered_entries = vec![];
        let mut prev: Option<PageLabel> = None;

        for (i, label) in self.labels.into_iter().enumerate() {
            if let Some(n_prev) = &prev {
                if n_prev.style != label.style
                    || n_prev.prefix != label.prefix
                    || n_prev.offset.map(|n| n.get()) != label.offset.map(|n| n.get() + 1)
                {
                    filtered_entries.push((i, label.clone()));
                    prev = Some(label);
                }
            } else {
                filtered_entries.push((i, label.clone()));
                prev = Some(label);
            }
        }

        let mut chunk = Chunk::new();
        let mut num_tree = chunk.indirect(root_ref).start::<NumberTree<Ref>>();
        let mut nums = num_tree.nums();

        for (page_num, label) in filtered_entries {
            let label_ref = sc.add(label);
            nums.insert(page_num as i32, label_ref);
        }

        nums.finish();
        num_tree.finish();

        chunk
    }
}

impl RegisterableObject for PageLabelContainer {}

#[cfg(test)]
mod tests {
    use crate::object::page::{Page, PageLabel};
    use crate::rgb::Rgb;
    use crate::serialize::{SerializeSettings, SerializerContext};
    use crate::surface::StreamBuilder;
    use crate::test_utils::check_snapshot;
    use crate::Fill;
    use pdf_writer::types::NumberingStyle;
    use std::num::{NonZeroI32, NonZeroU32};
    use tiny_skia_path::{PathBuilder, Rect, Size};
    use crate::document::Document;

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
            PageLabel::default(),
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
            PageLabel::default(),
        );
        sc.add(page);

        check_snapshot("page/page_with_resources", sc.finish().as_bytes());
    }

    #[test]
    fn page_label() {
        let mut sc = SerializerContext::new(SerializeSettings::default_test());

        let page_label = PageLabel::new(
            Some(NumberingStyle::Arabic),
            Some("P".to_string()),
            NonZeroU32::new(2).unwrap(),
        );

        sc.add(page_label);

        check_snapshot("page/page_label", sc.finish().as_bytes());
    }

    // TODO: Fix issues with not being able to create empty pages with just start_page_with.
    // TODO: Fix issue with two duplicate pages not showing up.

    #[test]
    fn page_label_complex() {
        let mut db = Document::new(SerializeSettings::default_test());
        let mut page = db.start_page_with(Size::from_wh(200.0, 200.0).unwrap(), PageLabel::default());
        let mut surface = page.surface();
        surface.finish();
        page.finish();

        let mut page = db.start_page_with(Size::from_wh(250.0, 200.0).unwrap(), PageLabel::default());
        let mut surface = page.surface();
        surface.finish();
        page.finish();

        let mut page = db.start_page_with(Size::from_wh(200.0, 200.0).unwrap(), PageLabel::new(
            Some(NumberingStyle::LowerRoman),
            None,
            NonZeroU32::new(2).unwrap()
        ));
        let mut surface = page.surface();
        surface.finish();
        page.finish();


        check_snapshot("page/page_label_complex", &db.finish());
    }
}
