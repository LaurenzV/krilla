//! Configuring PDF version and export mode.

pub mod validate;
mod version;

pub use validate::{Archival, UniversalAccessibility, ValidationError, Validators};
pub(crate) use validate::Validator;
pub use version::PdfVersion;

/// A configuration of validator and PDF version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configuration {
    validators: Validators,
    version: PdfVersion,
}

impl Default for Configuration {
    fn default() -> Self {
        Self::new()
    }
}

impl Configuration {
    /// Create a new configuration from a set of validators and a PDF version.
    ///
    /// Returns `None` if the configuration is invalid.
    pub fn new_with(validators: impl Into<Validators>, version: PdfVersion) -> Option<Self> {
        let validators = validators.into();

        for v in validators.iter() {
            if !v.compatible_with_version(version) {
                return None;
            }
        }

        Some(Self {
            validators,
            version,
        })
    }

    /// Create a new configuration from a set of validators. 
    /// An appropriate PDF version will be set automatically.
    pub fn new_with_validators(validators: impl Into<Validators>) -> Self {
        let validators = validators.into();
        let version = validators.recommended_version();
        // Guaranteed to be valid.
        Self::new_with(validators, version).unwrap()
    }

    /// Create a new configuration from a PDF version and no validator.
    pub fn new_with_version(version: PdfVersion) -> Self {
        Self {
            validators: Validators::dummy(),
            version,
        }
    }

    /// Create a new configuration without any validator.
    pub fn new() -> Self {
        let validators = Validators::dummy();
        let version = validators.recommended_version();
        
        Self {
            validators,
            version,
        }
    }

    /// Return the validators of the configuration.
    pub fn validators(&self) -> &Validators {
        &self.validators
    }

    /// Return the PDF version of the configuration.
    pub fn version(&self) -> PdfVersion {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use crate::configure::{
        Archival, Configuration, PdfVersion, UniversalAccessibility, Validators,
    };

    #[test]
    fn invalid_combination_1() {
        assert_eq!(
            Configuration::new_with(Archival::A1_B, PdfVersion::Pdf17),
            None
        );
    }

    #[test]
    fn invalid_combination_2() {
        // A-4 requires at least PDF 2.0; UA1 allows at most PDF 1.7.
        assert!(Validators::with(Some(Archival::A4), Some(UniversalAccessibility::UA1)).is_none());
    }

    #[test]
    fn invalid_combination_3() {
        // A-1b is only valid up to PDF 1.4; PDF 1.7 is out of range.
        assert!(
            Configuration::new_with(
                Validators::with(Some(Archival::A1_B), Some(UniversalAccessibility::UA1)).unwrap(),
                PdfVersion::Pdf17
            )
            .is_none()
        );
    }

    #[test]
    fn multi_validator_pdf_a3b_pdf_ua1() {
        let _ = Configuration::new_with_validators(
            Validators::with(Some(Archival::A3_B), Some(UniversalAccessibility::UA1)).unwrap(),
        );
    }

    #[test]
    fn multi_validator_pdfa2a_pdfua1() {
        let _ = Configuration::new_with_validators(
            Validators::with(Some(Archival::A2_A), Some(UniversalAccessibility::UA1)).unwrap(),
        );
    }

    #[test]
    fn empty_validators() {
        let config = Configuration::new_with_validators(Validators::dummy());
        
        assert!(config.validators().is_empty());
        assert_eq!(config.version(), PdfVersion::Pdf17);
    }

    #[test]
    fn default_config() {
        let config = Configuration::new_with_validators(Validators::dummy());
        let default = Configuration::new();
        assert_eq!(config.version(), default.version());
        assert_eq!(config.validators(), default.validators());
    }
}
