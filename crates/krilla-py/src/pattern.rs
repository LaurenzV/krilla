//! Pattern types for krilla Python bindings.

use pyo3::prelude::*;

use crate::geometry::Transform;
use crate::stream::Stream;

/// A repeating pattern for fills.
///
/// Patterns can only be used with the document that created them.
#[pyclass]
#[derive(Clone)]
pub struct Pattern {
    pub(crate) inner: krilla::paint::Pattern,
    pub(crate) doc_id: usize,
}

#[pymethods]
impl Pattern {
    /// Create a new pattern from a stream.
    ///
    /// Args:
    ///     stream: The stream containing the pattern content
    ///     width: Pattern tile width
    ///     height: Pattern tile height
    ///     transform: Optional transformation for the pattern
    #[new]
    #[pyo3(signature = (stream, width, height, transform=None))]
    fn new(stream: Stream, width: f32, height: f32, transform: Option<Transform>) -> Self {
        Pattern {
            inner: krilla::paint::Pattern {
                stream: stream.into_inner(),
                transform: transform.map(|t| t.into_inner()).unwrap_or_default(),
                width,
                height,
            },
            doc_id: 0, // Will be set properly when used with a document
        }
    }

    /// The pattern tile width.
    #[getter]
    fn width(&self) -> f32 {
        self.inner.width
    }

    /// The pattern tile height.
    #[getter]
    fn height(&self) -> f32 {
        self.inner.height
    }

    fn __repr__(&self) -> String {
        format!("Pattern(width={}, height={})", self.inner.width, self.inner.height)
    }
}

impl Pattern {
    pub fn with_doc_id(mut self, doc_id: usize) -> Self {
        self.doc_id = doc_id;
        self
    }

    pub fn validate_document(&self, doc_id: usize) -> PyResult<()> {
        if self.doc_id != 0 && self.doc_id != doc_id {
            Err(pyo3::exceptions::PyValueError::new_err(
                "Pattern can only be used with the document that created it",
            ))
        } else {
            Ok(())
        }
    }

    pub fn into_inner(self) -> krilla::paint::Pattern {
        self.inner
    }
}
