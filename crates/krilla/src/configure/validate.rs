//! Exporting with a specific PDF conformance level.
//!
//! PDF defines a number of additional conformance levels that restrict the features of PDF that
//! can be used to a specific subset.
//!
//! You can use a [`Validator`] by creating a corresponding [`Configuration`]
//! you want to build the document with. There are three important aspects that play into this:
//! - krilla will internally write the file in a way that conforms to the given standard, i.e.
//!   by settings appropriate metadata. This happens under-the-hood and is completely abstracted
//!   away from the user.
//! - For aspects that are out of control of krilla and dependent on the input, krilla will perform
//!   a validation that the input is compatible with the standard. krilla will record all violations,
//!   and when calling `document.finish()`, in case there is at least one violation, krilla will
//!   return them as an error, instead of returning the finished document. See [`ValidationError`].
//! - Finally, some standards have requirements that cannot possibly be validated by krilla, as
//!   they are semantic in nature. It is upon you, as a user of that library, to ensure that those
//!   requirements are fulfilled. Therefore, while krilla tries to make it as easy as possible
//!   to generate compliant PDFs, it is still highly recommended that you familiarize yourself
//!   with the PDF specification as well as the specifications for the substandards. This is
//!   especially true for standards related to universal accessibility.
//!   
//!  You can find some requirements below **Requirements** for each [`Validator`].
//!
//! [`Configuration`]: crate::configure::Configuration

use std::collections::HashMap;
use std::fmt::Debug;

use pdf_writer::types::OutputIntentSubtype;
use xmp_writer::pdfa::PdfAExtSchemasWriter;
use xmp_writer::{Namespace, XmpWriter};

use crate::color::separation::SeparationColorant;
use crate::color::separation::SeparationSpace;
use crate::color::RegularColor;
use crate::configure::PdfVersion;
use crate::interchange::embed::EmbedError;
use crate::surface::Location;
use crate::text::Font;
use crate::text::GlyphId;

/// An error that occurred during validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValidationError {
    /// There was a string that was longer than the maximum allowed length (32767).
    ///
    /// Can for example occur if you set a title or an author that is longer than
    /// the given length.
    TooLongString,
    /// There was a name that was longer than the maximum allowed length (127).
    ///
    /// Can for example occur if the font name is too long.
    TooLongName,
    /// There was an array that was longer than the maximum allowed length (8191).
    /// Can only occur for PDF 1.4.
    ///
    /// Can for example occur if a text too long was written.
    TooLongArray,
    /// There was a dictionary with more entries than the maximum allowed (4095).
    /// Can only occur for PDF 1.4.
    ///
    /// Can for example occur if too many annotations are added to a page.
    TooLongDictionary,
    /// There was a float that is higher than the maximum allowed (32767).
    /// Can only occur for PDF 1.4.
    TooLargeFloat,
    /// The PDF exceeds the upper limit for indirect objects (8388607).
    ///
    /// Occurs if the PDF is simply too long.
    TooManyIndirectObjects,
    /// The PDF contains a content stream that exceeds maximum allowed q/Q nesting level (28).
    ///
    /// Can only occur if the user stacks many clip paths.
    TooHighQNestingLevel,
    /// The PDF contains PostScript code, which is forbidden by some export formats.
    ///
    /// Occurs if a gradient with spread method `Repeat`/`Reflect` or a sweep gradient was used.
    ContainsPostScript(Option<Location>),
    /// No CMYK ICC profile was provided, even though one is necessary.
    ///
    /// Occurs if the export format requires a device-independent color representation,
    /// and a CMYK color was used in the document.
    MissingCMYKProfile,
    /// The same Separation colorant was used with multiple different fallback colors.
    ///
    /// Occurs if the user specified multiple Separation color spaces with the same colorant but a different fallback color.
    InconsistentSeparationFallback(SeparationColorant),
    /// The `.notdef` glyph was used, which is forbidden by some export formats.
    ///
    /// Can occur if a glyph could not be found in the font for a corresponding codepoint
    /// in the input text, or if it was explicitly mapped that way.
    ///
    /// The third argument contains the text range of the glyph.
    ContainsNotDefGlyph(Font, Option<Location>, String),
    /// A glyph was mapped to no codepoint at all, which is forbidden by some
    /// standards.
    NoCodepointMapping(Font, GlyphId, Option<Location>),
    /// A glyph was mapped either to the codepoint 0x0, 0xFEFF or 0xFFFE, which
    /// is forbidden by some standards.
    ///
    /// Can occur if those codepoints appeared in the input text, or were
    /// explicitly mapped to that glyph.
    InvalidCodepointMapping(Font, GlyphId, char, Option<Location>),
    /// A glyph was mapped to a codepoint in the Unicode private use area, which is forbidden
    /// by some standards, like for example PDF/A-2a.
    // Note that the standard doesn't explicitly forbid it, but instead requires an ActualText
    // attribute to be present. But we just completely forbid it, for simplicity.
    UnicodePrivateArea(Font, GlyphId, char, Option<Location>),
    /// A font has a license that requires explicit permission of the legal owner for embedding
    /// but the standard requires font programs to be legally embeddable for universal rendering.
    RestrictedLicense(Font),
    /// No document language was set via the metadata, even though it is required
    /// by the standard.
    NoDocumentLanguage,
    /// No title was provided for the document, even though it is required by
    /// the standard.
    NoDocumentTitle,
    /// A figure or formula is missing an alt text.
    MissingAltText(Option<Location>),
    /// A heading is missing a title.
    MissingHeadingTitle,
    /// The document does not contain an outline.
    MissingDocumentOutline,
    /// An annotation is missing an alt text.
    MissingAnnotationAltText(Option<Location>),
    /// The date of the document is missing.
    // We need this because for some standards we need to add the
    // xmp:History attribute.
    MissingDocumentDate,
    /// The PDF contains transparency, which is forbidden by some standards (e.g. PDF/A-1).
    Transparency(Option<Location>),
    /// The PDF contains an image with `interpolate` set to `true`.
    ImageInterpolation(Option<Location>),
    /// The PDF contains an embedded file.
    EmbeddedFile(EmbedError, Option<Location>),
    /// The PDF contains no tagging.
    MissingTagging,
    /// The PDF contains another embedded PDF.
    ///
    /// This is currently forbidden in validated export because we cannot manually verify
    /// whether the file actually fulfills all the criteria for the export mode.
    EmbeddedPDF(Option<Location>),
    /// A feature only available in a later PDF version was required.
    RequiresNewerPdfVersion(VersionedFeature, Option<Location>),
    /// No external output profile reference was provided for PDF/X-4p or
    /// PDF/X-6p.
    ///
    /// Occurs if the export target is PDF/X-4p or PDF/X-6p and the caller did
    /// not set [`SerializeSettings::external_output_profile`].
    ///
    /// [`SerializeSettings::external_output_profile`]:
    /// crate::SerializeSettings::external_output_profile
    MissingExternalOutputProfile,
    /// An external output profile reference was provided, but none of the
    /// active validators makes use of it.
    ///
    /// Occurs if [`SerializeSettings::external_output_profile`] is `Some` while
    /// no PDF/X-4p or PDF/X-6p validator is configured.
    ///
    /// [`SerializeSettings::external_output_profile`]:
    /// crate::SerializeSettings::external_output_profile
    ExternalOutputProfileUnsupportedByValidator,
    /// The PDF contains an RGB color, which is forbidden by PDF/X-1a.
    ///
    /// Occurs if an RGB color was used in fills, strokes, gradients, images,
    /// or separation fallback colors when exporting to PDF/X-1a. Grayscale
    /// colors are permitted.
    ContainsRgb(Option<Location>),
    /// A gradient's stops are not all in the same color space.
    ///
    /// Occurs if the [`Stop`](crate::paint::Stop)s supplied to a
    /// [`LinearGradient`](crate::paint::LinearGradient),
    /// [`RadialGradient`](crate::paint::RadialGradient), or
    /// [`SweepGradient`](crate::paint::SweepGradient) resolve to different
    /// color spaces. krilla normalizes the stops to the first stop's color
    /// space when this happens.
    MixedGradientColorSpaces(Option<Location>),
    /// A page is missing both a TrimBox and an ArtBox, which is required by
    /// PDF/X.
    ///
    /// Occurs if a page does not have either a TrimBox or an ArtBox set in
    /// its [`PageSettings`](crate::page::PageSettings). The first field is
    /// the zero-based index of the offending page.
    MissingTrimOrArtBox(usize, Option<Location>),
    /// A page in a PDF/X-6 or PDF/X-6p file is missing the mandatory TrimBox.
    ///
    /// ISO 15930-9 §6.9 requires every page to carry a TrimBox; unlike the
    /// earlier levels an ArtBox is not an acceptable substitute. The first
    /// field is the zero-based index of the offending page.
    MissingTrimBox(usize, Option<Location>),
    /// The PDF contains an annotation that krilla does not support under
    /// PDF/X-1a.
    ///
    /// ISO 15930-4 §6.13 permits non-TrapNet/PrinterMark annotations only when
    /// their `Rect` lies wholly outside the print area, but krilla supports
    /// neither TrapNet/PrinterMark nor positioned-Link annotations under
    /// PDF/X-1a, so it conservatively rejects every annotation at this level.
    ContainsAnnotation(Option<Location>),
    /// The PDF contains an interactive action under a PDF/X level that forbids
    /// them.
    ///
    /// PDF/X-1a, PDF/X-3, PDF/X-4 and PDF/X-4p forbid all Actions and
    /// JavaScripts (ISO 15930-4 §6.14, ISO 15930-6 §6.14, ISO 15930-7 §6.18).
    /// PDF/X-6/-6p (ISO 15930-9 §6.14) instead permit GoTo/URI actions — the
    /// only action types krilla emits — so this is never raised for them.
    /// krilla emits actions only as the target of a link annotation; a link with
    /// an in-document destination (rather than an action) is always permitted.
    ContainsAction(Option<Location>),
    /// A page sets both a TrimBox and an ArtBox, which is forbidden by PDF/X.
    ///
    /// A PDF/X page must carry exactly one of the two; setting both is a
    /// conformance error. The first field is the zero-based index of the
    /// offending page.
    BothTrimAndArtBox(usize, Option<Location>),
    /// A page box is not nested within its containing box, which is required by
    /// PDF/X.
    ///
    /// Every page box must be enclosed by the boxes containing it
    /// (ISO 15930-4 §6.8, ISO 15930-7 §6.12): the MediaBox must contain the
    /// CropBox, BleedBox, TrimBox and ArtBox; a CropBox (if present) must contain
    /// the BleedBox, TrimBox and ArtBox; and a BleedBox (if present) must contain
    /// the TrimBox/ArtBox. The first field is the zero-based index of the
    /// offending page.
    PageBoxNotNested(usize, Option<Location>),
    /// A page box has a non-positive (zero or negative) extent, which is not a
    /// valid page region under PDF/X.
    ///
    /// The first field is the zero-based index of the offending page.
    DegeneratePageBox(usize, Option<Location>),
    /// A page box exceeds the PDF 1.4 maximum page dimension (14400 units per
    /// side), which the PDF 1.4-based PDF/X-1a and PDF/X-3 inherit.
    ///
    /// The first field is the zero-based index of the offending page.
    PageBoxTooLarge(usize, Option<Location>),
    /// An annotation is not positioned wholly outside the page's print area,
    /// which is required by PDF/X (PDF/X-1a forbids annotations outright and
    /// reports [`ContainsAnnotation`](ValidationError::ContainsAnnotation)
    /// instead).
    ///
    /// PDF/X requires every annotation — other than the exempt TrapNet and
    /// PrinterMark subtypes, neither of which krilla emits — to lie entirely
    /// outside the BleedBox, or outside the TrimBox/ArtBox if no BleedBox is
    /// present.
    AnnotationInsidePrintArea(Option<Location>),
    /// An annotation uses an RGB color, which is forbidden by PDF/X under the
    /// CMYK output intent krilla writes.
    ///
    /// Unlike page content, an annotation's `/C` or `/IC` color array cannot be
    /// ICC-wrapped, so an RGB annotation color is emitted as DeviceRGB, which is
    /// not characterized by a CMYK output intent.
    AnnotationContainsRgb(Option<Location>),
    /// Device colour content is used that the PDF/X output intent does not
    /// characterize.
    ///
    /// A device colour space may be used only if it matches the output intent's
    /// colour space, or the intent is CMYK and the space is DeviceGray
    /// (ISO 15930-7 §6.4.3.2, ISO 15930-9 §6.6.3.2). This is raised when
    /// DeviceCMYK content is emitted under a non-CMYK output intent (the
    /// embedded [`cmyk_profile`](crate::SerializeSettings::cmyk_profile) or the
    /// external [`ExternalOutputProfile`](crate::ExternalOutputProfile) is not a
    /// CMYK profile), or when DeviceGray content is emitted under an RGB output
    /// intent (which would require a DefaultGray colour space krilla does not
    /// emit).
    OutputIntentColorSpaceMismatch(Option<Location>),
    /// The output-intent ICC profile uses an ICC specification version newer
    /// than the target PDF version supports.
    ///
    /// PDF 1.4 (PDF/X-1a, PDF/X-3) admits only ICC v2 profiles; PDF 1.6
    /// (PDF/X-4, PDF/X-4p) up to ICC v4.2 (ISO 15076-1:2005); PDF 2.0
    /// (PDF/X-6, PDF/X-6p) up to ICC v4.3 (ISO 15076-1:2010).
    IncompatibleOutputProfileVersion(Option<Location>),
    /// The output-intent ICC profile is not an output device profile, which a
    /// PDF/X output intent requires.
    ///
    /// ISO 15930-7 §6.4.2.1 requires the profile to be an Output Device Profile
    /// (ICC Device Class `prtr`). Display (`mntr`), input (`scnr`) and transform
    /// profiles (`link`, `spac`, `abst`, `nmcl`) are not valid PDF/X output
    /// intents (this is stricter than PDF/A, which also admits `mntr`).
    InvalidOutputProfileDeviceClass(Option<Location>),
    /// The output-intent ICC profile's data colour space is not one of
    /// GRAY/RGB/CMYK, which a PDF/X output intent requires.
    ///
    /// ISO 15930-7 §6.4.1 / ISO 15930-9 §6.6.1: "The characterized printing
    /// condition shall have one colour channel (grayscale), three colour
    /// channels (RGB) or four colour channels (CMYK)." A four-channel but
    /// non-`'CMYK'` profile (e.g. an `'4CLR'` DeviceN profile, reserved for the
    /// PDF/X-5n/-6n levels krilla does not implement) is therefore rejected.
    InvalidOutputProfileColorSpace(Option<Location>),
}

/// Features that may require a later PDF version than the current one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VersionedFeature {
    /// Tabbing through the document according to the structure order.
    StructureOrderTabbing,
    /// Header and footer artifact subtypes.
    HeaderFooterArtifactSubtypes,
    /// Scope attribute for table header cells.
    TableHeaderScope,
}

impl VersionedFeature {
    /// Get the minimum PDF version required for this feature.
    pub fn minimum_pdf_version(&self) -> PdfVersion {
        match self {
            VersionedFeature::StructureOrderTabbing => PdfVersion::Pdf15,
            VersionedFeature::HeaderFooterArtifactSubtypes => PdfVersion::Pdf17,
            VersionedFeature::TableHeaderScope => PdfVersion::Pdf15,
        }
    }
}

/// Collection of validators with at most one validator for each standard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub struct Validators {
    a: Option<Archival>,
    ua: Option<Accessibility>,
    x: Option<Prepress>,
}

impl Validators {
    /// Returns a filtered `Validators` containing only validators that prohibit the given error,
    /// or `None` if no validator prohibits it.
    pub fn prohibits(self, error: &ValidationError) -> Option<Self> {
        let a = self.a.filter(|v| v.prohibits(error));
        let ua = self.ua.filter(|v| v.prohibits(error));
        let x = self.x.filter(|v| v.prohibits(error));

        let any = a.is_some() || ua.is_some() || x.is_some();
        any.then_some(Self { a, ua, x })
    }

    /// Returns `true` if no validators are set.
    pub fn is_empty(self) -> bool {
        self.a.is_none() && self.ua.is_none() && self.x.is_none()
    }

    /// Returns the number of set validators.
    pub fn len(self) -> usize {
        usize::from(self.a.is_some())
            + usize::from(self.ua.is_some())
            + usize::from(self.x.is_some())
    }

    /// Returns the PDF/A validator, if set.
    pub fn archival(self) -> Option<Archival> {
        self.a
    }

    /// Returns the PDF/UA accessibility validator, if set.
    pub fn accessibility(self) -> Option<Accessibility> {
        self.ua
    }

    /// Returns the PDF/X prepress validator, if set.
    pub fn prepress(self) -> Option<Prepress> {
        self.x
    }

    /// Whether the font must supply valid Unicode code points for each of the
    /// drawn glyphs.
    pub(crate) fn requires_codepoint_mappings(self) -> bool {
        self.into_iter().any(Validator::requires_codepoint_mappings)
    }

    /// Force the `DisplayDocTitle` flag set.
    pub(crate) fn requires_display_doc_title(self) -> bool {
        self.ua
            .is_some_and(Accessibility::requires_display_doc_title)
    }

    /// Force sRGB profiles for `DeviceGray` and `DeviceRgb` colorspaces.
    pub(crate) fn requires_no_device_cs(self) -> bool {
        // PDF/X-1a forbids ICCBased color spaces, so device color spaces must be
        // emitted directly (the CMYK output intent provides the device-independent
        // interpretation). This overrides PDF/A's preference for ICC substitution.
        if self.x == Some(Prepress::X1A) {
            return false;
        }

        self.a.is_some_and(Archival::requires_no_device_cs)
            || self.x.is_some_and(Prepress::requires_no_device_cs)
    }

    /// Force the `Print` flag set and the `Hidden`, `Invisible`,
    /// `ToggleNoView`, and `NoView` flags unset.
    pub(crate) fn requires_annotation_flags(self) -> bool {
        self.a.is_some_and(Archival::requires_annotation_flags)
            || self.x.is_some_and(Prepress::requires_annotation_flags)
    }

    /// Whether Tagged PDF must be enabled.
    pub(crate) fn requires_tagging(self) -> bool {
        self.into_iter().any(Validator::requires_tagging)
    }

    /// Whether XMP metadata must be written.
    pub(crate) fn requires_xmp_metadata(self) -> bool {
        self.into_iter().any(Validator::requires_xmp_metadata)
    }

    /// Whether any extension schemata should be descibed using the "pdfaSchema"
    /// namespace.
    pub(crate) fn requires_xmp_metadata_extension_schema(self) -> bool {
        self.a
            .is_some_and(Archival::requires_xmp_metadata_extension_schema)
    }

    /// Whether the `instanceID` field is allowed in XMP.
    pub(crate) fn prohibits_instance_id_in_xmp_metadata(self) -> bool {
        self.a
            .is_some_and(Archival::prohibits_instance_id_in_xmp_metadata)
    }

    /// Whether the xmpMM:History entry is required.
    pub(crate) fn requires_file_provenance_information(self) -> bool {
        self.a
            .is_some_and(Archival::requires_file_provenance_information)
    }

    /// Whether the `/Info` dictionary is forbidden in the file trailer.
    pub(crate) fn prohibits_info_dict(self) -> bool {
        // PDF/A-4 (PDF 2.0) deprecates the document information dictionary, but
        // the PDF 1.4/1.6-based PDF/X levels (X-1a/X-3/X-4/X-4p) still use it for
        // `/GTS_PDFXVersion` and `/Trapped`, so a PDF/X validator overrides
        // PDF/A-4's prohibition — except PDF/X-6/-6p, which are themselves PDF
        // 2.0-based: ISO 15930-9 §6.5.2 forbids the `Info` key unless a
        // `PieceInfo` entry exists (which krilla never writes), carrying the
        // metadata in XMP instead.
        (self.a.is_some_and(Archival::prohibits_info_dict) && self.x.is_none())
            || matches!(self.x, Some(Prepress::X6 | Prepress::X6P))
    }

    /// Whether a non-printable file header is mandatory.
    pub(crate) fn requires_binary_header(self) -> bool {
        self.a.is_some_and(Archival::requires_binary_header)
            || self.x.is_some_and(Prepress::requires_binary_header)
    }

    /// Whether the `EmbeddedFiles` key in the name dictionary of the document
    /// catalog dictionary should be written even if empty.
    pub(crate) fn requires_embedded_files_when_empty(self) -> bool {
        self.a
            .is_some_and(Archival::requires_embedded_files_when_empty)
    }

    /// Whether any of these standards explicitly specifies the `/AF` key.
    ///
    /// The `/AF` key may be supported by the underlying PDF version instead:
    /// Starting at PDF 2.0, the key is specified by ISO 32000 and does not need
    /// to be added by PDF/A.
    pub(crate) fn specifies_associated_files(self) -> bool {
        self.a.is_some_and(Archival::specifies_associated_files)
    }

    /// The output intent subtypes that must be written, in order.
    ///
    /// PDF/A contributes a `GTS_PDFA1` output intent and PDF/X a `GTS_PDFX` one.
    /// When both are set, both are written: every PDF/X level permits additional
    /// output intents with a different `S` key (ISO 15930-4/-6/-7/-9, e.g.
    /// ISO 15930-7 §6.4.2.1: "Additional output intent dictionaries may be
    /// present; if so, they shall use different values for the S key"), and every
    /// PDF/A level permits multiple output intents that share one embedded
    /// profile (ISO 19005-1 §6.2.2, ISO 19005-2/-3/-4 §6.2.3). The two intents name the same
    /// embedded CMYK device target. The only combination that cannot be written
    /// is rejected up front by [`Validators::has_incompatible_output_intents`].
    pub(crate) fn output_intents(self) -> Vec<OutputIntentSubtype<'static>> {
        let mut intents = Vec::new();
        if let Some(a) = self.a {
            intents.push(a.output_intent());
        }
        if let Some(x) = self.x {
            intents.push(x.output_intent());
        }
        intents
    }

    /// Whether the active PDF/A and PDF/X validators have irreconcilable
    /// output-intent requirements that no single file can satisfy.
    ///
    /// PDF/A and PDF/X otherwise compose freely: both standards permit a file to
    /// carry a `GTS_PDFA1` and a `GTS_PDFX` output intent sharing one embedded
    /// profile (ISO 15930 §6.x; ISO 19005-1 §6.2.2, ISO 19005-2/-3/-4 §6.2.3).
    /// The sole exception is the
    /// external-profile PDF/X variants (PDF/X-4p, PDF/X-6p): their `GTS_PDFX`
    /// intent references the profile via `DestOutputProfileRef`, which PDF/A
    /// forbids — PDF/A requires the output profile to be embedded for
    /// self-containment.
    pub(crate) fn has_incompatible_output_intents(self) -> bool {
        self.a.is_some()
            && self
                .x
                .is_some_and(Prepress::requires_external_output_profile)
    }

    /// Whether the PDF/X output intent references its ICC profile externally
    /// (PDF/X-4p, PDF/X-6p) instead of embedding it.
    pub(crate) fn requires_external_output_profile(self) -> bool {
        self.x
            .is_some_and(Prepress::requires_external_output_profile)
    }

    /// Whether the embedded CMYK output profile should be used when writing the
    /// output intent for the given subtype.
    ///
    /// The PDFX intent embeds the CMYK profile only for the non-`p` variants
    /// (the `p` variants reference it externally via `DestOutputProfileRef`,
    /// handled before this is consulted). The PDFA intent is only present for a
    /// combined PDF/A + PDF/X export, which is restricted to the embedded-profile
    /// PDF/X levels (PDF/A forbids external references, so the `p` variants
    /// cannot be combined with it); it always embeds the CMYK profile so PDF/A
    /// characterizes the DeviceCMYK content itself. Both intents then name the
    /// same CMYK device target, as permitted by ISO 15930-7 §6.4.2.1 and
    /// ISO 19005-2 §6.2.3.
    pub(crate) fn uses_cmyk_output_profile_for_subtype(
        self,
        subtype: OutputIntentSubtype<'_>,
    ) -> bool {
        self.is_pdf_x()
            && ((subtype == OutputIntentSubtype::PDFX && !self.requires_external_output_profile())
                || (subtype == OutputIntentSubtype::PDFA && self.a.is_some()))
    }

    /// Whether every page must have a TrimBox or an ArtBox (but not both).
    ///
    /// This is the rule for the PDF 1.4/1.6-based levels (ISO 15930-4 §6.8,
    /// ISO 15930-6 §6.8, ISO 15930-7 §6.12). PDF/X-6/-6p instead mandate a
    /// TrimBox specifically — see [`Validators::requires_trim_box`].
    pub(crate) fn requires_trim_or_art_box(self) -> bool {
        matches!(
            self.x,
            Some(Prepress::X1A | Prepress::X3 | Prepress::X4 | Prepress::X4P)
        )
    }

    /// Whether every page must have a TrimBox specifically (PDF/X-6/-6p).
    ///
    /// ISO 15930-9 §6.9: "Each Page object in a PDF/X-6 conforming file shall
    /// include a TrimBox." Unlike the earlier levels an ArtBox is not an
    /// acceptable substitute, and a coexisting ArtBox is permitted.
    pub(crate) fn requires_trim_box(self) -> bool {
        matches!(self.x, Some(Prepress::X6 | Prepress::X6P))
    }

    /// Whether annotations must lie wholly outside the print area.
    ///
    /// PDF/X-3/-4/-4p require this (ISO 15930-6 §6.13, ISO 15930-7 §6.17).
    /// PDF/X-1a forbids annotations outright, and PDF/X-6/-6p (ISO 15930-9
    /// §6.12) permit annotations inside the visible area, so neither imposes
    /// this positional rule.
    pub(crate) fn requires_annotations_outside_print_area(self) -> bool {
        matches!(self.x, Some(Prepress::X3 | Prepress::X4 | Prepress::X4P))
    }

    /// Whether trapping metadata (`/Trapped`, `xmp:Trapped`) must be written
    /// (all PDF/X standards).
    pub(crate) fn requires_trapping_metadata(self) -> bool {
        self.x.is_some()
    }

    /// Whether only CMYK, grayscale, and Separation colors may be used (PDF/X-1a).
    pub(crate) fn requires_cmyk_only(self) -> bool {
        self.x == Some(Prepress::X1A)
    }

    /// Whether all annotations are forbidden (PDF/X-1a).
    pub(crate) fn forbids_annotations(self) -> bool {
        self.x.is_some_and(Prepress::forbids_annotations)
    }

    /// Whether interactive actions are forbidden (all PDF/X standards).
    pub(crate) fn forbids_actions(self) -> bool {
        self.x.is_some_and(Prepress::forbids_actions)
    }

    /// Whether the XMP `xmp:MetadataDate` entry must be written (PDF/X-4/-6).
    pub(crate) fn requires_xmp_metadata_date(self) -> bool {
        self.x.is_some_and(Prepress::requires_xmp_metadata_date)
    }

    /// Whether the XMP `xmpMM:VersionID` entry must be written (PDF/X-4/-6).
    pub(crate) fn requires_xmp_version_id(self) -> bool {
        self.x.is_some_and(Prepress::requires_xmp_version_id)
    }

    /// The `GTS_PDFXVersion` identification string for the active PDF/X
    /// validator, if any.
    pub(crate) fn gts_pdfx_version_string(self) -> Option<&'static str> {
        self.x.map(Prepress::gts_pdfx_version_string)
    }

    /// Whether any PDF/X standard is active.
    pub(crate) fn is_pdf_x(self) -> bool {
        self.x.is_some()
    }

    pub(crate) fn write_xmp(self, xmp: &mut XmpWriter) {
        if self.requires_xmp_metadata_extension_schema() {
            let mut extension_schemas = xmp.extension_schemas();
            if let Some(a) = self.a {
                a.write_xmp_extension_schema_description(&mut extension_schemas);
            }
            if let Some(ua) = self.ua {
                ua.write_xmp_extension_schema_description(&mut extension_schemas);
            }
            // When a PDF/X validator is combined with a PDF/A standard that uses
            // inline extension schemata, the `pdfxid:GTS_PDFXVersion` property
            // must be described as well.
            if let Some(x) = self.x {
                x.write_xmp_extension_schema_description(&mut extension_schemas);
            }
        }

        if let Some(a) = self.a {
            a.write_xmp(xmp);
        }

        if let Some(ua) = self.ua {
            ua.write_xmp(xmp);
        }

        if let Some(x) = self.x {
            x.write_xmp(xmp);
        }
    }

    /// Returns the maximum PDF version allowed by all active validators.
    pub fn max(self) -> PdfVersion {
        self.a
            .map_or(PdfVersion::MAX, |v| v.max())
            .min(self.ua.map_or(PdfVersion::MAX, |v| v.max()))
            .min(self.x.map_or(PdfVersion::MAX, |v| v.max()))
    }

    /// Returns the minimum PDF version required by all active validators, if any.
    pub fn min(self) -> Option<PdfVersion> {
        self.a
            .and_then(|v| v.min())
            .max(self.ua.and_then(|v| v.min()))
            .max(self.x.and_then(|v| v.min()))
    }
}

impl IntoIterator for Validators {
    type Item = Validator;
    type IntoIter = std::iter::Flatten<std::array::IntoIter<Option<Validator>, 3>>;

    fn into_iter(self) -> Self::IntoIter {
        [
            self.a.map(Validator::A),
            self.ua.map(Validator::Ua),
            self.x.map(Validator::X),
        ]
        .into_iter()
        .flatten()
    }
}

/// A builder for constructing a [`Validators`] collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub struct ValidatorsBuilder(Validators);

impl ValidatorsBuilder {
    /// Set a validator, overwriting the current one if the same standard family is already set.
    pub fn set_validator(self, validator: Validator) -> Self {
        match validator {
            Validator::A(a) => self.with_archival_validator(a),
            Validator::Ua(ua) => self.with_accessibility_validator(ua),
            Validator::X(x) => self.with_prepress_validator(x),
        }
    }

    /// Set the PDF/A validator, overwriting the current one if already set.
    pub fn with_archival_validator(mut self, archival: Archival) -> Self {
        self.0.a = Some(archival);
        self
    }

    /// Set the PDF/UA accessibility validator, overwriting the current one if already set.
    pub fn with_accessibility_validator(mut self, accessibility: Accessibility) -> Self {
        self.0.ua = Some(accessibility);
        self
    }

    /// Set the PDF/X prepress validator, overwriting the current one if already set.
    pub fn with_prepress_validator(mut self, prepress: Prepress) -> Self {
        self.0.x = Some(prepress);
        self
    }

    pub(crate) fn finish(self) -> Result<Validators, Validators> {
        let min = self.0.min().unwrap_or(PdfVersion::MIN);
        let max = self.0.max();

        if min > max {
            Err(self.0)
        } else {
            Ok(self.0)
        }
    }
}

/// A PDF validator for a specific conformance standard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Validator {
    /// A PDF/A validator.
    A(Archival),
    /// A PDF/UA accessibility validator.
    Ua(Accessibility),
    /// A PDF/X prepress validator.
    X(Prepress),
}

impl Validator {
    fn requires_codepoint_mappings(self) -> bool {
        match self {
            Self::A(a) => a.requires_codepoint_mappings(),
            Self::Ua(ua) => ua.requires_codepoint_mappings(),
            Self::X(x) => x.requires_codepoint_mappings(),
        }
    }

    fn requires_tagging(self) -> bool {
        match self {
            Self::A(a) => a.requires_tagging(),
            Self::Ua(ua) => ua.requires_tagging(),
            Self::X(x) => x.requires_tagging(),
        }
    }

    fn requires_xmp_metadata(self) -> bool {
        match self {
            Self::A(a) => a.requires_xmp_metadata(),
            Self::Ua(ua) => ua.requires_xmp_metadata(),
            Self::X(x) => x.requires_xmp_metadata(),
        }
    }

    /// Minimum PDF version required to use this validator, if any.
    pub fn min(self) -> Option<PdfVersion> {
        match self {
            Self::A(a) => a.min(),
            Self::Ua(ua) => ua.min(),
            Self::X(x) => x.min(),
        }
    }

    /// Maximum PDF version this standard can be used with.
    pub fn max(self) -> PdfVersion {
        match self {
            Self::A(a) => a.max(),
            Self::Ua(ua) => ua.max(),
            Self::X(x) => x.max(),
        }
    }

    /// Returns a human-readable string representation of the validator.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A(a) => a.as_str(),
            Self::Ua(ua) => ua.as_str(),
            Self::X(x) => x.as_str(),
        }
    }
}

impl From<Archival> for Validator {
    fn from(a: Archival) -> Self {
        Self::A(a)
    }
}

impl From<Accessibility> for Validator {
    fn from(ua: Accessibility) -> Self {
        Self::Ua(ua)
    }
}

impl From<Prepress> for Validator {
    fn from(x: Prepress) -> Self {
        Self::X(x)
    }
}

/// A PDF/A conformance level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum Archival {
    /// The validator for the PDF/A-1a standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A-1b.
    /// - You need to follow all requirements outlined in the _Other Notes_ section of the
    ///   [`tagging`] module.
    /// - You need to follow all best practices when using [tags](`crate::interchange::tagging::Tag`), as outlined in the documentation
    ///   of each tag.
    /// - Artifacts such as page numbers, backgrounds, cut marks and color bars should be specified
    ///   correspondingly as artifacts.
    /// - Word boundaries need to be explicitly specified with a space. The same applies to words at
    ///   the end of a line that are not followed by punctuation.
    /// - To the fullest extent possible, the logical structure of the document should be encoded
    ///   correspondingly in the tag tree using appropriate grouping tags.
    /// - Language identifiers used must be valid according to RFC 3066.
    /// - You should provide an alternate text to span content tags, if applicable.
    /// - You should provide the expansion of abbreviations to span content tags, if applicable.
    ///
    /// [`tagging`]: crate::interchange::tagging
    A1_A,
    /// The validator for the PDF/A-1b standard.
    ///
    /// **Requirements**: -
    A1_B,
    /// The validator for the PDF/A-2a standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A-2b.
    /// - You need to follow all requirements outlined in the _Other Notes_ section of the
    ///   [`tagging`] module.
    /// - You need to follow all best practices when using [tags](`crate::interchange::tagging::Tag`), as outlined in the documentation
    ///   of each tag.
    /// - Artifacts such as page numbers, backgrounds, cut marks and color bars should be specified
    ///   correspondingly as artifacts.
    /// - Word boundaries need to be explicitly specified with a space. The same applies to words at
    ///   the end of a line that are not followed by punctuation.
    /// - To the fullest extent possible, the logical structure of the document should be encoded
    ///   correspondingly in the tag tree using appropriate grouping tags.
    /// - Language identifiers used must be valid according to RFC 3066.
    /// - You should provide an alternate text to span content tags, if applicable.
    /// - You should provide the expansion of abbreviations to span content tags, if applicable.
    ///
    /// [`tagging`]: crate::interchange::tagging
    A2_A,
    /// The validator for the PDF/A-2b standard.
    ///
    /// **Requirements**:
    /// - You should only use fonts that are legally embeddable in a file for unlimited,
    ///   universal rendering.
    A2_B,
    /// The validator for the PDF/A-2u standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A-2b
    A2_U,
    /// The validator for the PDF/A-3a standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A-2a
    A3_A,
    /// The validator for the PDF/A-3b standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A-2b
    A3_B,
    /// The validator for the PDF/A-3u standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A-2b
    A3_U,
    /// The validator for the PDF/A-4 standard.
    ///
    /// **Requirements**:
    /// - While not required, it's recommended to enable tagging.
    A4,
    /// The validator for the PDF/A-4f standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A-4
    A4F,
    /// The validator for the PDF/A-4e standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A-4
    A4E,
}

impl Archival {
    fn prohibits(self, error: &ValidationError) -> bool {
        match (self, error) {
            // PDF/X-specific errors have a uniform verdict across every PDF/A
            // profile: PDF/A normalizes mixed gradient color spaces and never
            // makes use of an external output profile, but it permits RGB,
            // annotations, and pages without a TrimBox/ArtBox.
            (
                _,
                ValidationError::MixedGradientColorSpaces(_)
                | ValidationError::ExternalOutputProfileUnsupportedByValidator,
            ) => true,
            (
                _,
                ValidationError::ContainsRgb(_)
                | ValidationError::MissingTrimOrArtBox(_, _)
                | ValidationError::MissingTrimBox(_, _)
                | ValidationError::BothTrimAndArtBox(_, _)
                | ValidationError::PageBoxNotNested(_, _)
                | ValidationError::DegeneratePageBox(_, _)
                | ValidationError::PageBoxTooLarge(_, _)
                | ValidationError::ContainsAnnotation(_)
                | ValidationError::ContainsAction(_)
                | ValidationError::AnnotationInsidePrintArea(_)
                | ValidationError::AnnotationContainsRgb(_)
                | ValidationError::OutputIntentColorSpaceMismatch(_)
                | ValidationError::IncompatibleOutputProfileVersion(_)
                | ValidationError::InvalidOutputProfileDeviceClass(_)
                | ValidationError::InvalidOutputProfileColorSpace(_)
                | ValidationError::MissingExternalOutputProfile,
            ) => false,
            // Forbidden under all PDF/A-1 profiles.
            (
                Self::A1_A | Self::A1_B,
                ValidationError::TooLongString
                | ValidationError::TooLongName
                | ValidationError::TooLongArray
                | ValidationError::TooLongDictionary
                | ValidationError::TooLargeFloat
                | ValidationError::TooManyIndirectObjects
                | ValidationError::TooHighQNestingLevel
                | ValidationError::ContainsPostScript(_)
                | ValidationError::MissingCMYKProfile
                | ValidationError::RestrictedLicense(_)
                | ValidationError::MissingDocumentDate
                | ValidationError::Transparency(_)
                | ValidationError::ImageInterpolation(_)
                | ValidationError::EmbeddedFile(EmbedError::Existence, _)
                | ValidationError::EmbeddedPDF(_),
            ) => true,
            // Allowed under all PDF/A-1 profiles.
            (
                Self::A1_A | Self::A1_B,
                ValidationError::InconsistentSeparationFallback(_)
                | ValidationError::InvalidCodepointMapping(_, _, _, _)
                | ValidationError::UnicodePrivateArea(_, _, _, _)
                | ValidationError::NoDocumentTitle
                | ValidationError::MissingHeadingTitle
                | ValidationError::MissingDocumentOutline
                | ValidationError::EmbeddedFile(_, _)
                | ValidationError::RequiresNewerPdfVersion(
                    VersionedFeature::HeaderFooterArtifactSubtypes
                    | VersionedFeature::StructureOrderTabbing
                    | VersionedFeature::TableHeaderScope,
                    _,
                ),
            ) => false,
            // Forbidden under PDF/A-1a but allowed under PDF/A-1b.
            (
                Self::A1_A | Self::A1_B,
                ValidationError::ContainsNotDefGlyph(_, _, _)
                | ValidationError::NoCodepointMapping(_, _, _)
                | ValidationError::NoDocumentLanguage
                | ValidationError::MissingAltText(_)
                | ValidationError::MissingAnnotationAltText(_)
                | ValidationError::MissingTagging,
            ) => self == Self::A1_A,

            // Forbidden under all PDF/A-2 and PDF/A-3 profiles.
            (
                Self::A2_A | Self::A2_B | Self::A2_U | Self::A3_A | Self::A3_B | Self::A3_U,
                ValidationError::TooLongString
                | ValidationError::TooLongName
                | ValidationError::TooManyIndirectObjects
                | ValidationError::TooHighQNestingLevel
                | ValidationError::ContainsPostScript(_)
                | ValidationError::MissingCMYKProfile
                | ValidationError::InconsistentSeparationFallback(_)
                | ValidationError::ContainsNotDefGlyph(_, _, _)
                | ValidationError::RestrictedLicense(_)
                | ValidationError::MissingDocumentDate
                | ValidationError::ImageInterpolation(_)
                | ValidationError::EmbeddedPDF(_),
            ) => true,
            // Allowed under all PDF/A-2 and PDF/A-3 profiles.
            (
                Self::A2_A | Self::A2_B | Self::A2_U | Self::A3_A | Self::A3_B | Self::A3_U,
                ValidationError::TooLongArray
                | ValidationError::TooLongDictionary
                | ValidationError::TooLargeFloat
                | ValidationError::NoDocumentTitle
                | ValidationError::Transparency(_)
                | ValidationError::MissingHeadingTitle
                | ValidationError::MissingDocumentOutline
                | ValidationError::RequiresNewerPdfVersion(
                    VersionedFeature::HeaderFooterArtifactSubtypes
                    | VersionedFeature::StructureOrderTabbing
                    | VersionedFeature::TableHeaderScope,
                    _,
                ),
            ) => false,
            // Forbidden under PDF/A-2 but allowed under PDF/A-3.
            (
                Self::A2_A | Self::A2_B | Self::A2_U | Self::A3_A | Self::A3_B | Self::A3_U,
                ValidationError::EmbeddedFile(EmbedError::Existence, _),
            ) => self == Self::A2_A || self == Self::A2_B || self == Self::A2_U,
            // Forbidden under PDF/A-3 but allowed under PDF/A-2.
            (
                Self::A2_A | Self::A2_B | Self::A2_U | Self::A3_A | Self::A3_B | Self::A3_U,
                ValidationError::EmbeddedFile(
                    EmbedError::MissingDate
                    | EmbedError::MissingDescription
                    | EmbedError::MissingMimeType,
                    _,
                ),
            ) => self == Self::A3_A || self == Self::A3_B || self == Self::A3_U,
            // Forbidden under PDF/A-2 and PDF/A-3 accessible profiles.
            (
                Self::A2_A | Self::A2_B | Self::A2_U | Self::A3_A | Self::A3_B | Self::A3_U,
                ValidationError::UnicodePrivateArea(_, _, _, _)
                | ValidationError::NoDocumentLanguage
                | ValidationError::MissingAltText(_)
                | ValidationError::MissingAnnotationAltText(_)
                | ValidationError::MissingTagging,
            ) => self == Self::A2_A || self == Self::A3_A,
            // Forbidden under PDF/A-2 and PDF/A-3 accessible and Unicode profiles.
            (
                Self::A2_A | Self::A2_B | Self::A2_U | Self::A3_A | Self::A3_B | Self::A3_U,
                ValidationError::NoCodepointMapping(_, _, _)
                | ValidationError::InvalidCodepointMapping(_, _, _, _),
            ) => {
                self == Self::A2_A || self == Self::A2_U || self == Self::A3_A || self == Self::A3_U
            }

            // Forbidden under all PDF/A-4 profiles.
            (
                Self::A4 | Self::A4F | Self::A4E,
                ValidationError::MissingCMYKProfile
                | ValidationError::InconsistentSeparationFallback(_)
                | ValidationError::ContainsNotDefGlyph(_, _, _)
                | ValidationError::NoCodepointMapping(_, _, _)
                | ValidationError::InvalidCodepointMapping(_, _, _, _)
                | ValidationError::UnicodePrivateArea(_, _, _, _)
                | ValidationError::RestrictedLicense(_)
                | ValidationError::MissingDocumentDate
                | ValidationError::ImageInterpolation(_)
                | ValidationError::EmbeddedPDF(_),
            ) => true,
            // Allowed under all PDF/A-4 profiles.
            (
                Self::A4 | Self::A4F | Self::A4E,
                ValidationError::TooLongString
                | ValidationError::TooLongName
                | ValidationError::TooLongArray
                | ValidationError::TooLongDictionary
                | ValidationError::TooLargeFloat
                | ValidationError::TooManyIndirectObjects
                | ValidationError::TooHighQNestingLevel
                | ValidationError::ContainsPostScript(_)
                | ValidationError::NoDocumentLanguage
                | ValidationError::NoDocumentTitle
                | ValidationError::MissingAltText(_)
                | ValidationError::MissingHeadingTitle
                | ValidationError::MissingDocumentOutline
                | ValidationError::MissingAnnotationAltText(_)
                | ValidationError::Transparency(_)
                | ValidationError::EmbeddedFile(
                    EmbedError::MissingDate | EmbedError::MissingMimeType,
                    _,
                )
                | ValidationError::MissingTagging
                | ValidationError::RequiresNewerPdfVersion(
                    VersionedFeature::HeaderFooterArtifactSubtypes
                    | VersionedFeature::StructureOrderTabbing
                    | VersionedFeature::TableHeaderScope,
                    _,
                ),
            ) => false,
            // Forbidden under PDF/A-4 but allowed under other PDF/A-4 profiles.
            (
                Self::A4 | Self::A4F | Self::A4E,
                ValidationError::EmbeddedFile(EmbedError::Existence, _),
            ) => self == Self::A4,
            // Allowed under PDF/A-4 but forbidden under other profiles.
            (
                Self::A4 | Self::A4F | Self::A4E,
                ValidationError::EmbeddedFile(EmbedError::MissingDescription, _),
            ) => self == Self::A4,
        }
    }

    fn requires_codepoint_mappings(self) -> bool {
        match self {
            Self::A1_A
            | Self::A2_A
            | Self::A2_U
            | Self::A3_A
            | Self::A3_U
            | Self::A4
            | Self::A4F
            | Self::A4E => true,
            Self::A1_B | Self::A2_B | Self::A3_B => false,
        }
    }

    fn requires_no_device_cs(self) -> bool {
        match self {
            Self::A1_A
            | Self::A1_B
            | Self::A2_A
            | Self::A2_B
            | Self::A2_U
            | Self::A3_A
            | Self::A3_B
            | Self::A3_U
            | Self::A4
            | Self::A4F
            | Self::A4E => true,
        }
    }

    fn requires_annotation_flags(self) -> bool {
        match self {
            Self::A1_A
            | Self::A1_B
            | Self::A2_A
            | Self::A2_B
            | Self::A2_U
            | Self::A3_A
            | Self::A3_B
            | Self::A3_U
            | Self::A4
            | Self::A4F
            | Self::A4E => true,
        }
    }

    fn requires_tagging(self) -> bool {
        match self {
            Self::A1_A | Self::A2_A | Self::A3_A => true,
            Self::A1_B
            | Self::A2_B
            | Self::A2_U
            | Self::A3_B
            | Self::A3_U
            | Self::A4
            | Self::A4F
            | Self::A4E => false,
        }
    }

    fn requires_xmp_metadata(self) -> bool {
        match self {
            Self::A1_A
            | Self::A1_B
            | Self::A2_A
            | Self::A2_B
            | Self::A2_U
            | Self::A3_A
            | Self::A3_B
            | Self::A3_U
            | Self::A4
            | Self::A4F
            | Self::A4E => true,
        }
    }

    fn requires_xmp_metadata_extension_schema(self) -> bool {
        match self {
            Self::A1_A
            | Self::A1_B
            | Self::A2_A
            | Self::A2_B
            | Self::A2_U
            | Self::A3_A
            | Self::A3_B
            | Self::A3_U => true,
            // Clause 6.7.2.3 of PDF/A-4 recommends ("should") a RELAX NG
            // definition of its metadata contents to be embedded as an
            // associated file. It no longer uses the inline schema definition
            // using the "pdfaSchema" namespaces for extension schemata.
            Self::A4 | Self::A4F | Self::A4E => false,
        }
    }

    fn prohibits_instance_id_in_xmp_metadata(self) -> bool {
        match self {
            Self::A1_A | Self::A1_B => true,
            Self::A2_A
            | Self::A2_B
            | Self::A2_U
            | Self::A3_A
            | Self::A3_B
            | Self::A3_U
            | Self::A4
            | Self::A4F
            | Self::A4E => false,
        }
    }

    fn requires_file_provenance_information(self) -> bool {
        match self {
            Self::A1_A
            | Self::A1_B
            | Self::A2_A
            | Self::A2_B
            | Self::A2_U
            | Self::A3_A
            | Self::A3_B
            | Self::A3_U
            | Self::A4
            | Self::A4F
            | Self::A4E => true,
        }
    }

    fn prohibits_info_dict(self) -> bool {
        match self {
            Self::A1_A
            | Self::A1_B
            | Self::A2_A
            | Self::A2_B
            | Self::A2_U
            | Self::A3_A
            | Self::A3_B
            | Self::A3_U => false,
            Self::A4 | Self::A4F | Self::A4E => true,
        }
    }

    fn requires_binary_header(self) -> bool {
        match self {
            Self::A1_A
            | Self::A1_B
            | Self::A2_A
            | Self::A2_B
            | Self::A2_U
            | Self::A3_A
            | Self::A3_B
            | Self::A3_U
            | Self::A4
            | Self::A4F
            | Self::A4E => true,
        }
    }

    fn requires_embedded_files_when_empty(self) -> bool {
        match self {
            Self::A1_A
            | Self::A1_B
            | Self::A2_A
            | Self::A2_B
            | Self::A2_U
            | Self::A3_A
            | Self::A3_B
            | Self::A3_U
            | Self::A4
            | Self::A4E => false,
            Self::A4F => true,
        }
    }

    /// Whether this standard explicitly specifies the `/AF` key.
    ///
    /// The `/AF` key may be supported by the underlying PDF version instead:
    /// Starting at PDF 2.0, the key is specified by ISO 32000 and does not need
    /// to be added by PDF/A.
    fn specifies_associated_files(self) -> bool {
        match self {
            Self::A3_A | Self::A3_B | Self::A3_U => true,
            Self::A1_A
            | Self::A1_B
            | Self::A2_A
            | Self::A2_B
            | Self::A2_U
            | Self::A4
            | Self::A4F
            | Self::A4E => false,
        }
    }

    fn output_intent(self) -> OutputIntentSubtype<'static> {
        match self {
            Self::A1_A
            | Self::A1_B
            | Self::A2_A
            | Self::A2_B
            | Self::A2_U
            | Self::A3_A
            | Self::A3_B
            | Self::A3_U
            | Self::A4
            | Self::A4F
            | Self::A4E => OutputIntentSubtype::PDFA,
        }
    }

    fn write_xmp(self, xmp: &mut XmpWriter) {
        match self {
            Self::A1_A => {
                xmp.pdfa_part(1);
                xmp.pdfa_conformance("A");
            }
            Self::A1_B => {
                xmp.pdfa_part(1);
                xmp.pdfa_conformance("B");
            }
            Self::A2_A => {
                xmp.pdfa_part(2);
                xmp.pdfa_conformance("A");
            }
            Self::A2_B => {
                xmp.pdfa_part(2);
                xmp.pdfa_conformance("B");
            }
            Self::A2_U => {
                xmp.pdfa_part(2);
                xmp.pdfa_conformance("U");
            }
            Self::A3_A => {
                xmp.pdfa_part(3);
                xmp.pdfa_conformance("A");
            }
            Self::A3_B => {
                xmp.pdfa_part(3);
                xmp.pdfa_conformance("B");
            }
            Self::A3_U => {
                xmp.pdfa_part(3);
                xmp.pdfa_conformance("U");
            }
            Self::A4 => {
                xmp.pdfa_part(4);
                xmp.pdfa_rev(2020);
            }
            Self::A4F => {
                xmp.pdfa_part(4);
                xmp.pdfa_rev(2020);
                xmp.pdfa_conformance("F");
            }
            Self::A4E => {
                xmp.pdfa_part(4);
                xmp.pdfa_rev(2020);
                xmp.pdfa_conformance("E");
            }
        }
    }

    fn write_xmp_extension_schema_description(
        self,
        extension_schemas: &mut PdfAExtSchemasWriter<'_, '_>,
    ) {
        if !self.requires_xmp_metadata_extension_schema() {
            return;
        }

        extension_schemas
            .xmp_media_management()
            .properties()
            .describe_instance_id();
        extension_schemas.pdf().properties().describe_all();
    }

    /// Returns a human-readable string representation of the conformance level.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A1_A => "PDF/A-1a",
            Self::A1_B => "PDF/A-1b",
            Self::A2_A => "PDF/A-2a",
            Self::A2_B => "PDF/A-2b",
            Self::A2_U => "PDF/A-2u",
            Self::A3_A => "PDF/A-3a",
            Self::A3_B => "PDF/A-3b",
            Self::A3_U => "PDF/A-3u",
            Self::A4 => "PDF/A-4",
            Self::A4F => "PDF/A-4f",
            Self::A4E => "PDF/A-4e",
        }
    }

    /// Minimum PDF version required to use this standard, if any.
    pub const fn min(self) -> Option<PdfVersion> {
        match self {
            // PDF/A-1 through 3 require XMP `/Metadata` streams, which require PDF 1.4.
            Self::A1_A | Self::A1_B => Some(PdfVersion::Pdf14),
            Self::A2_A | Self::A2_B | Self::A2_U => Some(PdfVersion::Pdf14),
            Self::A3_A | Self::A3_B | Self::A3_U => Some(PdfVersion::Pdf14),
            Self::A4 | Self::A4F | Self::A4E => Some(PdfVersion::Pdf20),
        }
    }

    /// Maximum PDF version this standard can be used with.
    pub const fn max(self) -> PdfVersion {
        match self {
            Self::A1_A | Self::A1_B => PdfVersion::Pdf14,
            Self::A2_A | Self::A2_B | Self::A2_U | Self::A3_A | Self::A3_B | Self::A3_U => {
                PdfVersion::Pdf17
            }
            Self::A4 | Self::A4F | Self::A4E => PdfVersion::Pdf20,
        }
    }
}

/// A validator for exporting PDF documents to a specific subset of PDF.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum Accessibility {
    /// The validator for the PDF/UA-1 standard.
    ///
    /// **Requirements**:
    ///
    /// General:
    /// - All real content should be tagged accordingly.
    /// - All artifacts should be marked accordingly.
    /// - The tag tree should reflect the logical reading order of the
    ///   document.
    /// - Information should not be conveyed by contrast, color, format
    ///   or layout.
    /// - All "best practice" notes in [`TagKind`] need to be complied with.
    ///
    /// Text:
    /// - You should make use of the `Alt`, `ActualText`, `Lang` and `Expansion` attributes
    ///   whenever possible.
    /// - Usually, you can provide an empty string as `Lang` to indicate that a language is unknown.
    ///   You should not do that in PDF/UA.
    /// - Stretchable characters (such as brackets, which often consist of several glyphs)
    ///   should be marked accordingly with `ActualText`.
    ///
    ///  Graphics:
    /// - Graphics should be tagged as figures (unless they are an artifact).
    /// - Graphics need to be followed by a caption.
    /// - Graphics that possess semantic values only in combination with other graphics
    ///   should be tagged with a single Figure tag for each figure.
    /// - If a more accessible representation exists, it should be used over graphics.
    ///
    /// Headings:
    /// - Headings should be tagged as such.
    /// - For not strongly structured documents, H1 should be the first
    ///   heading.
    ///
    /// Tables:
    /// - Tables should include headers and be tagged accordingly.
    /// - Tables should only be used to represent content within logical row/column relationship.
    ///
    /// Lists:
    /// - List items should be tagged with Li tags, if necessary also with
    ///   Lbl and LBody tags.
    /// - Lists should only be used when the content is intended to be read
    ///   as a list.
    ///
    /// Mathematical expressions:
    /// - All mathematical expressions should be enclosed with
    ///   a `Formula` tag.
    ///
    /// Headers and footers:
    /// - Headers and footers should be marked as corresponding
    ///   artifacts.
    ///
    /// Notes and references:
    /// - Footnotes, endnotes, note labels and references should be
    ///   tagged accordingly and use tagged annotations.
    /// - Footnotes and end notes should use the `Note` tag.
    ///
    /// Navigation:
    /// - The document must contain an outline, and it should reflect
    ///   the reading order of the document.
    /// - Page labels should be semantically appropriate.
    ///
    /// Annotations:
    /// - Annotations should be present in the tag tree in the correct
    ///   reading order.
    ///
    /// Fonts:
    /// - You should only use fonts that are legally embeddable in a file for unlimited,
    ///   universal rendering.
    ///
    /// [`TagKind`]: crate::interchange::tagging::TagKind
    UA1,
}

impl Accessibility {
    fn prohibits(self, error: &ValidationError) -> bool {
        match (self, error) {
            // PDF/X-specific errors: PDF/UA normalizes mixed gradient color
            // spaces and never makes use of an external output profile, but it
            // permits RGB, annotations, and pages without a TrimBox/ArtBox.
            (
                _,
                ValidationError::MixedGradientColorSpaces(_)
                | ValidationError::ExternalOutputProfileUnsupportedByValidator,
            ) => true,
            (
                _,
                ValidationError::ContainsRgb(_)
                | ValidationError::MissingTrimOrArtBox(_, _)
                | ValidationError::MissingTrimBox(_, _)
                | ValidationError::BothTrimAndArtBox(_, _)
                | ValidationError::PageBoxNotNested(_, _)
                | ValidationError::DegeneratePageBox(_, _)
                | ValidationError::PageBoxTooLarge(_, _)
                | ValidationError::ContainsAnnotation(_)
                | ValidationError::ContainsAction(_)
                | ValidationError::AnnotationInsidePrintArea(_)
                | ValidationError::AnnotationContainsRgb(_)
                | ValidationError::OutputIntentColorSpaceMismatch(_)
                | ValidationError::IncompatibleOutputProfileVersion(_)
                | ValidationError::InvalidOutputProfileDeviceClass(_)
                | ValidationError::InvalidOutputProfileColorSpace(_)
                | ValidationError::MissingExternalOutputProfile,
            ) => false,
            (
                Self::UA1,
                ValidationError::ContainsNotDefGlyph(_, _, _)
                | ValidationError::NoCodepointMapping(_, _, _)
                | ValidationError::InvalidCodepointMapping(_, _, _, _)
                | ValidationError::RestrictedLicense(_)
                | ValidationError::NoDocumentTitle
                | ValidationError::MissingAltText(_)
                | ValidationError::MissingHeadingTitle
                | ValidationError::MissingDocumentOutline
                | ValidationError::MissingAnnotationAltText(_)
                | ValidationError::EmbeddedFile(EmbedError::MissingDescription, _)
                | ValidationError::MissingTagging
                | ValidationError::EmbeddedPDF(_)
                | ValidationError::RequiresNewerPdfVersion(
                    VersionedFeature::HeaderFooterArtifactSubtypes
                    | VersionedFeature::StructureOrderTabbing
                    | VersionedFeature::TableHeaderScope,
                    _,
                ),
            ) => true,
            (
                Self::UA1,
                ValidationError::TooLongString
                | ValidationError::TooLongName
                | ValidationError::TooLongArray
                | ValidationError::TooLongDictionary
                | ValidationError::TooLargeFloat
                | ValidationError::TooManyIndirectObjects
                | ValidationError::TooHighQNestingLevel
                | ValidationError::ContainsPostScript(_)
                | ValidationError::MissingCMYKProfile
                | ValidationError::InconsistentSeparationFallback(_)
                | ValidationError::UnicodePrivateArea(_, _, _, _)
                | ValidationError::NoDocumentLanguage
                | ValidationError::Transparency(_)
                | ValidationError::ImageInterpolation(_)
                | ValidationError::EmbeddedFile(
                    EmbedError::Existence | EmbedError::MissingDate | EmbedError::MissingMimeType,
                    _,
                )
                | ValidationError::MissingDocumentDate,
            ) => false,
        }
    }

    fn requires_codepoint_mappings(self) -> bool {
        match self {
            Self::UA1 => true,
        }
    }

    fn requires_display_doc_title(self) -> bool {
        match self {
            Self::UA1 => true,
        }
    }

    const fn requires_tagging(self) -> bool {
        true
    }

    fn requires_xmp_metadata(self) -> bool {
        match self {
            Self::UA1 => true,
        }
    }

    fn write_xmp(self, xmp: &mut XmpWriter) {
        match self {
            Self::UA1 => {
                xmp.pdfua_part(1);
            }
        }
    }

    fn write_xmp_extension_schema_description(
        self,
        extension_schemas: &mut PdfAExtSchemasWriter<'_, '_>,
    ) {
        // Needs to be updated if [`Self::write_xmp`] gains more properties.
        extension_schemas.pdfua_id().properties().describe_part();
    }

    /// Returns a human-readable string representation of the accessibility level.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UA1 => "PDF/UA-1",
        }
    }

    /// Minimum PDF version required to use this standard, if any.
    pub const fn min(self) -> Option<PdfVersion> {
        match self {
            // PDF/UA-1 requires Tagged PDF and XMP `/Metadata` streams, which both require PDF 1.4.
            Self::UA1 => Some(PdfVersion::Pdf14),
        }
    }

    /// Maximum PDF version this standard can be used with.
    pub const fn max(self) -> PdfVersion {
        match self {
            // PDF/UA-1 is specified against PDF 1.7.
            Self::UA1 => PdfVersion::Pdf17,
        }
    }
}

/// A PDF/X conformance level for reliable prepress data exchange.
///
/// PDF/X is orthogonal to [`Archival`] (PDF/A) and [`Accessibility`] (PDF/UA):
/// a document may target a PDF/X standard on its own or in combination with a
/// PDF/A one (for example PDF/A-2b + PDF/X-4). All PDF/X standards require an
/// output intent (embedded, or referenced externally for the `p` variants), a
/// creation date, and trapping information. Every page must carry a TrimBox or
/// ArtBox — a TrimBox specifically for PDF/X-6/-6p.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum Prepress {
    /// The validator for the PDF/X-1a:2003 standard (ISO 15930-4).
    ///
    /// Based on PDF 1.4.
    ///
    /// **Requirements**:
    /// - A CMYK ICC profile must be provided via the `cmyk_profile` setting.
    /// - Only CMYK, grayscale, and Separation colors may be used (no RGB).
    /// - No transparency is allowed.
    /// - No annotations are allowed (krilla only supports Link annotations,
    ///   which are not permitted by PDF/X-1a).
    /// - Every page must have a TrimBox or ArtBox set.
    /// - A document title must be set via metadata.
    /// - A creation date must be set via metadata.
    X1A,
    /// The validator for the PDF/X-3:2003 standard (ISO 15930-6).
    ///
    /// Based on PDF 1.4.
    ///
    /// **Requirements**:
    /// - A printer/output ICC profile must be provided via the `cmyk_profile`
    ///   setting for the embedded PDF/X output intent.
    /// - No transparency is allowed.
    /// - Every page must have a TrimBox or ArtBox set.
    /// - A document title must be set via metadata.
    /// - A creation date must be set via metadata.
    X3,
    /// The validator for the PDF/X-4 standard (ISO 15930-7).
    ///
    /// Based on PDF 1.6.
    ///
    /// **Requirements**:
    /// - A printer/output ICC profile must be provided via the `cmyk_profile`
    ///   setting for the embedded PDF/X output intent.
    /// - Every page must have a TrimBox or ArtBox set.
    /// - A document title must be set via metadata.
    /// - A creation date must be set via metadata.
    X4,
    /// The validator for the PDF/X-4p standard (ISO 15930-7).
    ///
    /// Like PDF/X-4, but the output intent ICC profile is referenced
    /// externally instead of being embedded. Based on PDF 1.6.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/X-4.
    /// - The `external_output_profile` setting must be provided.
    X4P,
    /// The validator for the PDF/X-6 standard (ISO 15930-9).
    ///
    /// Based on PDF 2.0, which relaxes several of the earlier levels'
    /// restrictions: a document title is not required, GoTo/URI actions and
    /// annotations inside the print area are permitted, and there is no Info
    /// dictionary (the metadata is carried in XMP).
    ///
    /// **Requirements**:
    /// - A printer/output ICC profile must be provided via the `cmyk_profile`
    ///   setting for the embedded PDF/X output intent.
    /// - Every page must have a TrimBox set (a coexisting ArtBox is permitted).
    /// - A creation date must be set via metadata.
    X6,
    /// The validator for the PDF/X-6p standard (ISO 15930-9).
    ///
    /// Like PDF/X-6, but the output intent ICC profile is referenced
    /// externally instead of being embedded. Based on PDF 2.0.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/X-6.
    /// - The `external_output_profile` setting must be provided.
    X6P,
}

impl Prepress {
    fn prohibits(self, error: &ValidationError) -> bool {
        match (self, error) {
            // Forbidden by every PDF/X standard.
            (
                _,
                ValidationError::MissingCMYKProfile
                | ValidationError::ContainsNotDefGlyph(_, _, _)
                | ValidationError::RestrictedLicense(_)
                | ValidationError::MissingDocumentDate
                | ValidationError::EmbeddedPDF(_)
                | ValidationError::MixedGradientColorSpaces(_)
                | ValidationError::MissingTrimOrArtBox(_, _)
                | ValidationError::PageBoxNotNested(_, _)
                | ValidationError::DegeneratePageBox(_, _)
                | ValidationError::OutputIntentColorSpaceMismatch(_)
                | ValidationError::IncompatibleOutputProfileVersion(_)
                | ValidationError::InvalidOutputProfileDeviceClass(_)
                | ValidationError::InvalidOutputProfileColorSpace(_),
            ) => true,
            // A document title is required by PDF/X-1a/-3 (Info `Title`),
            // PDF/X-4/-4p (ISO 15930-7: `dc:title` in the mandatory metadata
            // set). PDF/X-6/-6p (ISO 15930-9 §6.11) do not require it.
            (Self::X1A | Self::X3 | Self::X4 | Self::X4P, ValidationError::NoDocumentTitle) => true,
            (Self::X6 | Self::X6P, ValidationError::NoDocumentTitle) => false,
            // PDF/X-6/-6p (ISO 15930-9 §6.9) require a TrimBox specifically.
            (Self::X6 | Self::X6P, ValidationError::MissingTrimBox(_, _)) => true,
            (
                Self::X1A | Self::X3 | Self::X4 | Self::X4P,
                ValidationError::MissingTrimBox(_, _),
            ) => false,
            // A page carries exactly one of TrimBox/ArtBox under the PDF
            // 1.4/1.6-based levels (ISO 15930-4/-6 §6.8, ISO 15930-7 §6.12:
            // "a TrimBox or an ArtBox, but not both"). PDF/X-6/-6p drop the
            // "but not both" clause and permit a coexisting ArtBox.
            (
                Self::X1A | Self::X3 | Self::X4 | Self::X4P,
                ValidationError::BothTrimAndArtBox(_, _),
            ) => true,
            (Self::X6 | Self::X6P, ValidationError::BothTrimAndArtBox(_, _)) => false,
            // Allowed by every PDF/X standard. PDF/X does not constrain
            // tagging, accessibility, or codepoint mappings, so the
            // accessibility-related errors and `RequiresNewerPdfVersion` are
            // never raised.
            (
                _,
                ValidationError::NoCodepointMapping(_, _, _)
                | ValidationError::InvalidCodepointMapping(_, _, _, _)
                | ValidationError::UnicodePrivateArea(_, _, _, _)
                | ValidationError::NoDocumentLanguage
                | ValidationError::MissingAltText(_)
                | ValidationError::MissingHeadingTitle
                | ValidationError::MissingDocumentOutline
                | ValidationError::MissingAnnotationAltText(_)
                | ValidationError::ImageInterpolation(_)
                | ValidationError::MissingTagging
                | ValidationError::RequiresNewerPdfVersion(_, _),
            ) => false,

            // The external output profile is required by the -p variants and
            // unsupported by all others.
            (_, ValidationError::MissingExternalOutputProfile) => {
                matches!(self, Self::X4P | Self::X6P)
            }
            (_, ValidationError::ExternalOutputProfileUnsupportedByValidator) => {
                !matches!(self, Self::X4P | Self::X6P)
            }

            // PDF/X-1a and PDF/X-3 are based on PDF 1.4, so its structural
            // limits apply and transparency is forbidden. PDF/X-4 (PDF 1.6) and
            // PDF/X-6 (PDF 2.0) relax these. (A document title is required by
            // every PDF/X standard and is handled above.)
            (
                Self::X1A | Self::X3,
                ValidationError::TooLongString
                | ValidationError::TooLongName
                | ValidationError::TooLongArray
                | ValidationError::TooLongDictionary
                | ValidationError::TooLargeFloat
                | ValidationError::TooManyIndirectObjects
                | ValidationError::TooHighQNestingLevel
                | ValidationError::ContainsPostScript(_)
                | ValidationError::Transparency(_),
            ) => true,
            // PDF/X-4/-4p are based on PDF 1.6, whose architectural limits
            // (PDF Reference Table C.1) ISO 15930-7 §6.25 forbids violating. The
            // string, name, indirect-object and q/Q-nesting caps survive into
            // PDF 1.6 and are enforced (mirroring krilla's PDF/A-2/-3 handling);
            // the array/dictionary/float caps were PDF 1.4-only and are relaxed.
            (
                Self::X4 | Self::X4P,
                ValidationError::TooLongString
                | ValidationError::TooLongName
                | ValidationError::TooManyIndirectObjects
                | ValidationError::TooHighQNestingLevel,
            ) => true,
            (
                Self::X4 | Self::X4P,
                ValidationError::TooLongArray
                | ValidationError::TooLongDictionary
                | ValidationError::TooLargeFloat
                | ValidationError::ContainsPostScript(_)
                | ValidationError::Transparency(_),
            ) => false,
            // PDF/X-6/-6p are based on PDF 2.0 (ISO 32000-2), which defines no
            // architectural-limits annex; they also permit transparency and
            // PostScript calculator functions.
            (
                Self::X6 | Self::X6P,
                ValidationError::TooLongString
                | ValidationError::TooLongName
                | ValidationError::TooLongArray
                | ValidationError::TooLongDictionary
                | ValidationError::TooLargeFloat
                | ValidationError::TooManyIndirectObjects
                | ValidationError::TooHighQNestingLevel
                | ValidationError::ContainsPostScript(_)
                | ValidationError::Transparency(_),
            ) => false,

            // Separation fallback consistency is enforced by every PDF/X
            // standard: a colorant name must map to a single tint transform,
            // independent of whether the fallback color is RGB or CMYK.
            (_, ValidationError::InconsistentSeparationFallback(_)) => true,

            // PDF/X-1a is CMYK-only and forbids RGB and annotations; the other
            // standards permit them (annotations subject to the positioning and
            // color rules below).
            (_, ValidationError::ContainsRgb(_)) => self == Self::X1A,
            (_, ValidationError::ContainsAnnotation(_)) => self == Self::X1A,
            // PDF/X-1a/-3/-4/-4p forbid interactive actions (ISO 15930-4 §6.14,
            // ISO 15930-6 §6.14, ISO 15930-7 §6.18: "shall not include Actions
            // or JavaScripts"). PDF/X-6/-6p (ISO 15930-9 §6.14) permit GoTo and
            // URI actions — the only kinds krilla emits — so they are allowed.
            // A link to an in-document destination carries no action regardless.
            (Self::X1A | Self::X3 | Self::X4 | Self::X4P, ValidationError::ContainsAction(_)) => {
                true
            }
            (Self::X6 | Self::X6P, ValidationError::ContainsAction(_)) => false,
            // The 14400-unit page-size limit is a PDF 1.4 limit, so it applies
            // to the PDF 1.4-based PDF/X-1a and PDF/X-3 only.
            (_, ValidationError::PageBoxTooLarge(_, _)) => matches!(self, Self::X1A | Self::X3),
            // PDF/X-1a and PDF/X-3 (PDF 1.4 blind exchange) forbid embedded
            // files; PDF/X-4/X-6 (PDF 1.6/2.0) permit them.
            (Self::X1A | Self::X3, ValidationError::EmbeddedFile(EmbedError::Existence, _)) => true,
            (_, ValidationError::EmbeddedFile(_, _)) => false,
            // PDF/X-3/-4/-4p require annotations to lie wholly outside the print
            // area (ISO 15930-6 §6.13, ISO 15930-7 §6.17). PDF/X-1a forbids
            // annotations outright (ContainsAnnotation); PDF/X-6/-6p (ISO
            // 15930-9 §6.12) permit annotations inside the visible area, so the
            // positional rule does not apply to either.
            (_, ValidationError::AnnotationInsidePrintArea(_)) => {
                matches!(self, Self::X3 | Self::X4 | Self::X4P)
            }
            // An annotation border colour must still be characterized by the
            // output intent at every level that permits annotations (an
            // uncharacterized DeviceRGB border fails this under a non-RGB
            // intent). PDF/X-1a forbids annotations, so it never reaches here.
            (_, ValidationError::AnnotationContainsRgb(_)) => self != Self::X1A,
        }
    }

    fn requires_codepoint_mappings(self) -> bool {
        false
    }

    fn requires_no_device_cs(self) -> bool {
        // PDF/X-1a forbids ICCBased color spaces, so device color spaces must
        // be emitted directly. All other PDF/X standards permit ICCBased and
        // benefit from device-independent substitution.
        self != Self::X1A
    }

    fn requires_annotation_flags(self) -> bool {
        // PDF/X-1a forbids annotations entirely, so annotation flags are moot.
        self != Self::X1A
    }

    const fn requires_tagging(self) -> bool {
        false
    }

    fn requires_xmp_metadata(self) -> bool {
        true
    }

    fn requires_binary_header(self) -> bool {
        true
    }

    fn requires_external_output_profile(self) -> bool {
        matches!(self, Self::X4P | Self::X6P)
    }

    fn forbids_annotations(self) -> bool {
        self == Self::X1A
    }

    fn forbids_actions(self) -> bool {
        // PDF/X-1a/-3/-4/-4p forbid Actions and JavaScripts (ISO 15930-4 §6.14,
        // ISO 15930-6 §6.14, ISO 15930-7 §6.18). PDF/X-6/-6p (ISO 15930-9 §6.14)
        // permit GoTo/URI actions, the only kinds krilla emits.
        matches!(self, Self::X1A | Self::X3 | Self::X4 | Self::X4P)
    }

    fn requires_xmp_metadata_date(self) -> bool {
        matches!(self, Self::X4 | Self::X4P | Self::X6 | Self::X6P)
    }

    fn requires_xmp_version_id(self) -> bool {
        matches!(self, Self::X4 | Self::X4P | Self::X6 | Self::X6P)
    }

    /// The `GTS_PDFXVersion` identification string for this standard.
    ///
    /// The `p` variants use a distinct identifier (`PDF/X-4p`, `PDF/X-6p`),
    /// matching the GTS_PDFXVersion values defined by ISO 15930-7/-9 for the
    /// externally-referenced-profile conformance levels.
    fn gts_pdfx_version_string(self) -> &'static str {
        match self {
            Self::X1A => "PDF/X-1a:2003",
            Self::X3 => "PDF/X-3:2003",
            Self::X4 => "PDF/X-4",
            Self::X4P => "PDF/X-4p",
            Self::X6 => "PDF/X-6",
            Self::X6P => "PDF/X-6p",
        }
    }

    fn output_intent(self) -> OutputIntentSubtype<'static> {
        OutputIntentSubtype::PDFX
    }

    fn write_xmp(self, xmp: &mut XmpWriter) {
        xmp.pdfx_version(self.gts_pdfx_version_string());
        // ISO 15930-9 §6.11.3 Table 3 marks the `pdfxid:rev` property (the
        // four-digit year of the ISO 15930 revision) as required for PDF/X-6/-6p.
        // The earlier levels (ISO 15930-4/-6/-7) define no `rev` field.
        if matches!(self, Self::X6 | Self::X6P) {
            xmp.element("rev", Namespace::PdfXId).value(2020);
        }
    }

    fn write_xmp_extension_schema_description(
        self,
        extension_schemas: &mut PdfAExtSchemasWriter<'_, '_>,
    ) {
        // This inline extension-schema description is emitted only when paired
        // with a PDF/A level that uses inline schemas (PDF/A-1/2/3, i.e.
        // alongside PDF/X-1a/-3/-4). PDF/X-6/-6p only version-overlap with
        // PDF/A-4, which emits no inline schema, so the `pdfxid:rev` property
        // (written to XMP by `write_xmp` for X-6/-6p) needs no description here.
        let mut schema = extension_schemas.add_schema();
        schema.namespace(Namespace::PdfXId);
        schema
            .properties()
            .add_property()
            .category(true)
            .description("Version of the PDF/X standard to which the document conforms")
            .name("GTS_PDFXVersion")
            .value_type("Text");
    }

    /// Returns a human-readable string representation of the conformance level.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X1A => "PDF/X-1a",
            Self::X3 => "PDF/X-3",
            Self::X4 => "PDF/X-4",
            Self::X4P => "PDF/X-4p",
            Self::X6 => "PDF/X-6",
            Self::X6P => "PDF/X-6p",
        }
    }

    /// Minimum PDF version required to use this standard, if any.
    pub const fn min(self) -> Option<PdfVersion> {
        Some(match self {
            // PDF/X-1a:2003 and PDF/X-3:2003 are based on PDF 1.4.
            Self::X1A | Self::X3 => PdfVersion::Pdf14,
            // PDF/X-4 is based on PDF 1.6.
            Self::X4 | Self::X4P => PdfVersion::Pdf16,
            // PDF/X-6 is based on PDF 2.0.
            Self::X6 | Self::X6P => PdfVersion::Pdf20,
        })
    }

    /// Maximum PDF version this standard can be used with.
    pub const fn max(self) -> PdfVersion {
        match self {
            Self::X1A | Self::X3 => PdfVersion::Pdf14,
            Self::X4 | Self::X4P => PdfVersion::Pdf16,
            Self::X6 | Self::X6P => PdfVersion::Pdf20,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct ValidationStore {
    /// Maps from the name of a Separation colorant to a hash of its fallback
    /// color. Used to track that a name is only ever matched with a single
    /// fallback color. Since Krilla manages the `tintTransform` functions,
    /// those are always equivalent.
    separation_fallback_map: HashMap<SeparationColorant, RegularColor>,
}

impl ValidationStore {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Register a colorant and its fallback and raise an error if it already
    /// exists.
    pub(crate) fn validate_separation(
        &mut self,
        separation: &SeparationSpace,
    ) -> Result<(), ValidationError> {
        if self
            .separation_fallback_map
            .entry(separation.colorant.clone())
            .or_insert(separation.fallback)
            == &separation.fallback
        {
            Ok(())
        } else {
            Err(ValidationError::InconsistentSeparationFallback(
                separation.colorant.clone(),
            ))
        }
    }
}
