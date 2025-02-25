//! Creating new PDF documents.
//!
//! When using krilla, the starting point is always the creation of a [`Document`]. A document
//! represents _one_ PDF document, to which you can add pages or configure them in any
//! other way you want.
//!
//! Unfortunately, creating PDFs always requires some kind of global state to keep track
//! of different aspects in the creation process, meaning that it is not possible to
//! generate multiple pages at the same time. Instead, you need to add pages separately
//! by calling the [`Document::start_page`] method, which returns a new [`Page`] object that mutably
//! borrows the global state from the document. Once the page is dropped, the global
//! state is passed back to the original document, which you can then use to add even
//! more pages.
//!
//! [`Page`]: crate::page::Page

use tiny_skia_path::{Rect, Size};

use crate::error::KrillaResult;
use crate::metadata::Metadata;
use crate::object::outline::Outline;
use crate::object::page::Page;
use crate::object::page::PageLabel;
use crate::serialize::{SerializeContext, SerializeSettings};
use crate::tagging::TagTree;

/// A PDF document.
pub struct Document {
    pub(crate) serializer_context: SerializeContext,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    /// Create a new document with default settings.
    pub fn new() -> Self {
        Self {
            serializer_context: SerializeContext::new(SerializeSettings::default()),
        }
    }

    /// Create a new document with specific serialization settings.
    pub fn new_with(serialize_settings: SerializeSettings) -> Self {
        Self {
            serializer_context: SerializeContext::new(serialize_settings),
        }
    }

    /// Start a new page with default settings.
    pub fn start_page(&mut self) -> Page {
        let page_index = self.serializer_context.page_infos().iter().len();
        Page::new(
            &mut self.serializer_context,
            page_index,
            PageSettings::default(),
        )
    }

    /// Start a new page with specific page settings.
    pub fn start_page_with(&mut self, page_settings: PageSettings) -> Page {
        let page_index = self.serializer_context.page_infos().iter().len();
        Page::new(&mut self.serializer_context, page_index, page_settings)
    }

    /// Set the outline of the document.
    pub fn set_outline(&mut self, outline: Outline) {
        self.serializer_context.set_outline(outline);
    }

    /// Set the metadata of the document.
    pub fn set_metadata(&mut self, metadata: Metadata) {
        self.serializer_context.set_metadata(metadata);
    }

    /// Set the tag tree of the document.
    pub fn set_tag_tree(&mut self, tag_tree: TagTree) {
        self.serializer_context.set_tag_tree(tag_tree);
    }

    /// Attempt to write the document to a PDF.
    pub fn finish(mut self) -> KrillaResult<Vec<u8>> {
        // Write empty page if none has been created yet.
        if self.serializer_context.page_infos().is_empty() {
            self.start_page();
        }

        Ok(self.serializer_context.finish()?.finish())
    }
}

#[derive(Clone, Debug)]
/// The settings of a page.
pub struct PageSettings {
    /// The media box of the page, which defines the visible area of the surface.
    media_box: Option<Rect>,
    /// The page label of the page.
    page_label: PageLabel,
    /// The size of the surface.
    surface_size: Size,
    /// The crop box of the page
    crop_box: Option<Rect>,
    /// The bleed box of the page
    bleed_box: Option<Rect>,
    /// The trim box of the page
    trim_box: Option<Rect>,
    /// The actual content boundaries
    art_box: Option<Rect>,
}

impl PageSettings {
    /// Create new page settings and define the size of the page surface.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            media_box: Some(Rect::from_xywh(0.0, 0.0, width, height).unwrap()),
            surface_size: Size::from_wh(width, height).unwrap(),
            ..Default::default()
        }
    }

    /// Change the media box.
    ///
    /// The media box defines the visible area of the page when opening the PDF,
    /// so it can be distinct from the size of the surface, but in the majority
    /// of the cases you want them to match in size and align the media box
    /// at the origin of the coordinate system.
    ///
    /// If set to `None`, the dimensions will be chosen in such a way that all
    /// contents fit on the page.
    pub fn with_media_box(mut self, media_box: Option<Rect>) -> PageSettings {
        self.media_box = media_box;
        self
    }

    /// Change the page label.
    pub fn with_page_label(mut self, page_label: PageLabel) -> PageSettings {
        self.page_label = page_label;
        self
    }

    /// The current media box.
    pub(crate) fn media_box(&self) -> Option<Rect> {
        self.media_box
    }

    /// The current surface size.
    pub(crate) fn surface_size(&self) -> Size {
        self.surface_size
    }

    /// The current page label.
    pub(crate) fn page_label(&self) -> &PageLabel {
        &self.page_label
    }

    /// Change the crop box.
    ///
    /// The crop box defines the region to which the page contents are to be clipped
    /// when displayed or printed. Default is the media box.
    ///
    /// If `None`, no /CropBox attribute will be written to the page.
    pub fn with_crop_box(mut self, crop_box: Option<Rect>) -> PageSettings {
        self.crop_box = crop_box;
        self
    }

    /// The current crop box.
    pub(crate) fn crop_box(&self) -> Option<Rect> {
        self.crop_box
    }

    /// Change the bleed box.
    ///
    /// The bleed box defines the region to which the page contents needs to be clipped
    /// when output in a production environment. It includes any extra bleed area needed
    /// for printing.
    ///
    /// If `None`, no /BleedBox attribute will be written to the page.
    pub fn with_bleed_box(mut self, bleed_box: Option<Rect>) -> PageSettings {
        self.bleed_box = bleed_box;
        self
    }

    /// The current bleed box.
    pub(crate) fn bleed_box(&self) -> Option<Rect> {
        self.bleed_box
    }

    /// Change the trim box.
    ///
    /// The trim box defines the intended dimensions of the finished page after trimming.
    /// It may be smaller than the media box and bleed box to accommodate bleed for printing.
    ///
    /// If `None`, no /TrimBox attribute will be written to the page.
    pub fn with_trim_box(mut self, trim_box: Option<Rect>) -> PageSettings {
        self.trim_box = trim_box;
        self
    }

    /// The current trim box.
    pub(crate) fn trim_box(&self) -> Option<Rect> {
        self.trim_box
    }

    /// Change the art box.
    ///
    /// The art box defines the extent of the page's meaningful content (including
    /// potential white space) as intended by the page's creator.
    ///
    /// If `None`, no /ArtBox attribute will be written to the page.
    pub fn with_art_box(mut self, art_box: Option<Rect>) -> PageSettings {
        self.art_box = art_box;
        self
    }

    /// The current art box.
    pub(crate) fn art_box(&self) -> Option<Rect> {
        self.art_box
    }
}

impl Default for PageSettings {
    fn default() -> Self {
        // Default for A4.
        let width = 595.0;
        let height = 842.0;

        Self {
            media_box: Some(Rect::from_xywh(0.0, 0.0, width, height).unwrap()),
            surface_size: Size::from_wh(width, height).unwrap(),
            page_label: PageLabel::default(),
            crop_box: None,
            bleed_box: None,
            trim_box: None,
            art_box: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::metadata::{DateTime, Metadata};
    use crate::Document;
    use krilla_macros::snapshot;

    #[snapshot(document)]
    fn empty_document(_: &mut Document) {}

    #[snapshot(document)]
    fn metadata_empty(document: &mut Document) {
        let metadata = Metadata::new();
        document.set_metadata(metadata);
    }

    fn metadata_impl(document: &mut Document) {
        let date = DateTime::new(2024)
            .month(11)
            .day(8)
            .hour(22)
            .minute(23)
            .second(18)
            .utc_offset_hour(1)
            .utc_offset_minute(12);
        let metadata = Metadata::new()
            .creation_date(date)
            .subject("A very interesting subject".to_string())
            .modification_date(date)
            .creator("krilla".to_string())
            .producer("krilla".to_string())
            .language("en".to_string())
            .keywords(vec![
                "keyword1".to_string(),
                "keyword2".to_string(),
                "keyword3".to_string(),
            ])
            .title("An awesome title".to_string())
            .authors(vec!["John Doe".to_string(), "Max Mustermann".to_string()]);
        document.set_metadata(metadata);
    }

    #[snapshot(document)]
    fn metadata_full(document: &mut Document) {
        metadata_impl(document);
    }

    #[snapshot(document, settings_5)]
    fn metadata_full_with_xmp(document: &mut Document) {
        metadata_impl(document);
    }

    #[snapshot(document, settings_16)]
    fn pdf_version_14(document: &mut Document) {
        metadata_impl(document);
    }
}
