//! Configuring PDF version and export mode.

pub mod validate;
pub mod version;

pub use validate::{ValidationError, Validator};
pub use version::PdfVersion;

/// A configuration of validator and PDF version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configuration {
    validator: Validator,
    version: PdfVersion,
}

impl Default for Configuration {
    fn default() -> Self {
        Self::new()
    }
}

impl Configuration {
    /// Create a new configuration from a validator and a PDF version.
    ///
    /// Returns `None` if the configuration is invalid.
    pub fn new_with(validator: Validator, version: PdfVersion) -> Option<Self> {
        if validator.compatible_with_version(version) {
            Some(Self { validator, version })
        } else {
            None
        }
    }

    /// Create a new configuration from a validator. An appropriate PDF version will be set
    /// automatically.
    pub fn new_with_validator(validator: Validator) -> Self {
        Self::new_with(validator, validator.recommended_version()).unwrap()
    }

    /// Create a new configuration from a PDF version and no validator.
    pub fn new_with_version(version: PdfVersion) -> Self {
        Self::new_with(Validator::None, version).unwrap()
    }

    /// Create a new configuration without any validator.
    pub fn new() -> Self {
        Self::new_with_validator(Validator::None)
    }

    /// Return the validator of the configuration.
    pub fn validator(&self) -> Validator {
        self.validator
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
            Configuration::new_with(Validator::A1_B, PdfVersion::Pdf17),
            None
        );
    }
}
