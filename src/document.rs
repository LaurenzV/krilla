//! Creating new PDF documents.
//!
//! When using krilla, the starting point is always the creation of a `Document`. A document
//! represents _one_ PDF document, to which you can add pages or configure them in any
//! other way you want.
//!
//! Unfortunately, creating PDFs always requires some kind of global state to keep track
//! of different aspects in the creation process, meaning that it is not possible to
//! generate multiple pages at the same time. Instead, you need to add pages separately
//! by calling the `add_page` method, which returns a new `Page` object that mutably
//! borrows the global state from the document. Once the page is dropped, the global
//! state is passed back to the original document, which you can then use to add even
//! more pages.

use crate::error::KrillaResult;
use crate::object::outline::Outline;
use crate::object::page::Page;
use crate::object::page::PageLabel;
use crate::serialize::{SerializeSettings, SerializerContext};
use tiny_skia_path::Rect;

/// A PDF document.
pub struct Document {
    serializer_context: SerializerContext,
}

impl Document {
    /// Create a new document with default settings.
    pub fn new() -> Self {
        Self {
            serializer_context: SerializerContext::new(SerializeSettings::default()),
        }
    }

    /// Create a new document with specific serialization settings.
    pub fn new_with(serialize_settings: SerializeSettings) -> Self {
        Self {
            serializer_context: SerializerContext::new(serialize_settings),
        }
    }

    /// Start a new page with default settings.
    pub fn start_page(&mut self) -> Page {
        Page::new(&mut self.serializer_context, PageSettings::default())
    }

    /// Start a new page with specific page settings.
    pub fn start_page_with(&mut self, page_settings: PageSettings) -> Page {
        Page::new(&mut self.serializer_context, page_settings)
    }

    /// Set the outline of the document.
    pub fn set_outline(&mut self, outline: Outline) {
        self.serializer_context.set_outline(outline);
    }

    /// Attempt to write the document to a PDF.
    pub fn finish(self) -> KrillaResult<Vec<u8>> {
        Ok(self.serializer_context.finish()?.finish())
    }
}

#[derive(Clone)]
/// The settings of a page.
pub struct PageSettings {
    /// The media box of the page.
    ///
    /// **Default**: The dimensions of an A4 page.
    pub media_box: Rect,
    /// The page label of the page.
    ///
    /// **Default**: No page label.
    pub page_label: PageLabel,
}

impl PageSettings {
    pub fn with_size(width: f32, height: f32) -> PageSettings {
        PageSettings {
            media_box: Rect::from_xywh(0.0, 0.0, width, height).unwrap(),
            ..Default::default()
        }
    }
}

impl Default for PageSettings {
    fn default() -> Self {
        Self {
            media_box: Rect::from_xywh(0.0, 0.0, 595.2765, 841.89108).unwrap(),
            page_label: PageLabel::default(),
        }
    }
}
