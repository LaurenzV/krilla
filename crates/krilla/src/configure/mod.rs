//! Configuring PDF version and export mode.

pub mod validate;
mod version;

pub use validate::{ValidationError, Validator};
pub use version::PdfVersion;

/// A configuration of validator and PDF version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configuration {
    validators: Vec<Validator>,
    version: PdfVersion,
}

impl Default for Configuration {
    fn default() -> Self {
        Self::new()
    }
}

impl Configuration {
    /// Create a new configuration from validators and a PDF version.
    ///
    /// Returns `None` if the configuration is invalid.
    pub fn new_with(
        validators: impl IntoIterator<Item = Validator>,
        version: PdfVersion,
    ) -> Option<Self> {
        let validators = Self::collect_compatible_validators(validators)?;

        for v in &validators {
            if !v.compatible_with_version(version) {
                return None;
            }
        }

        Some(Self {
            validators,
            version,
        })
    }

    /// Create a new configuration from a validator. An appropriate PDF version
    /// will be set automatically.
    pub fn new_with_validator(validator: Validator) -> Self {
        Self::new_with(std::iter::once(validator), validator.recommended_version()).unwrap()
    }

    /// Create a new configuration from multiple validators. An appropriate PDF version
    /// will be set automatically.
    ///
    /// Returns `None` if the validators are not compatible with each other.
    pub fn new_with_validators(validators: impl IntoIterator<Item = Validator>) -> Option<Self> {
        let validators = Self::collect_compatible_validators(validators)?;
        let version = validators
            .iter()
            .map(|v| v.maximum_pdf_version().unwrap_or(PdfVersion::Pdf20))
            .max()
            .unwrap_or(PdfVersion::Pdf17);

        Self::new_with(validators, version)
    }

    fn collect_compatible_validators(
        validators: impl IntoIterator<Item = Validator>,
    ) -> Option<Vec<Validator>> {
        let validators: Vec<Validator> = validators.into_iter().collect();
        for i in 0..validators.len() {
            for j in (i + 1)..validators.len() {
                if !validators[i].mutually_compatible_with(validators[j]) {
                    return None;
                }
            }
        }

        Some(validators)
    }

    /// Create a new configuration from a PDF version and no validator.
    pub fn new_with_version(version: PdfVersion) -> Self {
        Self {
            validators: vec![],
            version,
        }
    }

    /// Create a new configuration without any validator.
    pub fn new() -> Self {
        Self {
            validators: vec![],
            version: PdfVersion::Pdf17,
        }
    }

    /// Return the validators of the configuration.
    pub fn validators(&self) -> &[Validator] {
        &self.validators
    }

    /// Return the PDF version of the configuration.
    pub fn version(&self) -> PdfVersion {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use crate::configure::{Configuration, PdfVersion, Validator};

    #[test]
    fn invalid_combination_1() {
        assert_eq!(
            Configuration::new_with(std::iter::once(Validator::A1_B), PdfVersion::Pdf17),
            None
        );
    }

    #[test]
    fn invalid_combination_2() {
        // Same standard family
        assert!(Configuration::new_with_validators([Validator::A2_B, Validator::A3_B]).is_none());
    }

    #[test]
    fn invalid_combination_3() {
        // A-4 requires at least PDF 2.0; UA1 allows at most PDF 1.7.
        assert!(Configuration::new_with_validators([Validator::A4, Validator::UA1]).is_none());
    }

    #[test]
    fn invalid_combination_4() {
        // Duplicate validator
        assert!(Configuration::new_with_validators([Validator::A3_B, Validator::A3_B]).is_none());
    }

    #[test]
    fn invalid_combination_5() {
        // A-1b is only valid up to PDF 1.4; PDF 1.7 is out of range.
        assert!(
            Configuration::new_with([Validator::A1_B, Validator::UA1], PdfVersion::Pdf17).is_none()
        );
    }

    #[test]
    fn multi_validator_pdf_a3b_pdf_ua1() {
        let config = Configuration::new_with_validators([Validator::A3_B, Validator::UA1]).unwrap();
        assert_eq!(config.validators(), &[Validator::A3_B, Validator::UA1]);
        assert_eq!(config.version(), PdfVersion::Pdf17);
    }

    #[test]
    fn multi_validator_pdfa2a_pdfua1() {
        let config = Configuration::new_with_validators([Validator::A2_A, Validator::UA1]);
        assert!(config.is_some());
    }

    #[test]
    fn empty_validators() {
        let config = Configuration::new_with_validators([]).unwrap();
        assert!(config.validators().is_empty());
        assert_eq!(config.version(), PdfVersion::Pdf17);
    }

    #[test]
    fn default_config() {
        let config = Configuration::new_with_validators([]).unwrap();
        let default = Configuration::new();
        assert_eq!(config.version(), default.version());
        assert_eq!(config.validators(), default.validators());
    }
}
