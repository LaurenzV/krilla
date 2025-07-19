//! Including other PDF files.

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::{Arc, OnceLock};

use hayro_write::{ExtractionError, ExtractionQuery};
use pdf_writer::Ref;

pub use hayro_write::{Page, Pdf};

use crate::chunk_container::EmbeddedPdfChunk;
use crate::configure::{PdfVersion, ValidationError};
use crate::error::{KrillaError, KrillaResult};
use crate::serialize::SerializeContext;
use crate::surface::Location;
use crate::util::{Deferred, Prehashed};

/// An error that can occur when embedding a PDF document.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PdfError {
    /// A page was requested that doesn't exist.
    InvalidPage(usize),
    /// The PDF version of the embedded PDF is not compatible with the version of the PDF
    /// produced by krilla. This happens if the version of the krilla document is lower than
    /// the one of the embedded PDF.
    ///
    /// The argument indicates the version of the embedded PDF document.
    VersionMismatch(PdfVersion),
}

struct PdfDocumentRepr(Arc<Pdf>);

impl Debug for PdfDocumentRepr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PdfDocumentRepr {{ .. }}")
    }
}

impl Hash for PdfDocumentRepr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.data().as_ref().as_ref().hash(state);
    }
}

/// An external PDF document.
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct PdfDocument(Arc<Prehashed<PdfDocumentRepr>>);

impl PdfDocument {
    /// Load a new PDF document.
    pub fn new(pdf: Arc<Pdf>) -> PdfDocument {
        Self(Arc::new(Prehashed::new(PdfDocumentRepr(pdf))))
    }

    pub(crate) fn pdf(&self) -> &Pdf {
        &self.0.deref().0
    }

    pub(crate) fn pages(&self) -> &[Page] {
        self.0.deref().0.pages()
    }
}

#[derive(Default, Debug)]
pub(crate) struct PdfDocumentInfo {
    counter: u64,
    query_refs: Vec<Ref>,
    queries: Vec<ExtractionQuery>,
    locations: Vec<Option<Location>>,
}

impl PdfDocumentInfo {
    pub(crate) fn new(counter: u64) -> Self {
        Self {
            counter,
            ..Self::default()
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct PdfSerializerContext {
    infos: HashMap<PdfDocument, PdfDocumentInfo>,
    counter: u64,
}

impl PdfSerializerContext {
    pub(crate) fn add_page(
        &mut self,
        document: &PdfDocument,
        page_index: usize,
        ref_: Ref,
        location: Option<Location>,
    ) {
        let info = self.get_info(document);

        info.query_refs.push(ref_);
        info.queries.push(ExtractionQuery::new_page(page_index));
        info.locations.push(location);
    }

    fn get_info(&mut self, document: &PdfDocument) -> &mut PdfDocumentInfo {
        self.infos.entry(document.clone()).or_insert_with(|| {
            let info = PdfDocumentInfo::new(self.counter);
            self.counter += 1;

            info
        })
    }

    pub(crate) fn add_xobject(
        &mut self,
        document: &PdfDocument,
        page_index: usize,
        ref_: Ref,
        location: Option<Location>,
    ) {
        let info = self.get_info(document);

        info.query_refs.push(ref_);
        info.queries.push(ExtractionQuery::new_xobject(page_index));
        info.locations.push(location);
    }

    pub(crate) fn serialize(self, sc: &mut SerializeContext) -> KrillaResult<()> {
        let page_tree_parent_ref = sc.page_tree_ref();
        let krilla_version = sc.serialize_settings().configuration.version();

        let mut entries = self.infos.into_iter().collect::<Vec<_>>();
        // Make sure we always process them in the same order.
        entries.sort_by(|d1, d2| d1.1.counter.cmp(&d2.1.counter));

        for (doc, info) in entries {
            for location in info.locations.iter() {
                sc.register_validation_error(ValidationError::EmbeddedPDF(*location))
            }

            let container = &mut sc.chunk_container;

            let deferred_chunk = Deferred::new(move || {
                // We can't share the serializer context between threads, so each PDF has it's own
                // reference, and we remap it later in `ChunkContainer`.
                let mut new_ref = Ref::new(1);

                let first_location = info.locations.iter().flatten().next().cloned();
                let pdf = doc.pdf();

                let pdf_version = convert_pdf_version(pdf.version());

                if krilla_version < pdf_version {
                    return Err(KrillaError::Pdf(
                        doc.clone(),
                        PdfError::VersionMismatch(pdf_version),
                        first_location,
                    ));
                }

                let extracted =
                    hayro_write::extract(pdf, Box::new(|| new_ref.bump()), &info.queries);
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

fn convert_pdf_version(version: hayro_write::PdfVersion) -> PdfVersion {
    match version {
        // Those are obviously not right, but we don't support versions lower than 1.4 in krilla.
        // Since we only need this conversion to detect version mismatches (in which case the
        // version of the embedded PDF has to be higher than 1.4), this hack is sufficient for our
        // purposes.
        hayro_write::PdfVersion::Pdf10 => PdfVersion::Pdf14,
        hayro_write::PdfVersion::Pdf11 => PdfVersion::Pdf14,
        hayro_write::PdfVersion::Pdf12 => PdfVersion::Pdf14,
        hayro_write::PdfVersion::Pdf13 => PdfVersion::Pdf14,

        hayro_write::PdfVersion::Pdf14 => PdfVersion::Pdf14,
        hayro_write::PdfVersion::Pdf15 => PdfVersion::Pdf15,
        hayro_write::PdfVersion::Pdf16 => PdfVersion::Pdf16,
        hayro_write::PdfVersion::Pdf17 => PdfVersion::Pdf17,
        hayro_write::PdfVersion::Pdf20 => PdfVersion::Pdf20,
    }
}

fn convert_extraction_result<T>(
    result: Result<T, ExtractionError>,
    doc: &PdfDocument,
    location: Option<&Location>,
) -> KrillaResult<T> {
    result.map_err(|e| {
        let pdf_error = match e {
            ExtractionError::InvalidPageIndex(i) => PdfError::InvalidPage(i),
        };

        KrillaError::Pdf(doc.clone(), pdf_error, location.cloned())
    })
}
