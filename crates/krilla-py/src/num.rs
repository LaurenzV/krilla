//! Numeric types for krilla Python bindings.

use pyo3::prelude::*;

/// A floating-point number normalized to the range [0.0, 1.0].
///
/// Used for opacity values, gradient stops, and other normalized quantities.
#[pyclass(frozen)]
#[derive(Clone, Copy)]
pub struct NormalizedF32 {
    pub(crate) inner: krilla::num::NormalizedF32,
}

#[pymethods]
impl NormalizedF32 {
    /// Create a new normalized float.
    ///
    /// Raises ValueError if the value is not in the range [0.0, 1.0].
    #[new]
    fn new(value: f32) -> PyResult<Self> {
        krilla::num::NormalizedF32::new(value)
            .map(|n| NormalizedF32 { inner: n })
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "Value {} is not in the range [0.0, 1.0]",
                    value
                ))
            })
    }

    /// Create a normalized float representing 0.0.
    #[staticmethod]
    pub fn zero() -> Self {
        NormalizedF32 {
            inner: krilla::num::NormalizedF32::ZERO,
        }
    }

    /// Create a normalized float representing 1.0.
    #[staticmethod]
    pub fn one() -> Self {
        NormalizedF32 {
            inner: krilla::num::NormalizedF32::ONE,
        }
    }

    /// Get the underlying float value.
    pub fn get(&self) -> f32 {
        self.inner.get()
    }

    fn __repr__(&self) -> String {
        format!("NormalizedF32({})", self.inner.get())
    }

    fn __eq__(&self, other: &NormalizedF32) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }

    fn __float__(&self) -> f32 {
        self.inner.get()
    }
}

impl NormalizedF32 {
    pub const ZERO: NormalizedF32 = NormalizedF32 {
        inner: krilla::num::NormalizedF32::ZERO,
    };

    pub const ONE: NormalizedF32 = NormalizedF32 {
        inner: krilla::num::NormalizedF32::ONE,
    };

    pub fn into_inner(self) -> krilla::num::NormalizedF32 {
        self.inner
    }

    pub fn from_inner(inner: krilla::num::NormalizedF32) -> Self {
        NormalizedF32 { inner }
    }
}
