//! Configuration types for krilla Python bindings.

use pyo3::prelude::*;

/// PDF version.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PdfVersion {
    /// PDF 1.4
    Pdf14,
    /// PDF 1.5
    Pdf15,
    /// PDF 1.6
    Pdf16,
    /// PDF 1.7
    Pdf17,
    /// PDF 2.0
    Pdf20,
}

#[pymethods]
impl PdfVersion {
    /// Get the version string (e.g., "1.4", "2.0").
    fn as_str(&self) -> &'static str {
        self.into_inner().as_str()
    }

    fn __repr__(&self) -> String {
        format!("PdfVersion.{}", self.as_str().replace('.', "_"))
    }
}

impl PdfVersion {
    pub fn into_inner(self) -> krilla::configure::PdfVersion {
        match self {
            PdfVersion::Pdf14 => krilla::configure::PdfVersion::Pdf14,
            PdfVersion::Pdf15 => krilla::configure::PdfVersion::Pdf15,
            PdfVersion::Pdf16 => krilla::configure::PdfVersion::Pdf16,
            PdfVersion::Pdf17 => krilla::configure::PdfVersion::Pdf17,
            PdfVersion::Pdf20 => krilla::configure::PdfVersion::Pdf20,
        }
    }

    pub fn from_inner(inner: krilla::configure::PdfVersion) -> Self {
        match inner {
            krilla::configure::PdfVersion::Pdf14 => PdfVersion::Pdf14,
            krilla::configure::PdfVersion::Pdf15 => PdfVersion::Pdf15,
            krilla::configure::PdfVersion::Pdf16 => PdfVersion::Pdf16,
            krilla::configure::PdfVersion::Pdf17 => PdfVersion::Pdf17,
            krilla::configure::PdfVersion::Pdf20 => PdfVersion::Pdf20,
        }
    }
}

/// PDF validation standard.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Validator {
    /// No validation.
    None,
    /// PDF/A-1a (accessible).
    A1A,
    /// PDF/A-1b (basic).
    A1B,
    /// PDF/A-2a (accessible).
    A2A,
    /// PDF/A-2b (basic).
    A2B,
    /// PDF/A-2u (Unicode).
    A2U,
    /// PDF/A-3a (accessible).
    A3A,
    /// PDF/A-3b (basic).
    A3B,
    /// PDF/A-3u (Unicode).
    A3U,
    /// PDF/A-4 (basic).
    A4,
    /// PDF/A-4f (file).
    A4F,
    /// PDF/A-4e (engineering).
    A4E,
    /// PDF/UA-1 (universal accessibility).
    UA1,
}

#[pymethods]
impl Validator {
    /// Check if this validator is compatible with a PDF version.
    fn compatible_with_version(&self, version: PdfVersion) -> bool {
        self.into_inner().compatible_with_version(version.into_inner())
    }

    /// Get the recommended PDF version for this validator.
    fn recommended_version(&self) -> PdfVersion {
        PdfVersion::from_inner(self.into_inner().recommended_version())
    }

    fn __repr__(&self) -> String {
        format!("Validator.{:?}", self)
    }
}

impl Validator {
    pub fn into_inner(self) -> krilla::configure::Validator {
        match self {
            Validator::None => krilla::configure::Validator::None,
            Validator::A1A => krilla::configure::Validator::A1_A,
            Validator::A1B => krilla::configure::Validator::A1_B,
            Validator::A2A => krilla::configure::Validator::A2_A,
            Validator::A2B => krilla::configure::Validator::A2_B,
            Validator::A2U => krilla::configure::Validator::A2_U,
            Validator::A3A => krilla::configure::Validator::A3_A,
            Validator::A3B => krilla::configure::Validator::A3_B,
            Validator::A3U => krilla::configure::Validator::A3_U,
            Validator::A4 => krilla::configure::Validator::A4,
            Validator::A4F => krilla::configure::Validator::A4F,
            Validator::A4E => krilla::configure::Validator::A4E,
            Validator::UA1 => krilla::configure::Validator::UA1,
        }
    }

    pub fn from_inner(inner: krilla::configure::Validator) -> Self {
        match inner {
            krilla::configure::Validator::None => Validator::None,
            krilla::configure::Validator::A1_A => Validator::A1A,
            krilla::configure::Validator::A1_B => Validator::A1B,
            krilla::configure::Validator::A2_A => Validator::A2A,
            krilla::configure::Validator::A2_B => Validator::A2B,
            krilla::configure::Validator::A2_U => Validator::A2U,
            krilla::configure::Validator::A3_A => Validator::A3A,
            krilla::configure::Validator::A3_B => Validator::A3B,
            krilla::configure::Validator::A3_U => Validator::A3U,
            krilla::configure::Validator::A4 => Validator::A4,
            krilla::configure::Validator::A4F => Validator::A4F,
            krilla::configure::Validator::A4E => Validator::A4E,
            krilla::configure::Validator::UA1 => Validator::UA1,
        }
    }
}

/// PDF generation configuration.
#[pyclass]
#[derive(Clone)]
pub struct Configuration {
    inner: krilla::configure::Configuration,
}

#[pymethods]
impl Configuration {
    /// Create a new configuration with defaults.
    #[new]
    fn new() -> Self {
        Configuration {
            inner: krilla::configure::Configuration::new(),
        }
    }

    /// Create a configuration with a specific validator.
    #[staticmethod]
    fn with_validator(validator: Validator) -> Self {
        Configuration {
            inner: krilla::configure::Configuration::new_with_validator(validator.into_inner()),
        }
    }

    /// Create a configuration with a specific PDF version.
    #[staticmethod]
    fn with_version(version: PdfVersion) -> Self {
        Configuration {
            inner: krilla::configure::Configuration::new_with_version(version.into_inner()),
        }
    }

    /// Create a configuration with both validator and version.
    ///
    /// Returns None if the validator is incompatible with the version.
    #[staticmethod]
    fn with_validator_and_version(validator: Validator, version: PdfVersion) -> Option<Self> {
        krilla::configure::Configuration::new_with(validator.into_inner(), version.into_inner())
            .map(|c| Configuration { inner: c })
    }

    /// Get the validator.
    #[getter]
    fn validator(&self) -> Validator {
        Validator::from_inner(self.inner.validator())
    }

    /// Get the PDF version.
    #[getter]
    fn version(&self) -> PdfVersion {
        PdfVersion::from_inner(self.inner.version())
    }

    fn __repr__(&self) -> String {
        format!(
            "Configuration(validator={:?}, version={})",
            self.validator(),
            self.version().as_str()
        )
    }
}

impl Configuration {
    pub fn into_inner(self) -> krilla::configure::Configuration {
        self.inner
    }
}

/// Settings for PDF serialization.
#[pyclass]
#[derive(Clone)]
pub struct SerializeSettings {
    inner: krilla::SerializeSettings,
}

#[pymethods]
impl SerializeSettings {
    /// Create new serialize settings with defaults.
    #[new]
    fn new() -> Self {
        SerializeSettings {
            inner: krilla::SerializeSettings::default(),
        }
    }

    /// Create serialize settings with a specific configuration.
    #[staticmethod]
    fn with_configuration(configuration: Configuration) -> Self {
        SerializeSettings {
            inner: krilla::SerializeSettings {
                configuration: configuration.into_inner(),
                ..Default::default()
            },
        }
    }

    fn __repr__(&self) -> String {
        "SerializeSettings(...)".to_string()
    }
}

impl SerializeSettings {
    pub fn into_inner(self) -> krilla::SerializeSettings {
        self.inner
    }
}
