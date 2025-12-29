//! Interchange types for krilla Python bindings (metadata, outline, embed).

use pyo3::prelude::*;

use crate::geometry::Point;

// Import types that aren't at krilla:: root level
use krilla::embed::{EmbeddedFile as KrillaEmbeddedFile, MimeType as KrillaMimeType};
use krilla::outline::Outline as KrillaOutline;

// ==================== DateTime ====================

/// A datetime for PDF metadata.
///
/// Invalid values will be clamped to valid ranges.
#[pyclass]
#[derive(Clone, Copy)]
pub struct DateTime {
    pub(crate) inner: krilla::metadata::DateTime,
}

#[pymethods]
impl DateTime {
    /// Create a new minimal date with just a year.
    ///
    /// The year will be clamped to the range 0-9999.
    #[new]
    fn new(year: u16) -> Self {
        DateTime {
            inner: krilla::metadata::DateTime::new(year),
        }
    }

    /// Add the month field (1-12). Will be clamped to valid range.
    fn month(&self, month: u8) -> Self {
        DateTime {
            inner: self.inner.month(month),
        }
    }

    /// Add the day field (1-31). Will be clamped to valid range.
    fn day(&self, day: u8) -> Self {
        DateTime {
            inner: self.inner.day(day),
        }
    }

    /// Add the hour field (0-23). Will be clamped to valid range.
    fn hour(&self, hour: u8) -> Self {
        DateTime {
            inner: self.inner.hour(hour),
        }
    }

    /// Add the minute field (0-59). Will be clamped to valid range.
    fn minute(&self, minute: u8) -> Self {
        DateTime {
            inner: self.inner.minute(minute),
        }
    }

    /// Add the second field (0-59). Will be clamped to valid range.
    fn second(&self, second: u8) -> Self {
        DateTime {
            inner: self.inner.second(second),
        }
    }

    /// Add the UTC offset in hours (-23 through 23). Will be clamped to valid range.
    fn utc_offset_hour(&self, hour: i8) -> Self {
        DateTime {
            inner: self.inner.utc_offset_hour(hour),
        }
    }

    /// Add the UTC offset in minutes (0-59). Will be clamped to valid range.
    ///
    /// The sign is inherited from utc_offset_hour.
    fn utc_offset_minute(&self, minute: u8) -> Self {
        DateTime {
            inner: self.inner.utc_offset_minute(minute),
        }
    }

    fn __repr__(&self) -> String {
        format!("DateTime({:?})", self.inner)
    }
}

impl DateTime {
    pub fn into_inner(self) -> krilla::metadata::DateTime {
        self.inner
    }
}

// ==================== Enums ====================

/// The main text direction of the document.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MetadataTextDirection {
    LeftToRight,
    RightToLeft,
}

impl MetadataTextDirection {
    pub fn into_inner(self) -> krilla::metadata::TextDirection {
        match self {
            MetadataTextDirection::LeftToRight => krilla::metadata::TextDirection::LeftToRight,
            MetadataTextDirection::RightToLeft => krilla::metadata::TextDirection::RightToLeft,
        }
    }
}

/// How the PDF viewer should lay out the pages.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PageLayout {
    /// Only a single page at a time.
    SinglePage,
    /// A single, continuously scrolling column of pages.
    OneColumn,
    /// Two continuously scrolling columns of pages, with odd-numbered pages on the left.
    TwoColumnLeft,
    /// Two continuously scrolling columns of pages, with odd-numbered pages on the right.
    TwoColumnRight,
    /// Only two pages visible at a time, with odd-numbered pages on the left (PDF 1.5+).
    TwoPageLeft,
    /// Only two pages visible at a time, with odd-numbered pages on the right (PDF 1.5+).
    TwoPageRight,
}

impl PageLayout {
    pub fn into_inner(self) -> krilla::metadata::PageLayout {
        match self {
            PageLayout::SinglePage => krilla::metadata::PageLayout::SinglePage,
            PageLayout::OneColumn => krilla::metadata::PageLayout::OneColumn,
            PageLayout::TwoColumnLeft => krilla::metadata::PageLayout::TwoColumnLeft,
            PageLayout::TwoColumnRight => krilla::metadata::PageLayout::TwoColumnRight,
            PageLayout::TwoPageLeft => krilla::metadata::PageLayout::TwoPageLeft,
            PageLayout::TwoPageRight => krilla::metadata::PageLayout::TwoPageRight,
        }
    }
}

/// How an embedded file relates to the PDF document.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AssociationKind {
    /// The PDF was created from this source file.
    Source,
    /// This file was used to derive a visual presentation in the PDF.
    Data,
    /// An alternative representation of this document.
    Alternative,
    /// Additional resources for this document.
    Supplement,
    /// No clear relationship or unknown.
    Unspecified,
}

impl AssociationKind {
    pub fn into_inner(self) -> krilla::embed::AssociationKind {
        match self {
            AssociationKind::Source => krilla::embed::AssociationKind::Source,
            AssociationKind::Data => krilla::embed::AssociationKind::Data,
            AssociationKind::Alternative => krilla::embed::AssociationKind::Alternative,
            AssociationKind::Supplement => krilla::embed::AssociationKind::Supplement,
            AssociationKind::Unspecified => krilla::embed::AssociationKind::Unspecified,
        }
    }
}

// ==================== Metadata ====================

/// Metadata for a PDF document.
#[pyclass]
#[derive(Clone)]
pub struct Metadata {
    pub(crate) inner: krilla::metadata::Metadata,
}

#[pymethods]
impl Metadata {
    /// Create new empty metadata.
    #[new]
    fn new() -> Self {
        Metadata {
            inner: krilla::metadata::Metadata::new(),
        }
    }

    /// Set the title of the document.
    fn title(&self, title: String) -> Self {
        Metadata {
            inner: self.inner.clone().title(title),
        }
    }

    /// Set the description of the document.
    ///
    /// This should be a short, human-readable abstract or summary.
    fn description(&self, description: String) -> Self {
        Metadata {
            inner: self.inner.clone().description(description),
        }
    }

    /// Set the keywords that describe the document.
    fn keywords(&self, keywords: Vec<String>) -> Self {
        Metadata {
            inner: self.inner.clone().keywords(keywords),
        }
    }

    /// Set the main language of the document as an RFC 3066 language tag.
    ///
    /// Required for some export modes like PDF/A-3a.
    fn language(&self, language: String) -> Self {
        Metadata {
            inner: self.inner.clone().language(language),
        }
    }

    /// Set the creator tool of the document.
    fn creator(&self, creator: String) -> Self {
        Metadata {
            inner: self.inner.clone().creator(creator),
        }
    }

    /// Set the producer tool of the document.
    fn producer(&self, producer: String) -> Self {
        Metadata {
            inner: self.inner.clone().producer(producer),
        }
    }

    /// Set the authors of the document.
    fn authors(&self, authors: Vec<String>) -> Self {
        Metadata {
            inner: self.inner.clone().authors(authors),
        }
    }

    /// Set the creation date of the document.
    fn creation_date(&self, creation_date: DateTime) -> Self {
        Metadata {
            inner: self.inner.clone().creation_date(creation_date.into_inner()),
        }
    }

    /// Set a document ID for identifying different versions of the same document.
    fn document_id(&self, document_id: String) -> Self {
        Metadata {
            inner: self.inner.clone().document_id(document_id),
        }
    }

    /// Set the main text direction of the document.
    fn text_direction(&self, text_direction: MetadataTextDirection) -> Self {
        Metadata {
            inner: self
                .inner
                .clone()
                .text_direction(text_direction.into_inner()),
        }
    }

    /// Set how the viewer should lay out the pages.
    fn page_layout(&self, page_layout: PageLayout) -> Self {
        Metadata {
            inner: self.inner.clone().page_layout(page_layout.into_inner()),
        }
    }

    fn __repr__(&self) -> String {
        "Metadata(...)".to_string()
    }
}

impl Metadata {
    pub fn into_inner(self) -> krilla::metadata::Metadata {
        self.inner
    }
}

// ==================== Outline ====================

/// A destination pointing to a specific location on a specific page.
#[pyclass]
#[derive(Clone)]
pub struct XyzDestination {
    pub(crate) inner: krilla::destination::XyzDestination,
}

#[pymethods]
impl XyzDestination {
    /// Create a new XYZ destination.
    ///
    /// `page_index` is the 0-based index of the target page.
    /// `point` is the specific location on that page.
    #[new]
    fn new(page_index: usize, point: &Point) -> Self {
        XyzDestination {
            inner: krilla::destination::XyzDestination::new(page_index, point.into_inner()),
        }
    }

    fn __repr__(&self) -> String {
        "XyzDestination(...)".to_string()
    }
}

impl XyzDestination {
    pub fn into_inner(self) -> krilla::destination::XyzDestination {
        self.inner
    }
}

/// An outline node representing an entry in the document's navigation tree.
#[pyclass]
#[derive(Clone)]
pub struct OutlineNode {
    pub(crate) inner: krilla::outline::OutlineNode,
}

#[pymethods]
impl OutlineNode {
    /// Create a new outline node.
    ///
    /// `text` is the string displayed in the outline tree.
    /// `destination` is where to navigate when clicking this entry.
    #[new]
    fn new(text: String, destination: XyzDestination) -> Self {
        OutlineNode {
            inner: krilla::outline::OutlineNode::new(text, destination.into_inner()),
        }
    }

    /// Add a child node to this outline node.
    fn push_child(&mut self, node: &OutlineNode) {
        self.inner.push_child(node.inner.clone());
    }

    fn __repr__(&self) -> String {
        "OutlineNode(...)".to_string()
    }
}

/// An outline (navigation tree) for the PDF document.
#[pyclass]
#[derive(Clone)]
pub struct Outline {
    pub(crate) inner: krilla::outline::Outline,
}

#[pymethods]
impl Outline {
    /// Create a new empty outline.
    #[new]
    fn new() -> Self {
        Outline {
            inner: krilla::outline::Outline::new(),
        }
    }

    /// Push a top-level child to the outline.
    fn push_child(&mut self, node: &OutlineNode) {
        self.inner.push_child(node.inner.clone());
    }

    fn __repr__(&self) -> String {
        "Outline(...)".to_string()
    }
}

impl Outline {
    pub fn into_inner(self) -> KrillaOutline {
        self.inner
    }
}

// ==================== Embedded Files ====================

/// A MIME type for an embedded file.
#[pyclass]
#[derive(Clone)]
pub struct MimeType {
    pub(crate) inner: KrillaMimeType,
}

#[pymethods]
impl MimeType {
    /// Create a new MIME type.
    ///
    /// Returns None if the MIME type string is invalid.
    #[new]
    fn new(mime_type: &str) -> PyResult<Self> {
        KrillaMimeType::new(mime_type)
            .map(|inner| MimeType { inner })
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid MIME type: {}", mime_type))
            })
    }

    fn __repr__(&self) -> String {
        "MimeType(...)".to_string()
    }
}

impl MimeType {
    pub fn into_inner(self) -> KrillaMimeType {
        self.inner
    }
}

/// An embedded file attachment for the PDF document.
#[pyclass]
#[derive(Clone)]
pub struct EmbeddedFile {
    pub(crate) inner: KrillaEmbeddedFile,
}

#[pymethods]
impl EmbeddedFile {
    /// Create a new embedded file.
    ///
    /// `path` is the name/path of the embedded file.
    /// `data` is the raw file content as bytes.
    #[new]
    #[pyo3(signature = (path, data, mime_type=None, description=None, association_kind=AssociationKind::Unspecified, modification_date=None, compress=None))]
    fn new(
        path: String,
        data: Vec<u8>,
        mime_type: Option<MimeType>,
        description: Option<String>,
        association_kind: AssociationKind,
        modification_date: Option<DateTime>,
        compress: Option<bool>,
    ) -> Self {
        EmbeddedFile {
            inner: KrillaEmbeddedFile {
                path,
                mime_type: mime_type.map(|m| m.into_inner()),
                description,
                association_kind: association_kind.into_inner(),
                data: data.into(),
                modification_date: modification_date.map(|d| d.into_inner()),
                compress,
                location: None,
            },
        }
    }

    fn __repr__(&self) -> String {
        format!("EmbeddedFile(path={:?})", self.inner.path)
    }
}

impl EmbeddedFile {
    pub fn into_inner(self) -> KrillaEmbeddedFile {
        self.inner
    }
}
