//! Setting document metadata.

use pdf_writer::{Date, Pdf, Ref, TextStr};

/// Metadata for a PDF document.
#[derive(Default, Clone)]
pub struct Metadata {
    title: Option<String>,
    subject: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    keywords: Option<Vec<String>>,
    authors: Option<Vec<String>>,
    modification_date: Option<Date>,
    creation_date: Option<Date>,
}

impl Metadata {
    /// Create new metadata.
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// The title of the document.
    pub fn title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    /// The subject of the document.
    pub fn subject(mut self, subject: String) -> Self {
        self.subject = Some(subject);
        self
    }

    /// The keywords that describe the document.
    pub fn keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = Some(keywords);
        self
    }

    /// The creator tool of the document.
    pub fn creator(mut self, creator: String) -> Self {
        self.creator = Some(creator);
        self
    }

    /// The producer tool of the document.
    pub fn producer(mut self, producer: String) -> Self {
        self.producer = Some(producer);
        self
    }

    /// The authors of the document.
    pub fn authors(mut self, authors: Vec<String>) -> Self {
        self.authors = Some(authors);
        self
    }

    /// The creation date of the document.
    pub fn creation_date(mut self, creation_date: Date) -> Self {
        self.creation_date = Some(creation_date);
        self
    }

    /// The modification date of the document.
    pub fn modification_date(mut self, modification_date: Date) -> Self {
        self.modification_date = Some(modification_date);
        self
    }

    pub(crate) fn has_document_info(&self) -> bool {
        self.title.is_some()
            || self.producer.is_some()
            || self.keywords.is_some()
            || self.authors.is_some()
            || self.creator.is_some()
            || self.modification_date.is_some()
            || self.creation_date.is_some()
            || self.subject.is_some()
    }

    pub(crate) fn serialize_document_info(&self, ref_: &mut Ref, pdf: &mut Pdf) {
        if self.has_document_info() {
            let ref_ = ref_.bump();
            let mut document_info = pdf.document_info(ref_);

            if let Some(title) = &self.title {
                document_info.title(TextStr(title));
            }

            if let Some(subject) = &self.subject {
                document_info.subject(TextStr(subject));
            }

            if let Some(keywords) = &self.keywords {
                let joined = keywords.join(", ");
                document_info.keywords(TextStr(&joined));
            }

            if let Some(authors) = &self.authors {
                let joined = authors.join(", ");
                document_info.author(TextStr(&joined));
            }

            if let Some(creator) = &self.creator {
                document_info.creator(TextStr(creator));
            }

            if let Some(producer) = &self.producer {
                document_info.producer(TextStr(producer));
            }

            if let Some(date) = self.modification_date {
                document_info.modified_date(date);
            }

            if let Some(date) = self.creation_date {
                document_info.creation_date(date);
            }
        }
    }
}
