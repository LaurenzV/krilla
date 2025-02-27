//! Embedding attachments to a PDF file.

use std::sync::Arc;
use std::ops::DerefMut;
use pdf_writer::{Chunk, Finish, Ref};
use pdf_writer::types::AssociationKind;
use crate::metadata::pdf_date;
use crate::object::{Cacheable, ChunkContainerFn};
use crate::serialize::SerializeContext;
use crate::stream::FilterStreamBuilder;
use crate::util::NameExt;

/// An error while embedding the file.
pub enum EmbedError {
    /// The document doesn't contain a date, which is required for embedded files
    /// in some export modes.
    MissingDate
}

/// An embedded file.
#[derive(Debug, Clone, Hash)]
pub struct EmbeddedFile {
    /// The name of the embedded file.
    pub name: String,
    /// The mime type of the embedded file.
    pub mime_type: String,
    /// A description of the embedded file.
    pub description: String,
    /// The association kind of the embedded file.
    pub association_kind: AssociationKind,
    /// The raw data of the embedded file.
    pub data: Arc<Vec<u8>>,
    /// Whether the embedded file should be compressed (recommended to turn off if the
    /// original file already has compression).
    pub compress: bool
}

impl Cacheable for EmbeddedFile {
    fn chunk_container(&self) -> ChunkContainerFn {
        Box::new(|cc| &mut cc.embedded_files)
    }

    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();
        let stream_ref = sc.new_ref();

        let file_stream = if self.compress {
            FilterStreamBuilder::new_from_binary_data(&self.data)
        }   else {
            FilterStreamBuilder::new_from_uncompressed(&self.data)
        }.finish(&sc.serialize_settings());
        
        let mut embedded_file_stream = chunk.embedded_file(stream_ref, &file_stream.encoded_data());
        file_stream.write_filters(embedded_file_stream.deref_mut().deref_mut());
        
        embedded_file_stream.subtype(self.mime_type.to_pdf_name());
        let mut params = embedded_file_stream.params();
        params.size(self.data.len() as i32);

        if let Some(date_time) = sc.metadata()
            .and_then(|m| m.modification_date.or_else(|| m.creation_date))
        {
            let date = pdf_date(date_time);
            params.modification_date(date);
        }   else {
            todo!();
        }
        
        params.finish();
        embedded_file_stream.finish();
        
        chunk   
    }
}

