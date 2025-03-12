//! Embedding attachments to a PDF file.

use std::ops::DerefMut;

use pdf_writer::{Chunk, Finish, Name, Ref, Str, TextStr};

use crate::configure::{PdfVersion, ValidationError};
use crate::metadata::pdf_date;
use crate::object::{Cacheable, ChunkContainerFn};
use crate::serialize::SerializeContext;
use crate::stream::FilterStreamBuilder;
use crate::util::NameExt;
use crate::Data;

pub use pdf_writer::types::AssociationKind;

/// An error while embedding the file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EmbedError {
    /// The selected standard does not support embedding files.
    Existence,
    /// The document doesn't contain a modification date, which is required for embedded files
    /// in some export modes.
    MissingDate,
    /// The embedded file is missing a human-readable description.
    MissingDescription,
    /// The mime type of the embedded file is missing.
    MissingMimeType,
}

/// An embedded file.
#[derive(Debug, Clone, Hash)]
pub struct EmbeddedFile {
    /// The name of the embedded file.
    pub path: String,
    /// The mime type of the embedded file.
    pub mime_type: Option<String>,
    /// A description of the embedded file.
    pub description: Option<String>,
    /// The association kind of the embedded file.
    pub association_kind: AssociationKind,
    /// The raw data of the embedded file.
    pub data: Data,
    /// Whether the embedded file should be compressed (recommended to turn off if the
    /// original file already has compression).
    pub compress: bool,
}

impl Cacheable for EmbeddedFile {
    fn chunk_container(&self) -> ChunkContainerFn {
        |cc| &mut cc.embedded_files
    }

    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk {
        sc.register_validation_error(ValidationError::EmbeddedFile(EmbedError::Existence));

        let mut chunk = Chunk::new();
        let stream_ref = sc.new_ref();

        let file_stream = if self.compress {
            FilterStreamBuilder::new_from_binary_data(self.data.as_ref())
        } else {
            FilterStreamBuilder::new_from_uncompressed(self.data.as_ref())
        }
        .finish(&sc.serialize_settings());

        let mut embedded_file_stream = chunk.embedded_file(stream_ref, file_stream.encoded_data());
        file_stream.write_filters(embedded_file_stream.deref_mut().deref_mut());

        if let Some(mime_type) = &self.mime_type {
            embedded_file_stream.subtype(mime_type.to_pdf_name());
        } else {
            sc.register_validation_error(ValidationError::EmbeddedFile(
                EmbedError::MissingMimeType,
            ));
        }

        let mut params = embedded_file_stream.params();
        params.size(self.data.as_ref().len() as i32);

        if let Some(date_time) = sc
            .metadata()
            .and_then(|m| m.modification_date.or(m.creation_date))
        {
            let date = pdf_date(date_time);
            params.modification_date(date);
        } else {
            sc.register_validation_error(ValidationError::EmbeddedFile(EmbedError::MissingDate));
        }

        params.finish();
        embedded_file_stream.finish();

        let mut file_spec = chunk.file_spec(root_ref);
        file_spec.path(Str(self.path.as_bytes()));

        if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf17 {
            file_spec.unic_file(TextStr(&self.path));
        }

        let mut ef = file_spec.insert(Name(b"EF")).dict();
        ef.pair(Name(b"F"), stream_ref);

        if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf17 {
            ef.pair(Name(b"UF"), stream_ref);
        }

        ef.finish();

        if sc
            .serialize_settings()
            .validator()
            .allows_associated_files()
        {
            file_spec.association_kind(self.association_kind);
        }

        if let Some(description) = self.description {
            file_spec.description(TextStr(&description));
        } else {
            sc.register_validation_error(ValidationError::EmbeddedFile(
                EmbedError::MissingDescription,
            ));
        }

        file_spec.finish();

        chunk
    }
}
