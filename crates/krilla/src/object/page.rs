//! Working with pages of a PDF document.

use crate::content::ContentBuilder;
use crate::document::PageSettings;
use crate::error::KrillaResult;
use crate::object::annotation::Annotation;
use crate::resource::ResourceDictionary;
use crate::serialize::{FilterStream, SerializerContext};
use crate::stream::Stream;
use crate::surface::Surface;
use crate::tagging::{Identifier, PageTagIdentifier};
use crate::util::{Deferred, RectExt};
use crate::version::PdfVersion;
use pdf_writer::types::{NumberingStyle, TabOrder};
use pdf_writer::writers::NumberTree;
use pdf_writer::{Chunk, Finish, Ref, TextStr};
use std::num::NonZeroU32;
use std::ops::DerefMut;
use tiny_skia_path::{Rect, Transform};

/// A single page.
///
/// You cannot create an instance of this type yourself. Instead, you should use the
/// [`Document::start_page`] (or a related method) to add a new page to a document. In most cases, all
/// you need to do is to call the [`Page::surface`] method, so you can start drawing on the page.
/// However, there are a few other operations you can perform, such as adding annotations
/// to a page.
///
/// [`Document::start_page`]: crate::Document::start_page
pub struct Page<'a> {
    sc: &'a mut SerializerContext,
    page_settings: PageSettings,
    page_index: usize,
    page_stream: Stream,
    num_mcids: i32,
    annotations: Vec<Annotation>,
}

impl<'a> Page<'a> {
    pub(crate) fn new(
        sc: &'a mut SerializerContext,
        page_index: usize,
        page_settings: PageSettings,
    ) -> Self {
        Self {
            sc,
            page_settings,
            page_index,
            num_mcids: 0,
            page_stream: Stream::empty(),
            annotations: vec![],
        }
    }

    pub(crate) fn root_transform(&self) -> Transform {
        page_root_transform(self.page_settings.surface_size().height())
    }

    /// Add an annotation to the page.
    pub fn add_annotation(&mut self, annotation: Annotation) {
        self.annotations.push(annotation);
    }

    /// Add a tagged annotation to the page.
    pub fn add_tagged_annotation(&mut self, mut annotation: Annotation) -> Identifier {
        let annot_index = self.annotations.len();
        let struct_parent = self.sc.get_annotation_parent(self.page_index, annot_index);
        annotation.struct_parent = struct_parent;
        self.add_annotation(annotation);

        match struct_parent {
            None => Identifier::dummy(),
            Some(_) => Identifier::new_annotation(self.page_index, annot_index),
        }
    }

    /// Get the surface of the page to draw on. Calling this multiple times
    /// on the same page will reset any previous drawings.
    pub fn surface(&mut self) -> Surface {
        let root_builder = ContentBuilder::new(self.root_transform());

        let finish_fn = Box::new(|stream, num_mcids| {
            self.page_stream = stream;
            self.num_mcids = num_mcids;
        });

        let page_identifier = if self.sc.serialize_settings.enable_tagging {
            Some(PageTagIdentifier::new(self.page_index, 0))
        } else {
            None
        };

        Surface::new(self.sc, root_builder, page_identifier, finish_fn)
    }

    /// A shorthand for `std::mem::drop`.
    pub fn finish(self) {}
}

pub(crate) fn page_root_transform(height: f32) -> Transform {
    Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, height)
}

impl Drop for Page<'_> {
    fn drop(&mut self) {
        let annotations = std::mem::take(&mut self.annotations);
        let page_settings = std::mem::take(&mut self.page_settings);

        let struct_parent = self
            .sc
            .get_page_struct_parent(self.page_index, self.num_mcids);

        let stream = std::mem::replace(&mut self.page_stream, Stream::empty());
        let page = InternalPage::new(
            stream,
            self.sc,
            annotations,
            struct_parent,
            page_settings,
            self.page_index,
        );
        self.sc.add_page(page);
    }
}

pub(crate) struct InternalPage {
    pub stream_ref: Ref,
    pub stream_resources: ResourceDictionary,
    pub stream_chunk: Deferred<Chunk>,
    pub page_settings: PageSettings,
    pub page_index: usize,
    pub struct_parent: Option<i32>,
    pub bbox: Rect,
    pub annotations: Vec<Annotation>,
}

impl InternalPage {
    pub(crate) fn new(
        mut stream: Stream,
        sc: &mut SerializerContext,
        annotations: Vec<Annotation>,
        struct_parent: Option<i32>,
        page_settings: PageSettings,
        page_index: usize,
    ) -> Self {
        for validation_error in stream.validation_errors {
            sc.register_validation_error(validation_error)
        }

        let stream_ref = sc.new_ref();
        let serialize_settings = sc.serialize_settings.clone();
        let stream_resources = std::mem::take(&mut stream.resource_dictionary);

        let stream_chunk = Deferred::new(move || {
            let mut chunk = Chunk::new();
            let page_stream =
                FilterStream::new_from_content_stream(&stream.content, &serialize_settings);

            let mut stream = chunk.stream(stream_ref, page_stream.encoded_data());
            page_stream.write_filters(stream.deref_mut());

            stream.finish();
            chunk
        });

        Self {
            stream_resources,
            stream_ref,
            stream_chunk,
            struct_parent,
            bbox: stream.bbox.0,
            annotations,
            page_settings,
            page_index,
        }
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        root_ref: Ref,
    ) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let mut annotation_refs = vec![];

        if !self.annotations.is_empty() {
            for annotation in &self.annotations {
                let annot_ref = sc.new_ref();

                let a = annotation.serialize(
                    sc,
                    annot_ref,
                    self.page_settings.surface_size().height(),
                )?;
                chunk.extend(&a);
                annotation_refs.push(annot_ref);
            }
        }

        let mut page = chunk.page(root_ref);
        self.stream_resources.to_pdf_resources(&mut page);

        let media_box = self
            .page_settings
            .media_box()
            .unwrap_or(self.bbox)
            .transform(page_root_transform(
                self.page_settings.surface_size().height(),
            ))
            .unwrap();
        // Convert to the proper PDF values.
        page.media_box(media_box.to_pdf_rect());

        if let Some(struct_parent) = self.struct_parent {
            page.struct_parents(struct_parent);

            // Only required for PDF/UA, but might as well always set it.
            if !self.annotations.is_empty()
                && sc.serialize_settings.pdf_version >= PdfVersion::Pdf15
            {
                page.tab_order(TabOrder::StructureOrder);
            }
        }

        page.parent(sc.page_tree_ref());
        page.contents(self.stream_ref);

        if !annotation_refs.is_empty() {
            page.annotations(annotation_refs.iter().copied());
        }

        // Populate the refs for each annotation in page infos.
        let page_info = &mut sc.page_infos_mut()[self.page_index];
        page_info.annotations = annotation_refs;

        page.finish();

        chunk.extend(self.stream_chunk.wait());

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
    /// Create a new page label.
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

    pub(crate) fn serialize(&self, root_ref: Ref) -> Chunk {
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
pub(crate) struct PageLabelContainer<'a> {
    labels: &'a [PageLabel],
}

impl<'a> PageLabelContainer<'a> {
    pub(crate) fn new(labels: &'a [PageLabel]) -> Option<Self> {
        if labels.iter().all(|f| f.is_empty()) {
            None
        } else {
            Some(PageLabelContainer { labels })
        }
    }

    pub(crate) fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
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

        chunk
    }
}

#[cfg(test)]
mod tests {

    use crate::document::{Document, PageSettings};
    use crate::object::page::{InternalPage, PageLabel};
    use crate::serialize::SerializerContext;
    use crate::stream::StreamBuilder;

    use crate::path::Fill;
    use crate::tests::{blue_fill, green_fill, purple_fill, rect_to_path, red_fill};
    use krilla_macros::{snapshot, visreg};
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

        let page_settings = PageSettings::new(200.0, 200.0);

        surface.fill_path(&path, Fill::default());
        surface.finish();
        let page = InternalPage::new(stream_builder.finish(), sc, vec![], None, page_settings, 0);
        sc.add_page(page);
    }

    #[snapshot(settings_2)]
    fn page_with_resources(sc: &mut SerializerContext) {
        let mut stream_builder = StreamBuilder::new(sc);
        let mut surface = stream_builder.surface();

        let mut builder = PathBuilder::new();
        builder.push_rect(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap());
        let path = builder.finish().unwrap();

        let page_settings = PageSettings::new(200.0, 200.0);

        surface.fill_path(&path, Fill::default());
        surface.finish();
        let page = InternalPage::new(stream_builder.finish(), sc, vec![], None, page_settings, 0);
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

    #[snapshot(document)]
    fn page_label_complex(d: &mut Document) {
        d.start_page_with(PageSettings::new(200.0, 200.0));
        d.start_page_with(PageSettings::new(250.0, 200.0));

        let settings = PageSettings::new(250.0, 200.0).with_page_label(PageLabel::new(
            Some(NumberingStyle::LowerRoman),
            None,
            NonZeroU32::new(2).unwrap(),
        ));

        d.start_page_with(settings);
    }

    fn media_box_impl(d: &mut Document, media_box: Rect) {
        let mut page =
            d.start_page_with(PageSettings::new(200.0, 200.0).with_media_box(Some(media_box)));
        let mut surface = page.surface();
        surface.fill_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), red_fill(0.5));
        surface.fill_path(&rect_to_path(100.0, 0.0, 200.0, 100.0), green_fill(0.5));
        surface.fill_path(&rect_to_path(0.0, 100.0, 100.0, 200.0), blue_fill(0.5));
        surface.fill_path(&rect_to_path(100.0, 100.0, 200.0, 200.0), purple_fill(0.5));
    }

    #[visreg(document)]
    fn custom_media_box_top_left(d: &mut Document) {
        media_box_impl(d, Rect::from_xywh(-100.0, -100.0, 200.0, 200.0).unwrap())
    }

    #[visreg(document)]
    fn custom_media_box_top_right(d: &mut Document) {
        media_box_impl(d, Rect::from_xywh(100.0, -100.0, 200.0, 200.0).unwrap())
    }

    #[visreg(document)]
    fn custom_media_box_bottom_left(d: &mut Document) {
        media_box_impl(d, Rect::from_xywh(-100.0, 100.0, 200.0, 200.0).unwrap())
    }

    #[visreg(document)]
    fn custom_media_box_bottom_right(d: &mut Document) {
        media_box_impl(d, Rect::from_xywh(100.0, 100.0, 200.0, 200.0).unwrap())
    }

    #[visreg(document)]
    fn custom_media_box_zoomed_out(d: &mut Document) {
        media_box_impl(d, Rect::from_xywh(-150.0, -200.0, 500.0, 500.0).unwrap())
    }
}
