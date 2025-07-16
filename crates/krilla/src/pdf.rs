//! Including other PDF files.

// TODO: Prohibit PDFs with validated export.
use crate::chunk_container::{ChunkContainer, EmbeddedPdfChunk};
use crate::error::{KrillaError, KrillaResult};
use crate::serialize::SerializeContext;
use crate::surface::Location;
use crate::util::{Deferred, Prehashed};
use crate::{Data, Document};
use hayro_write::{ExtractionError, ExtractionQuery, PdfData};
use pdf_writer::{Chunk, Ref};
use std::cell::OnceCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, OnceLock};

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

#[derive(Default, Debug)]
pub(crate) struct PdfDocumentInfo {
    query_refs: Vec<Ref>,
    queries: Vec<ExtractionQuery>,
    locations: Vec<Option<Location>>,
}

impl PdfDocumentInfo {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Default, Debug)]
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
            .or_insert(PdfDocumentInfo::new());

        info.query_refs.push(ref_);
        info.queries.push(ExtractionQuery::new_page(page_index));
        info.locations.push(location);
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
            .or_insert(PdfDocumentInfo::new());

        info.query_refs.push(ref_);
        info.queries.push(ExtractionQuery::new_xobject(page_index));
        info.locations.push(location);
    }

    pub(crate) fn serialize(
        self,
        page_tree_parent_ref: Ref,
        container: &mut ChunkContainer,
    ) -> KrillaResult<()> {
        for (doc, info) in self.infos {
            let deferred_chunk = Deferred::new(move || {
                // We can't share the serializer context between threads, so each PDF has it's own
                // reference, and we remap it later in `ChunkContainer`.
                let mut new_ref = Ref::new(1);

                // TODO: Don't just return an `Option` in hayro.
                let data: PdfData = doc.0.deref().0.clone();
                let first_location = info.locations.iter().flat_map(|l| l).next().cloned();
                let pdf = hayro_write::Pdf::new(data).ok_or(KrillaError::Pdf(
                    doc.clone(),
                    PdfError::LoadFailed,
                    first_location,
                ))?;

                let extracted =
                    hayro_write::extract(&pdf, Box::new(|| new_ref.bump()), &info.queries);
                let result = convert_extraction_result(extracted, &doc, first_location.as_ref())?;

                debug_assert_eq!(info.query_refs.len(), result.root_refs.len());

                let mut root_ref_mappings = HashMap::new();

                root_ref_mappings.insert(result.page_tree_parent_ref, page_tree_parent_ref);

                for ((should_ref, extraction_result), location) in info
                    .query_refs
                    .iter()
                    .zip(result.root_refs)
                    .zip(&info.locations)
                {
                    let assigned_ref = convert_extraction_result(
                        extraction_result,
                        &doc,
                        location.clone().as_ref(),
                    )?;

                    root_ref_mappings.insert(assigned_ref, *should_ref);
                }

                Ok(EmbeddedPdfChunk {
                    root_ref_mappings,
                    original_chunk: result.chunk,
                    new_chunk: OnceLock::new(),
                })
            });

            container.embedded_pdfs.push(deferred_chunk);
        }

        Ok(())
    }
}

fn convert_extraction_result<T>(
    result: Result<T, ExtractionError>,
    doc: &PdfDocument,
    location: Option<&Location>,
) -> KrillaResult<T> {
    result.map_err(|e| {
        let pdf_error = match e {
            ExtractionError::LoadPdfError => PdfError::LoadFailed,
            ExtractionError::InvalidPageIndex(i) => PdfError::InvalidPage(i),
            ExtractionError::InvalidPdf => PdfError::LoadFailed,
        };

        KrillaError::Pdf(doc.clone(), pdf_error, location.cloned())
    })
}
