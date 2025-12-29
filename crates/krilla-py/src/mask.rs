//! Mask types for krilla Python bindings.

use pyo3::prelude::*;
use std::sync::Arc;

use crate::enums::MaskType;
use crate::stream::Stream;

/// A mask for clipping or transparency effects.
///
/// Masks can only be used with the document that created them.
#[pyclass]
#[derive(Clone)]
pub struct Mask {
    pub(crate) inner: Arc<krilla::mask::Mask>,
    pub(crate) doc_id: usize,
}

#[pymethods]
impl Mask {
    /// Create a new mask from a stream.
    ///
    /// Args:
    ///     stream: The stream containing the mask content
    ///     mask_type: The type of mask (Luminosity or Alpha)
    #[new]
    fn new(stream: Stream, mask_type: MaskType) -> Self {
        Mask {
            inner: Arc::new(krilla::mask::Mask::new(
                stream.into_inner(),
                mask_type.into_inner(),
            )),
            doc_id: 0, // Will be set properly when used with a document
        }
    }

    fn __repr__(&self) -> String {
        "Mask(...)".to_string()
    }
}

impl Mask {
    pub fn with_doc_id(mut self, doc_id: usize) -> Self {
        self.doc_id = doc_id;
        self
    }

    pub fn validate_document(&self, doc_id: usize) -> PyResult<()> {
        if self.doc_id != 0 && self.doc_id != doc_id {
            Err(pyo3::exceptions::PyValueError::new_err(
                "Mask can only be used with the document that created it",
            ))
        } else {
            Ok(())
        }
    }

    pub fn inner_ref(&self) -> &krilla::mask::Mask {
        &self.inner
    }
}
