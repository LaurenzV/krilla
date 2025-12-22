//! Accessibility and tagging types for krilla Python bindings.

use pyo3::prelude::*;

/// A location identifier for tracking render operations.
///
/// Used to associate render operations with a unique identifier,
/// which helps backtrack validation errors to specific content.
#[pyclass(frozen)]
#[derive(Clone, Copy)]
pub struct Location {
    pub(crate) inner: krilla::surface::Location,
}

#[pymethods]
impl Location {
    /// Create a new location from a positive integer.
    ///
    /// Raises ValueError if the value is zero.
    #[new]
    fn new(value: u64) -> PyResult<Self> {
        std::num::NonZeroU64::new(value)
            .map(|n| Location { inner: n })
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("Location value must be non-zero")
            })
    }

    /// Get the underlying integer value.
    fn get(&self) -> u64 {
        self.inner.get()
    }

    fn __repr__(&self) -> String {
        format!("Location({})", self.inner.get())
    }

    fn __eq__(&self, other: &Location) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        self.inner.get()
    }
}

impl Location {
    pub fn into_inner(self) -> krilla::surface::Location {
        self.inner
    }
}

/// Type of artifact in a PDF document.
///
/// Artifacts represent pieces of content that are not part of the logical
/// structure and should be excluded from accessibility tools.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArtifactType {
    /// The header of a page.
    Header,
    /// The footer of a page.
    Footer,
    /// Page artifacts (e.g., cut marks, color bars).
    Page,
    /// Any other type of artifact (e.g., table strokes).
    Other,
}

impl ArtifactType {
    pub fn into_inner(self) -> krilla::tagging::ArtifactType {
        match self {
            ArtifactType::Header => krilla::tagging::ArtifactType::Header,
            ArtifactType::Footer => krilla::tagging::ArtifactType::Footer,
            ArtifactType::Page => krilla::tagging::ArtifactType::Page,
            ArtifactType::Other => krilla::tagging::ArtifactType::Other,
        }
    }
}

/// A span tag with text properties for accessibility.
///
/// Spans should not be too long - at most a single line of text.
#[pyclass]
#[derive(Clone)]
pub struct SpanTag {
    /// The language of the text (e.g., "en-US").
    #[pyo3(get, set)]
    pub lang: Option<String>,
    /// Alternate text describing the content.
    #[pyo3(get, set)]
    pub alt_text: Option<String>,
    /// Expanded form of an abbreviation.
    #[pyo3(get, set)]
    pub expanded: Option<String>,
    /// The actual text if different from displayed text.
    #[pyo3(get, set)]
    pub actual_text: Option<String>,
}

#[pymethods]
impl SpanTag {
    /// Create a new span tag.
    #[new]
    #[pyo3(signature = (lang=None, alt_text=None, expanded=None, actual_text=None))]
    fn new(
        lang: Option<String>,
        alt_text: Option<String>,
        expanded: Option<String>,
        actual_text: Option<String>,
    ) -> Self {
        SpanTag {
            lang,
            alt_text,
            expanded,
            actual_text,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "SpanTag(lang={:?}, alt_text={:?}, expanded={:?}, actual_text={:?})",
            self.lang, self.alt_text, self.expanded, self.actual_text
        )
    }
}

/// A content tag for accessibility marking.
///
/// Content tags associate content with semantic meaning for accessibility.
#[pyclass]
#[derive(Clone)]
pub enum ContentTag {
    /// An artifact (content not part of logical structure).
    Artifact { artifact_type: ArtifactType },
    /// A text span with properties.
    Span { tag: SpanTag },
    /// Other content that doesn't fit into Span or Artifact.
    Other {},
}

#[pymethods]
impl ContentTag {
    /// Create an artifact content tag.
    #[staticmethod]
    fn artifact(artifact_type: ArtifactType) -> Self {
        ContentTag::Artifact { artifact_type }
    }

    /// Create a span content tag.
    #[staticmethod]
    fn span(tag: SpanTag) -> Self {
        ContentTag::Span { tag }
    }

    /// Create an "other" content tag.
    #[staticmethod]
    fn other() -> Self {
        ContentTag::Other {}
    }

    fn __repr__(&self) -> String {
        match self {
            ContentTag::Artifact { artifact_type } => {
                format!("ContentTag.Artifact({:?})", artifact_type)
            }
            ContentTag::Span { .. } => "ContentTag.Span(...)".to_string(),
            ContentTag::Other {} => "ContentTag.Other".to_string(),
        }
    }
}

impl ContentTag {
    pub fn to_inner(&self) -> krilla::tagging::ContentTag<'_> {
        match self {
            ContentTag::Artifact { artifact_type } => {
                krilla::tagging::ContentTag::Artifact(artifact_type.into_inner())
            }
            ContentTag::Span { tag } => {
                krilla::tagging::ContentTag::Span(krilla::tagging::SpanTag {
                    lang: tag.lang.as_deref(),
                    alt_text: tag.alt_text.as_deref(),
                    expanded: tag.expanded.as_deref(),
                    actual_text: tag.actual_text.as_deref(),
                })
            }
            ContentTag::Other {} => krilla::tagging::ContentTag::Other,
        }
    }
}

/// An identifier returned from start_tagged.
///
/// Used as a leaf node in a tag tree for PDF/UA compliance.
/// This is an opaque handle for tracking tagged content sections.
#[pyclass(frozen)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Identifier {
    /// Internal ID (0 means dummy/placeholder)
    id: u64,
}

#[pymethods]
impl Identifier {
    fn __repr__(&self) -> String {
        if self.id == 0 {
            "Identifier(dummy)".to_string()
        } else {
            format!("Identifier({})", self.id)
        }
    }

    fn __eq__(&self, other: &Identifier) -> bool {
        self.id == other.id
    }

    fn __hash__(&self) -> u64 {
        self.id
    }

    /// Check if this is a dummy identifier (e.g., for artifacts).
    fn is_dummy(&self) -> bool {
        self.id == 0
    }
}

impl Identifier {
    /// Create a dummy identifier (for artifacts or when tagging is disabled).
    pub fn dummy() -> Self {
        Identifier { id: 0 }
    }

    /// Create an identifier from a krilla Identifier.
    /// Since we can't inspect the inner value, we generate our own ID.
    pub fn from_inner(_inner: krilla::tagging::Identifier) -> Self {
        // We can't access the inner value, so generate a unique ID
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Identifier {
            id: COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        }
    }
}
