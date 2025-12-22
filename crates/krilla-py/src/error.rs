//! Python exception types for krilla errors.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

// Create Python exception types
create_exception!(_krilla, KrillaError, PyException, "Base exception for krilla errors.");
create_exception!(_krilla, FontError, KrillaError, "Exception raised for font-related errors.");
create_exception!(
    _krilla,
    ValidationError,
    KrillaError,
    "Exception raised for PDF validation errors."
);
create_exception!(
    _krilla,
    ImageError,
    KrillaError,
    "Exception raised for image processing errors."
);

/// Convert a krilla::KrillaError to a Python exception.
pub fn to_py_err(err: krilla::error::KrillaError) -> PyErr {
    use krilla::error::KrillaError as KE;

    match err {
        KE::Font(_, msg) => FontError::new_err(msg),
        KE::Validation(errors) => {
            let messages: Vec<String> = errors.iter().map(|e| format!("{:?}", e)).collect();
            ValidationError::new_err(messages.join("; "))
        }
        KE::DuplicateTagId(id, _) => KrillaError::new_err(format!("Duplicate tag ID: {:?}", id)),
        KE::UnknownTagId(id, _) => KrillaError::new_err(format!("Unknown tag ID: {:?}", id)),
        #[cfg(feature = "raster-images")]
        KE::Image(_, _, msg) => ImageError::new_err(msg),
        #[cfg(feature = "raster-images")]
        KE::SixteenBitImage(_, _) => {
            ImageError::new_err("16-bit images require PDF version 1.5 or higher")
        }
        #[cfg(feature = "pdf")]
        KE::Pdf(_, err, _) => KrillaError::new_err(format!("PDF embedding error: {:?}", err)),
    }
}
