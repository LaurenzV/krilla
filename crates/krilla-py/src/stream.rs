//! Stream types for krilla Python bindings.

use pyo3::prelude::*;

/// An encoded stream of drawing instructions.
///
/// Streams are created using StreamBuilder and can be used to create
/// masks, patterns, and graphics.
#[pyclass]
#[derive(Clone)]
pub struct Stream {
    pub(crate) inner: krilla::stream::Stream,
}

#[pymethods]
impl Stream {
    fn __repr__(&self) -> String {
        "Stream(...)".to_string()
    }
}

impl Stream {
    pub fn into_inner(self) -> krilla::stream::Stream {
        self.inner
    }

    pub fn from_inner(inner: krilla::stream::Stream) -> Self {
        Stream { inner }
    }
}

/// Builder for creating streams.
///
/// StreamBuilder is obtained from Surface.stream_builder() and provides
/// a sub-drawing context for creating masks, patterns, etc.
#[pyclass]
pub struct StreamBuilder {
    // We can't store the actual StreamBuilder because it borrows from Surface.
    // Instead, we'll need to handle this differently in the document module.
    // This is a placeholder that will be constructed properly in document.rs.
    _placeholder: (),
}

#[pymethods]
impl StreamBuilder {
    fn __repr__(&self) -> String {
        "StreamBuilder(...)".to_string()
    }
}
