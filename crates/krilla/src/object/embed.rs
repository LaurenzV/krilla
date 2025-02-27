use std::sync::Arc;
use pdf_writer::types::AssociationKind;

/// An embedded file.
#[derive(Debug, Clone)]
pub struct EmbeddedFile {
    pub name: String,
    pub mime_type: String,
    pub description: String,
    pub association_kind: AssociationKind,
    pub data: Arc<Vec<u8>>
}