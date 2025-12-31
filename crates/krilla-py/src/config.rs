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
        self.into_inner()
            .compatible_with_version(version.into_inner())
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
    /// Create a new configuration.
    ///
    /// Args:
    ///     validator: The validation standard to use (e.g., Validator.A2B for PDF/A-2b).
    ///                If not specified, no validation is performed.
    ///     version: The PDF version to target. If not specified, uses the recommended
    ///              version for the validator, or PDF 1.7 if no validator is set.
    ///
    /// Raises:
    ///     ValueError: If the validator is incompatible with the specified version.
    #[new]
    #[pyo3(signature = (validator=None, version=None))]
    fn new(validator: Option<Validator>, version: Option<PdfVersion>) -> PyResult<Self> {
        let inner = match (validator, version) {
            (None, None) => krilla::configure::Configuration::new(),
            (Some(v), None) => krilla::configure::Configuration::new_with_validator(v.into_inner()),
            (None, Some(v)) => krilla::configure::Configuration::new_with_version(v.into_inner()),
            (Some(val), Some(ver)) => {
                krilla::configure::Configuration::new_with(val.into_inner(), ver.into_inner())
                    .ok_or_else(|| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "Validator {:?} is incompatible with PDF version {}",
                            val,
                            ver.as_str()
                        ))
                    })?
            }
        };
        Ok(Configuration { inner })
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
    /// Create new serialize settings.
    ///
    /// Args:
    ///     configuration: The PDF configuration (validator and version).
    ///     compress: Whether to compress content streams. Defaults to True.
    ///               Leads to significantly smaller files but longer running times.
    ///     ascii_compatible: Whether the PDF should be ASCII-compatible. Defaults to False.
    ///                       Note: This is best-effort only; some content may still be binary.
    ///     xmp_metadata: Whether to include XMP metadata. Defaults to False.
    ///                   May be overridden by certain validators (e.g., PDF/A requires it).
    ///     no_device_cs: Whether to use device-independent colors. Defaults to False.
    ///                   May be overridden by certain validators (e.g., PDF/A requires it).
    ///     enable_tagging: Whether to enable tagged PDF creation. Defaults to False.
    ///                     May be overridden by certain validators (e.g., PDF/UA requires it).
    #[new]
    #[pyo3(signature = (configuration=None, compress=true, ascii_compatible=false, xmp_metadata=false, no_device_cs=false, enable_tagging=false))]
    fn new(
        configuration: Option<Configuration>,
        compress: bool,
        ascii_compatible: bool,
        xmp_metadata: bool,
        no_device_cs: bool,
        enable_tagging: bool,
    ) -> Self {
        SerializeSettings {
            inner: krilla::SerializeSettings {
                configuration: configuration.map(|c| c.into_inner()).unwrap_or_default(),
                compress_content_streams: compress,
                ascii_compatible,
                xmp_metadata,
                no_device_cs,
                cmyk_profile: None,
                enable_tagging,
                render_svg_glyph_fn: |_, _, _, _, _| None,
            },
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "SerializeSettings(compress={}, ascii_compatible={}, xmp_metadata={}, no_device_cs={}, enable_tagging={})",
            self.inner.compress_content_streams,
            self.inner.ascii_compatible,
            self.inner.xmp_metadata,
            self.inner.no_device_cs,
            self.inner.enable_tagging
        )
    }
}

impl SerializeSettings {
    pub fn into_inner(self) -> krilla::SerializeSettings {
        self.inner
    }
}
