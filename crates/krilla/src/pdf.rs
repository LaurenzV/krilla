//! Including other PDF files.

use crate::surface::Location;
use crate::util::Prehashed;
use crate::{Data, Document};
use hayro_write::ExtractionQuery;
use pdf_writer::Ref;
use std::collections::HashMap;
use std::sync::Arc;

/// An error that can occur when embedding a PDF document.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PdfError {
    /// The PDF document is encrypted. Encrypted PDF documents are _currently_ not supported.
    Encrypted,
    /// The PDF failed to load, either because it's broken or due to a bug.
    LoadFailed,
    /// A page was requested that doesn't exist.
    InvalidPage(usize),
}

/// An external PDF document.
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct PdfDocument(Arc<Prehashed<Data>>);

impl PdfDocument {
    /// Load a new PDF document from the given data.
    pub fn new(data: Data) -> Self {
        Self(Arc::new(Prehashed::new(data)))
    }
}

pub(crate) struct PdfDocumentInfo {
    query_refs: Vec<Ref>,
    queries: Vec<ExtractionQuery>,
    location: Option<Location>,
}

impl PdfDocumentInfo {
    pub fn new(location: Option<Location>) -> Self {
        Self {
            query_refs: Vec::new(),
            queries: Vec::new(),
            location,
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

    pub(crate) fn add_page(
        &mut self,
        document: &PdfDocument,
        page_index: usize,
        ref_: Ref,
        location: Option<Location>,
    ) {
        let info = self
            .infos
            .entry(document.clone())
            .or_insert(PdfDocumentInfo::new(location));

        info.query_refs.push(ref_);
        info.queries.push(ExtractionQuery::new_page(page_index));
    }

    pub(crate) fn add_xobject(
        &mut self,
        document: &PdfDocument,
        page_index: usize,
        ref_: Ref,
        location: Option<Location>,
    ) {
        let info = self
            .infos
            .entry(document.clone())
            .or_insert(PdfDocumentInfo::new(location));

        info.query_refs.push(ref_);
        info.queries.push(ExtractionQuery::new_xobject(page_index));
    }
}
