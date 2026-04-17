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
//! [`Page`]: Page

use crate::destination::NamedDestination;
use crate::error::KrillaResult;
use crate::interchange::embed::EmbeddedFile;
use crate::interchange::metadata::Metadata;
use crate::interchange::outline::Outline;
use crate::interchange::tagging::TagTree;
use crate::page::{Page, PageSettings};
#[cfg(feature = "pdf")]
use crate::pdf::PdfDocument;
use crate::serialize::{SerializeContext, SerializeSettings};
use crate::surface::Location;

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
    /// Create a new document with default serialize settings.
    pub fn new() -> Self {
        Self {
            serializer_context: SerializeContext::new(SerializeSettings::default()),
        }
    }

    /// Create a new document with custom serialize settings.
    pub fn new_with(serialize_settings: SerializeSettings) -> Self {
        Self {
            serializer_context: SerializeContext::new(serialize_settings),
        }
    }

    /// Start a new page with default settings.
    pub fn start_page(&mut self) -> Page<'_> {
        let page_index = self.serializer_context.page_infos().iter().len();
        Page::new(
            &mut self.serializer_context,
            page_index,
            PageSettings::default(),
        )
    }

    /// Start a new page with specific page settings.
    pub fn start_page_with(&mut self, page_settings: PageSettings) -> Page<'_> {
        let page_index = self.serializer_context.page_infos().iter().len();
        Page::new(&mut self.serializer_context, page_index, page_settings)
    }

    /// Embed the pages (0-indexed) from the given
    /// PDF document.
    #[cfg(feature = "pdf")]
    pub fn embed_pdf_pages(&mut self, pdf: &PdfDocument, page_indices: &[usize]) {
        self.serializer_context.embed_pdf_pages(pdf, page_indices);
    }

    /// Set the location that should be assumed for subsequent operations.
    pub fn set_location(&mut self, location: Location) {
        self.serializer_context.set_location(location);
    }

    /// Reset the location that should be assumed for subsequent operations.
    pub fn reset_location(&mut self) {
        self.serializer_context.reset_location();
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

    /// Embed a new file in the PDF document.
    ///
    /// Returns `None` if the file couldn't be embedded because a file
    /// with the same name has already been embedded.
    pub fn embed_file(&mut self, file: EmbeddedFile) -> Option<()> {
        self.serializer_context.embed_file(file)
    }

    /// Manually register a global named destination.
    ///
    /// Named destinations used in link annotations are automatically registered, so you don't need
    /// to call this function for them.
    pub fn register_named_destination(&mut self, dest: NamedDestination) {
        self.serializer_context.register_named_destination(dest);
    }

    /// Attempt to export the document to a PDF file.
    pub fn finish(mut self) -> KrillaResult<Vec<u8>> {
        // Write empty page if none has been created yet.
        if self.serializer_context.page_infos().is_empty() {
            self.start_page();
        }

        Ok(self.serializer_context.finish()?.finish())
    }

    /// Attempt to export the document as PDF bytes, streaming them to a
    /// [`std::io::Write`] sink instead of returning them as a `Vec<u8>`.
    ///
    /// Semantically, this produces the same PDF as [`Document::finish`] —
    /// the byte stream written to `writer` is identical to the `Vec<u8>`
    /// that `finish()` would return. Use this when you want to pipe the
    /// output straight to a file, socket, pipe, or any other writer
    /// without allocating the full document in memory first.
    ///
    /// # Errors
    ///
    /// Returns the usual [`KrillaError`] variants on serialization
    /// failures, plus [`KrillaError::Io`] wrapping the underlying
    /// [`io::Error`] message if the writer fails.
    ///
    /// [`KrillaError`]: crate::error::KrillaError
    /// [`KrillaError::Io`]: crate::error::KrillaError::Io
    /// [`io::Error`]: std::io::Error
    pub fn finish_to_writer<W: std::io::Write>(mut self, mut writer: W) -> KrillaResult<()> {
        // Write empty page if none has been created yet.
        if self.serializer_context.page_infos().is_empty() {
            self.start_page();
        }

        let bytes = self.serializer_context.finish()?.finish();
        writer
            .write_all(&bytes)
            .map_err(|e| crate::error::KrillaError::Io(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two documents built identically must produce identical bytes whether
    /// we call `finish()` (returning a Vec) or `finish_to_writer()`
    /// (streaming into a Vec-as-writer).
    #[test]
    fn finish_to_writer_matches_finish() {
        fn build() -> Document {
            let mut doc = Document::new();
            doc.start_page_with(PageSettings::from_wh(200.0, 150.0).expect("valid size"));
            doc.start_page_with(PageSettings::from_wh(300.0, 200.0).expect("valid size"));
            doc
        }

        let via_finish = build().finish().expect("finish");

        let mut via_writer: Vec<u8> = Vec::new();
        build()
            .finish_to_writer(&mut via_writer)
            .expect("finish_to_writer");

        assert_eq!(via_finish, via_writer);
    }

    /// A writer that always fails must surface as `KrillaError::Io`.
    #[test]
    fn finish_to_writer_propagates_io_error() {
        struct FailingWriter;
        impl std::io::Write for FailingWriter {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "boom"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let mut doc = Document::new();
        doc.start_page_with(PageSettings::from_wh(100.0, 100.0).expect("valid size"));

        let err = doc
            .finish_to_writer(FailingWriter)
            .expect_err("should fail");
        assert!(matches!(err, crate::error::KrillaError::Io(_)));
    }
}
