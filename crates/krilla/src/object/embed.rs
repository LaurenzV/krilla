//! Embedding attachments to a PDF file.

use crate::configure::{PdfVersion, ValidationError};
use crate::metadata::pdf_date;
use crate::object::{Cacheable, ChunkContainerFn};
use crate::serialize::SerializeContext;
use crate::stream::FilterStreamBuilder;
use crate::util::NameExt;
use crate::Data;

use pdf_writer::{Chunk, Finish, Name, Ref, Str, TextStr};

use std::ops::DerefMut;

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

#[cfg(test)]
mod tests {
    use crate::configure::ValidationError;
    use crate::embed::{EmbedError, EmbeddedFile};
    use crate::error::KrillaError;
    use crate::metadata::{DateTime, Metadata};
    use crate::tagging::TagTree;
    use crate::tests::ASSETS_PATH;
    use crate::{Document, SerializeSettings};
    use krilla_macros::snapshot;
    use pdf_writer::types::AssociationKind;

    fn file_1() -> EmbeddedFile {
        let data = std::fs::read(ASSETS_PATH.join("emojis.txt")).unwrap();
        EmbeddedFile {
            path: "emojis.txt".to_string(),
            mime_type: Some("text/txt".to_string()),
            description: Some("The description of the file.".to_string()),
            association_kind: AssociationKind::Supplement,
            data: data.into(),
            compress: false,
        }
    }

    fn file_2() -> EmbeddedFile {
        let data =
            std::fs::read(ASSETS_PATH.join("svgs/resvg_structure_svg_nested_svg_with_rect.svg"))
                .unwrap();
        EmbeddedFile {
            path: "image.svg".to_string(),
            mime_type: Some("image/svg+xml".to_string()),
            description: Some("A nice SVG image!".to_string()),
            association_kind: AssociationKind::Supplement,
            data: data.into(),
            compress: false,
        }
    }

    fn file_3() -> EmbeddedFile {
        let data = std::fs::read(ASSETS_PATH.join("images/rgb8.png")).unwrap();

        EmbeddedFile {
            path: "rgb8.png".to_string(),
            mime_type: Some("image/png".to_string()),
            description: Some("A nice picture.".to_string()),
            association_kind: AssociationKind::Unspecified,
            data: data.into(),
            compress: false,
        }
    }

    #[snapshot(document)]
    fn embedded_file(d: &mut Document) {
        let file = file_1();
        d.embed_file(file);
    }

    #[snapshot(document)]
    fn embedded_file_with_compression(d: &mut Document) {
        let mut file = file_1();
        file.compress = true;

        d.embed_file(file);
    }

    #[snapshot(document)]
    fn multiple_embedded_files(d: &mut Document) {
        let f1 = file_1();
        let f2 = file_2();
        let f3 = file_3();

        d.embed_file(f1);
        d.embed_file(f2);
        d.embed_file(f3);
    }

    fn embedded_file_impl(d: &mut Document) {
        let metadata = Metadata::new()
            .modification_date(DateTime::new(2001))
            .language("en".to_string());
        d.set_metadata(metadata);
        let f1 = file_1();
        d.embed_file(f1);
    }

    #[snapshot(document, settings_23)]
    fn validation_pdf_a3_with_embedded_file(d: &mut Document) {
        embedded_file_impl(d)
    }

    #[snapshot(document, settings_27)]
    fn validation_pdf_a4f_with_embedded_file(d: &mut Document) {
        embedded_file_impl(d)
    }

    #[snapshot(document, settings_25)]
    fn pdf_20_with_embedded_file(d: &mut Document) {
        // Technically PDF 2.0 supports associated files, but we only use them for PDF/A-3.
        embedded_file_impl(d)
    }

    #[test]
    fn duplicate_embedded_file() {
        let mut d = Document::new();
        let f1 = file_1();
        let mut f2 = file_2();
        f2.path = f1.path.clone();

        assert!(d.embed_file(f1).is_some());
        assert!(d.embed_file(f2).is_none());
    }

    #[test]
    fn pdf_a3_missing_fields() {
        let mut d = Document::new_with(SerializeSettings::settings_23());
        let mut f1 = file_1();
        f1.description = None;
        d.embed_file(f1);

        assert_eq!(
            d.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::EmbeddedFile(EmbedError::MissingDate),
                ValidationError::EmbeddedFile(EmbedError::MissingDescription)
            ]))
        )
    }

    #[test]
    fn pdf_a2_embedded_file() {
        let mut d = Document::new_with(SerializeSettings::settings_13());
        let metadata = Metadata::new().language("en".to_string());
        d.set_metadata(metadata);
        d.set_tag_tree(TagTree::new());

        let mut f1 = file_1();
        f1.description = None;
        d.embed_file(f1);

        assert_eq!(
            d.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::EmbeddedFile(EmbedError::Existence),
            ]))
        )
    }
}
