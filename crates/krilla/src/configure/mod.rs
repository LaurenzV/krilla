//! Configuring PDF version and export mode.

pub mod validate;
mod version;

pub use validate::{Accessibility, Archival, Prepress, ValidationError, Validator, Validators};
pub use version::PdfVersion;

use crate::configure::validate::ValidatorsBuilder;

/// A configuration of validator and PDF version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Configuration {
    validators: Validators,
    version: PdfVersion,
}

impl Configuration {
    /// Return the validators of the configuration.
    pub fn validators(self) -> Validators {
        self.validators
    }

    /// Return the PDF version of the configuration.
    pub fn version(self) -> PdfVersion {
        self.version
    }
}

/// A configuration of validator and PDF version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ConfigurationBuilder {
    validators: ValidatorsBuilder,
    version: Option<PdfVersion>,
}

impl ConfigurationBuilder {
    /// Create a new `ConfigurationBuilder` with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the PDF version, overwriting the current one if already set.
    pub fn with_version(mut self, version: PdfVersion) -> Self {
        self.version = Some(version);
        self
    }

    /// Set a validator, overwriting the current one if the same standard family is already set.
    pub fn set_validator(mut self, validator: Validator) -> Self {
        self.validators = self.validators.set_validator(validator);
        self
    }

    /// Set the PDF/A validator, overwriting the current one if already set.
    pub fn with_archival_validator(mut self, archival: Archival) -> Self {
        self.validators = self.validators.with_archival_validator(archival);
        self
    }

    /// Set the PDF/UA accessibility validator, overwriting the current one if already set.
    pub fn with_accessibility_validator(mut self, ua: Accessibility) -> Self {
        self.validators = self.validators.with_accessibility_validator(ua);
        self
    }

    /// Set the PDF/X prepress validator, overwriting the current one if already set.
    pub fn with_prepress_validator(mut self, prepress: Prepress) -> Self {
        self.validators = self.validators.with_prepress_validator(prepress);
        self
    }

    /// Build the [`Configuration`], returning an error if the validators and version are incompatible.
    pub fn finish(self) -> Result<Configuration, ConfigurationError> {
        let validators = self
            .validators
            .finish()
            .map_err(ConfigurationError::NoOverlappingValidatorsRange)?;

        if validators.has_incompatible_output_intents() {
            return Err(ConfigurationError::IncompatibleOutputIntents(validators));
        }

        let validator_range = validators.min().unwrap_or(PdfVersion::MIN)..=validators.max();
        match self.version {
            Some(version) if validator_range.contains(&version) => Ok(Configuration {
                validators,
                version,
            }),
            Some(version) => Err(ConfigurationError::VersionDoesNotMatchValidatorsRange(
                version, validators,
            )),
            None if !validators.is_empty() => Ok(Configuration {
                validators,
                version: *validator_range.end(),
            }),
            None => {
                let version = PdfVersion::default();
                debug_assert!(validator_range.contains(&version));
                Ok(Configuration {
                    validators,
                    version,
                })
            }
        }
    }
}

/// An error that occurred while building a [`Configuration`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigurationError {
    /// The selected validators have no overlapping valid PDF version range.
    NoOverlappingValidatorsRange(Validators),
    /// The explicitly set PDF version falls outside the range allowed by the validators.
    VersionDoesNotMatchValidatorsRange(PdfVersion, Validators),
    /// The selected validators have irreconcilable output-intent requirements.
    ///
    /// Every PDF/X level permits additional output intents with a different `S`
    /// key (ISO 15930-4 §6.2.2, ISO 15930-7 §6.2.2), and PDF/A permits several
    /// output intents that share one embedded `DestOutputProfile` (ISO 19005-1
    /// §6.2.2), so PDF/A composes with the embedded-profile PDF/X levels
    /// (PDF/X-1a, PDF/X-3, PDF/X-4, PDF/X-6). The sole incompatibility is the
    /// external-profile PDF/X levels (PDF/X-4p, PDF/X-6p), whose
    /// `DestOutputProfileRef` PDF/A forbids: that combination is rejected.
    IncompatibleOutputIntents(Validators),
}

#[cfg(test)]
mod tests {
    use crate::configure::{
        Accessibility, Archival, Configuration, ConfigurationBuilder, ConfigurationError,
        PdfVersion, Prepress,
    };

    #[test]
    fn invalid_combination_1() {
        // A1_B max is PDF 1.4; explicit PDF 1.7 is out of range.
        assert!(matches!(
            ConfigurationBuilder::new()
                .with_version(PdfVersion::Pdf17)
                .with_archival_validator(Archival::A1_B)
                .finish(),
            Err(ConfigurationError::VersionDoesNotMatchValidatorsRange(
                PdfVersion::Pdf17,
                _
            ))
        ));
    }

    #[test]
    fn invalid_combination_2() {
        // A4 requires PDF 2.0; UA1 max is PDF 1.7 → no overlapping range.
        assert!(matches!(
            ConfigurationBuilder::new()
                .with_archival_validator(Archival::A4)
                .with_accessibility_validator(Accessibility::UA1)
                .finish(),
            Err(ConfigurationError::NoOverlappingValidatorsRange(_))
        ));
    }

    #[test]
    fn invalid_combination_3() {
        // A1_B max is PDF 1.4; UA1 max is PDF 1.7 → intersection is PDF14..=PDF14.
        // Explicitly setting PDF 1.7 is out of range.
        assert!(matches!(
            ConfigurationBuilder::new()
                .with_archival_validator(Archival::A1_B)
                .with_accessibility_validator(Accessibility::UA1)
                .with_version(PdfVersion::Pdf17)
                .finish(),
            Err(ConfigurationError::VersionDoesNotMatchValidatorsRange(
                PdfVersion::Pdf17,
                _
            ))
        ));
    }

    #[test]
    fn multi_validator_pdf_a3b_pdf_ua1() {
        let config = ConfigurationBuilder::new()
            .with_archival_validator(Archival::A3_B)
            .with_accessibility_validator(Accessibility::UA1)
            .finish()
            .unwrap();
        assert_eq!(config.validators().archival(), Some(Archival::A3_B));
        assert_eq!(
            config.validators().accessibility(),
            Some(Accessibility::UA1)
        );
        assert_eq!(config.version(), PdfVersion::Pdf17);
    }

    #[test]
    fn multi_validator_pdfa2a_pdfua1() {
        assert!(ConfigurationBuilder::new()
            .with_archival_validator(Archival::A2_A)
            .with_accessibility_validator(Accessibility::UA1)
            .finish()
            .is_ok());
    }

    #[test]
    fn empty_validators() {
        let config = ConfigurationBuilder::new().finish().unwrap();
        assert!(config.validators().is_empty());
        assert_eq!(config.version(), PdfVersion::Pdf17);
    }

    #[test]
    fn default_config() {
        assert_eq!(
            ConfigurationBuilder::new().finish().unwrap(),
            Configuration::default()
        );
    }

    #[test]
    fn prepress_versions_are_pinned() {
        let version = |x: Prepress| {
            ConfigurationBuilder::new()
                .with_prepress_validator(x)
                .finish()
                .unwrap()
                .version()
        };
        assert_eq!(version(Prepress::X1A), PdfVersion::Pdf14);
        assert_eq!(version(Prepress::X3), PdfVersion::Pdf14);
        assert_eq!(version(Prepress::X4), PdfVersion::Pdf16);
        assert_eq!(version(Prepress::X4P), PdfVersion::Pdf16);
        assert_eq!(version(Prepress::X6), PdfVersion::Pdf20);
        assert_eq!(version(Prepress::X6P), PdfVersion::Pdf20);
    }

    #[test]
    fn prepress_rejects_out_of_range_version() {
        // PDF/X-4 is pinned to PDF 1.6; PDF 1.7 is out of range.
        assert!(matches!(
            ConfigurationBuilder::new()
                .with_prepress_validator(Prepress::X4)
                .with_version(PdfVersion::Pdf17)
                .finish(),
            Err(ConfigurationError::VersionDoesNotMatchValidatorsRange(
                PdfVersion::Pdf17,
                _
            ))
        ));
    }

    #[test]
    fn combined_archival_prepress_negotiates_version() {
        // PDF/A-2b (1.4..=1.7) + PDF/X-4 (1.6) -> 1.6.
        let config = ConfigurationBuilder::new()
            .with_archival_validator(Archival::A2_B)
            .with_prepress_validator(Prepress::X4)
            .finish()
            .unwrap();
        assert_eq!(config.version(), PdfVersion::Pdf16);
        assert_eq!(config.validators().archival(), Some(Archival::A2_B));
        assert_eq!(config.validators().prepress(), Some(Prepress::X4));

        // PDF/A-4 (2.0) + PDF/X-6 (2.0) -> 2.0.
        let config = ConfigurationBuilder::new()
            .with_archival_validator(Archival::A4)
            .with_prepress_validator(Prepress::X6)
            .finish()
            .unwrap();
        assert_eq!(config.version(), PdfVersion::Pdf20);
    }

    #[test]
    fn combined_archival_prepress_without_overlap_is_rejected() {
        // PDF/A-1b (max 1.4) + PDF/X-4 (1.6) have no overlapping version range.
        assert!(matches!(
            ConfigurationBuilder::new()
                .with_archival_validator(Archival::A1_B)
                .with_prepress_validator(Prepress::X4)
                .finish(),
            Err(ConfigurationError::NoOverlappingValidatorsRange(_))
        ));
    }

    #[test]
    fn pdfa1_with_pdfx1a_x3_is_allowed() {
        // A PDF/A-1 file may additionally carry a GTS_PDFX output intent
        // alongside its GTS_PDFA1 one: ISO 19005-1 §6.2.2 permits several output
        // intents sharing one embedded profile, and ISO 15930-4 §6.2.2 permits
        // additional output intents with a different S key. Both standards are
        // PDF 1.4, so the combination resolves to a valid PDF 1.4 configuration.
        for prepress in [Prepress::X1A, Prepress::X3] {
            for archival in [Archival::A1_A, Archival::A1_B] {
                let config = ConfigurationBuilder::new()
                    .with_archival_validator(archival)
                    .with_prepress_validator(prepress)
                    .finish()
                    .unwrap_or_else(|e| panic!("{archival:?} + {prepress:?} must build: {e:?}"));
                assert_eq!(config.version(), PdfVersion::Pdf14);
            }
        }
    }

    #[test]
    fn pdfa2_a3_with_pdfx1a_x3_is_allowed() {
        // PDF/A-2/3 (which krilla admits down to PDF 1.4) likewise composes with
        // the embedded-profile PDF/X-1a/X-3: the shared GTS_PDFA1 profile and the
        // additional GTS_PDFX intent are mutually permitted. The intersection of
        // the version ranges is PDF 1.4.
        for prepress in [Prepress::X1A, Prepress::X3] {
            for archival in [Archival::A2_B, Archival::A2_U, Archival::A3_B] {
                let config = ConfigurationBuilder::new()
                    .with_archival_validator(archival)
                    .with_prepress_validator(prepress)
                    .finish()
                    .unwrap_or_else(|e| panic!("{archival:?} + {prepress:?} must build: {e:?}"));
                assert_eq!(config.version(), PdfVersion::Pdf14);
            }
        }
    }

    #[test]
    fn pdfa_with_external_output_profile_pdfx_is_rejected() {
        // PDF/A requires its output profile to be embedded, while PDF/X-4p and
        // PDF/X-6p reference it externally via DestOutputProfileRef, which PDF/A
        // forbids. The version ranges overlap (A2/A3 + X-4p at 1.6, A4 + X-6p at
        // 2.0), so this must be caught as an output-intent conflict.
        for (archival, prepress) in [
            (Archival::A2_B, Prepress::X4P),
            (Archival::A3_B, Prepress::X4P),
            (Archival::A4, Prepress::X6P),
        ] {
            assert!(
                matches!(
                    ConfigurationBuilder::new()
                        .with_archival_validator(archival)
                        .with_prepress_validator(prepress)
                        .finish(),
                    Err(ConfigurationError::IncompatibleOutputIntents(_))
                ),
                "{archival:?} + {prepress:?} must be rejected"
            );
        }
    }
}
