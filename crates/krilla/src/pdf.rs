//! Including other PDF files.

use crate::util::Prehashed;
use crate::{Data, Document};
use hayro_write::ExtractionQuery;
use pdf_writer::Ref;
use std::collections::HashMap;
use std::sync::Arc;

/// An external PDF document.
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct PdfDocument(Arc<Prehashed<Data>>);

impl PdfDocument {
    /// Load a new PDF document from the given data.
    pub fn new(data: Data) -> Self {
        Self(Arc::new(Prehashed::new(data)))
    }
}

impl Document {
    /// Embed the pages (starting with page index 0 for the first page) from the given
    /// PDF document.
    pub fn embed_pdf_pages(&mut self, pdf: &PdfDocument, page_indices: &[usize]) {}
}

pub(crate) struct PdfDocumentInfo {
    query_refs: Vec<Ref>,
    queries: Vec<ExtractionQuery>,
}

impl PdfDocumentInfo {
    pub fn new() -> Self {
        Self {
            query_refs: Vec::new(),
            queries: Vec::new(),
        }
    }
}

pub(crate) struct PdfSerializerContext {
    // TODO: Ensure reproducible output when writing.
    infos: HashMap<PdfDocument, PdfDocumentInfo>,
}

impl PdfSerializerContext {
    pub(crate) fn new() -> Self {
        Self {
            infos: HashMap::new(),
        }
    }

    pub(crate) fn add_page(&mut self, document: &PdfDocument, page_index: usize, ref_: Ref) {
        let info = self
            .infos
            .entry(document.clone())
            .or_insert(PdfDocumentInfo::new());

        info.query_refs.push(ref_);
        info.queries.push(ExtractionQuery::new_page(page_index));
    }

    pub(crate) fn add_xobject(&mut self, document: &PdfDocument, page_index: usize, ref_: Ref) {
        let info = self
            .infos
            .entry(document.clone())
            .or_insert(PdfDocumentInfo::new());

        info.query_refs.push(ref_);
        info.queries.push(ExtractionQuery::new_xobject(page_index));
    }
}
