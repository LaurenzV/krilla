//! Working with pages of a PDF document.

use crate::document::PageSettings;
use crate::error::KrillaResult;
use crate::object::annotation::Annotation;
use crate::serialize::{FilterStream, SerializerContext};
use crate::stream::{ContentBuilder, Stream};
use crate::surface::Surface;
use crate::util::RectExt;
use pdf_writer::types::NumberingStyle;
use pdf_writer::writers::NumberTree;
use pdf_writer::{Chunk, Finish, Ref, TextStr};
use std::num::NonZeroU32;
use std::ops::DerefMut;
use tiny_skia_path::Transform;

/// A single page.
///
/// You cannot create an instance of this type yourself. Instead, you should use the
/// `add_page` (or a related method) to add a new page to a document. In most cases, all
/// you need to do is to call the `surface` method so you can start drawing on the page.
/// However, there are a few other operations you can perform, such as adding annotations
/// to a page.
pub struct Page<'a> {
    sc: &'a mut SerializerContext,
    page_settings: PageSettings,
    page_stream: Stream,
    annotations: Vec<Annotation>,
}

impl<'a> Page<'a> {
    pub(crate) fn new(sc: &'a mut SerializerContext, page_settings: PageSettings) -> Self {
        Self {
            sc,
            page_settings,
            page_stream: Stream::empty(),
            annotations: vec![],
        }
    }

    pub(crate) fn root_transform(&self) -> Transform {
        Transform::from_row(
            1.0,
            0.0,
            0.0,
            -1.0,
            0.0,
            self.page_settings.media_box.height(),
        )
    }

    /// Add an annotation to the page.
    pub fn add_annotation(&mut self, annotation: Annotation) {
        self.annotations.push(annotation);
    }

    /// Get the surface of the page to draw on. Calling this multiple times
    /// on the same page will reset any previous drawings.
    pub fn surface(&mut self) -> Surface {
        let mut root_builder = ContentBuilder::new();
        // Invert the y-axis.
        root_builder.concat_transform(&self.root_transform());

        let finish_fn = Box::new(|stream| self.page_stream = stream);

        Surface::new(&mut self.sc, root_builder, finish_fn)
    }

    /// A shorthand for `std::mem::drop`.
    pub fn finish(self) {}
}

impl Drop for Page<'_> {
    fn drop(&mut self) {
        let annotations = std::mem::take(&mut self.annotations);
        let page_settings = std::mem::take(&mut self.page_settings);

        let stream = std::mem::replace(&mut self.page_stream, Stream::empty());
        let page = InternalPage::new(stream, annotations, page_settings);
        self.sc.add_page(page);
    }
}

pub(crate) struct InternalPage {
    pub stream: Stream,
    pub page_settings: PageSettings,
    pub annotations: Vec<Annotation>,
}

impl InternalPage {
    pub(crate) fn new(
        stream: Stream,
        annotations: Vec<Annotation>,
        page_settings: PageSettings,
    ) -> Self {
        Self {
            stream,
            annotations,
            page_settings,
        }
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        root_ref: Ref,
    ) -> KrillaResult<Chunk> {
        let stream_ref = sc.new_ref();

        let mut chunk = Chunk::new();

        let mut annotation_refs = vec![];

        if !self.annotations.is_empty() {
            for annotation in &self.annotations {
                let annot_ref = sc.new_ref();
                chunk.extend(&annotation.serialize(
                    sc,
                    annot_ref,
                    self.page_settings.media_box.height(),
                )?);
                annotation_refs.push(annot_ref);
            }
        }

        let mut page = chunk.page(root_ref);
        self.stream
            .resource_dictionary()
            .to_pdf_resources(sc, &mut page)?;

        page.media_box(self.page_settings.media_box.to_pdf_rect());
        page.parent(sc.page_tree_ref());
        page.contents(stream_ref);

        if !annotation_refs.is_empty() {
            page.annotations(annotation_refs);
        }

        page.finish();

        let page_stream =
            FilterStream::new_from_content_stream(self.stream.content(), &sc.serialize_settings);

        let mut stream = chunk.stream(stream_ref, &page_stream.encoded_data());
        page_stream.write_filters(stream.deref_mut());

        stream.finish();

        Ok(chunk)
    }
}

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
    pub fn new(style: Option<NumberingStyle>, prefix: Option<String>, offset: NonZeroU32) -> Self {
        Self {
            style,
            prefix,
            offset: Some(offset),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.style.is_none() && self.prefix.is_none() && self.offset.is_none()
    }

    pub fn serialize(&self, root_ref: Ref) -> Chunk {
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

#[derive(Hash)]
pub struct PageLabelContainer<'a> {
    labels: &'a [PageLabel],
}

impl<'a> PageLabelContainer<'a> {
    pub fn new(labels: &'a [PageLabel]) -> Option<Self> {
        return if labels.iter().all(|f| f.is_empty()) {
            None
        } else {
            Some(PageLabelContainer { labels })
        };
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        root_ref: Ref,
    ) -> KrillaResult<Chunk> {
        // Will always contain at least one entry, since we ensured that a PageLabelContainer cannot
        // be empty
        let mut filtered_entries = vec![];
        let mut prev: Option<PageLabel> = None;

        for (i, label) in self.labels.iter().enumerate() {
            if let Some(n_prev) = &prev {
                if n_prev.style != label.style
                    || n_prev.prefix != label.prefix
                    || n_prev.offset.map(|n| n.get()) != label.offset.map(|n| n.get() + 1)
                {
                    filtered_entries.push((i, label.clone()));
                    prev = Some(label.clone());
                }
            } else {
                filtered_entries.push((i, label.clone()));
                prev = Some(label.clone());
            }
        }

        let mut chunk = Chunk::new();
        let mut num_tree = chunk.indirect(root_ref).start::<NumberTree<Ref>>();
        let mut nums = num_tree.nums();

        for (page_num, label) in filtered_entries {
            let label_ref = sc.add_page_label(label);
            nums.insert(page_num as i32, label_ref);
        }

        nums.finish();
        num_tree.finish();

        Ok(chunk)
    }
}

#[cfg(test)]
mod tests {
    use crate::color::rgb::Rgb;
    use crate::document::PageSettings;
    use crate::object::page::{InternalPage, PageLabel};
    use crate::serialize::SerializerContext;
    use crate::surface::StreamBuilder;

    use crate::Fill;
    use krilla_macros::snapshot;
    use pdf_writer::types::NumberingStyle;
    use std::num::NonZeroU32;
    use tiny_skia_path::{PathBuilder, Rect};

    #[snapshot]
    fn page_simple(sc: &mut SerializerContext) {
        let mut stream_builder = StreamBuilder::new(sc);
        let mut surface = stream_builder.surface();

        let mut builder = PathBuilder::new();
        builder.push_rect(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap());
        let path = builder.finish().unwrap();

        let page_settings = PageSettings::with_size(200.0, 200.0);

        surface.fill_path(&path, Fill::<Rgb>::default());
        surface.finish();
        let page = InternalPage::new(stream_builder.finish(), vec![], page_settings);
        sc.add_page(page);
    }

    #[snapshot(settings_2)]
    fn page_with_resources(sc: &mut SerializerContext) {
        let mut stream_builder = StreamBuilder::new(sc);
        let mut surface = stream_builder.surface();

        let mut builder = PathBuilder::new();
        builder.push_rect(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap());
        let path = builder.finish().unwrap();

        let page_settings = PageSettings::with_size(200.0, 200.0);

        surface.fill_path(&path, Fill::<Rgb>::default());
        surface.finish();
        let page = InternalPage::new(stream_builder.finish(), vec![], page_settings);
        sc.add_page(page);
    }

    #[snapshot]
    fn page_label(sc: &mut SerializerContext) {
        let page_label = PageLabel::new(
            Some(NumberingStyle::Arabic),
            Some("P".to_string()),
            NonZeroU32::new(2).unwrap(),
        );

        sc.add_page_label(page_label);
    }

    // #[snapshot(document)]
    // fn page_label_complex(db: &mut Document) {
    //     db.start_page_with(Size::from_wh(200.0, 200.0).unwrap(), PageLabel::default());
    //     db.start_page_with(Size::from_wh(250.0, 200.0).unwrap(), PageLabel::default());
    //     db.start_page_with(
    //         Size::from_wh(200.0, 200.0).unwrap(),
    //         PageLabel::new(
    //             Some(NumberingStyle::LowerRoman),
    //             None,
    //             NonZeroU32::new(2).unwrap(),
    //         ),
    //     );
    // }
}
