//! Exporting with a specific PDF conformance level.
//!
//! PDF defines a number of additional conformance level that restrict the features of PDF that
//! can be used to a specific subset. Currently, krilla only supports some PDF/A conformance levels,
//! although more are planned for the future.
//!
//! You can use a [`Validator`] by setting the `validator` attribute of the [`SerializeSettings`]
//! you create the document with. There are three important aspects that play into this:
//! - krilla will internally write the file in a way that conforms to the given standard, i.e.
//!   by settings appropriate metadata. This happens under-the-hood and is completely abstracted
//!   away from the user.
//! - For aspects that are out of control of krilla and dependent on the input, krilla will perform
//!   a validation that the input is compatible with the standard. krilla will record all violations,
//!   and when calling `document.finish()`, in case there is at least one violation, krilla will
//!   return them as an error, instead of returning the finished document. See [`ValidationError`].
//! - Finally, some standards have requirements that cannot possibly be validated by krilla, as
//!   they are semantic in nature. It is upon your, as a user of that library, to ensure that those
//!   requirements are fulfilled.
//!   You can find them under **Requirements** for each [`Validator`].
//!
//! [`SerializeSettings`]: crate::SerializeSettings

use crate::embed::EmbedError;
use crate::font::Font;
use crate::version::PdfVersion;
use pdf_writer::types::OutputIntentSubtype;
use pdf_writer::Finish;
use skrifa::GlyphId;
use std::fmt::Debug;
use xmp_writer::XmpWriter;

/// An error that occurred during validation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValidationError {
    /// There was a string that was longer than the maximum allowed length (32767).
    ///
    /// Can for example occur if someone set a title or an author that is longer than
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
    /// Occurs if a gradient with spread method `Repeat`/`Reflect` was used.
    ContainsPostScript,
    /// No CMYK ICC profile was provided, even though one is necessary.
    ///
    /// Occurs if the export format requires a device-independent color representation,
    /// and a CMYK color was used in the document.
    MissingCMYKProfile,
    /// The `.notdef` glyph was used, which is forbidden by some export formats.
    ///
    /// Can occur if a glyph could not be found in the font for a corresponding codepoint
    /// in the input text, or if it was explicitly mapped that way.
    ContainsNotDefGlyph,
    /// A glyph was mapped either to the codepoint 0x0, 0xFEFF or 0xFFFE, or no codepoint at all,
    /// which is forbidden by some standards.
    ///
    /// Can occur if those codepoints appeared in the input text, or were explicitly
    /// mapped to that glyph.
    InvalidCodepointMapping(Font, GlyphId),
    /// A glyph was mapped to a codepoint in the Unicode private use area, which is forbidden
    /// by some standards, like for example PDF/A2-A.
    // Note that the standard doesn't explicitly forbid it, but instead requires an ActualText
    // attribute to be present. But we just completely forbid it, for simplicity.
    UnicodePrivateArea(Font, GlyphId),
    /// No document language was set via the metadata, even though it is required
    /// by the standard.
    NoDocumentLanguage,
    /// No title was provided for the document, even though it is required by
    /// the standard.
    NoDocumentTitle,
    /// A figure or formula is missing an alt text.
    MissingAltText,
    /// A heading is missing a title.
    MissingHeadingTitle,
    /// The document does not contain an outline.
    MissingDocumentOutline,
    /// An annotation is missing an alt text.
    MissingAnnotationAltText,
    /// The PDF contains transparency, which is forbidden by some standards (e.g. PDF/A1).
    Transparency,
    /// The PDF contains an image with `interpolate` set to `true`.
    ImageInterpolation,
    /// The PDF contains an embedded file.
    EmbeddedFile(EmbedError),
    /// The PDF contains no tagging.
    MissingTagging,
}

/// A validator for exporting PDF documents to a specific subset of PDF.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[allow(non_camel_case_types)]
pub enum Validator {
    /// A dummy validator, that does not perform any actual validation.
    ///
    /// **Requirements**: -
    #[default]
    None,
    /// The validator for the PDF/A1-A standard.
    ///
    /// **Requirements**:
    ///
    A1_A,
    /// The validator for the PDF/A1-B standard.
    ///
    /// **Requirements**: -
    A1_B,
    /// The validator for the PDF/A2-A standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A2-B.
    /// - You need to follow all requirements outlined in the _Other Notes_ section of the
    ///   [`tagging`] module.
    /// - You need to follow all best practices when using [tags](`crate::tagging::Tag`), as outlined in the documentation
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
    /// [`tagging`]: crate::tagging
    A2_A,
    /// The validator for the PDF/A2-B standard.
    ///
    /// **Requirements**:
    /// - You should only use fonts that are legally embeddable in a file for unlimited,
    ///   universal rendering.
    A2_B,
    /// The validator for the PDF/A2-U standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A2-B
    A2_U,
    /// The validator for the PDF/A3-A standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A2-A
    A3_A,
    /// The validator for the PDF/A3-B standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A2-B
    A3_B,
    /// The validator for the PDF/A3-U standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A2-B
    A3_U,
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
    /// - All "best practice" notes in [`Tag`] need to be complied with.
    ///
    /// Text:
    /// - You should make use of the `Alt`, `ActualText`, `Lang` and `Expansion` attributs
    ///   whenever possible.
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
    /// [`Tag`]: crate::tagging::Tag
    UA1,
}

impl Validator {
    pub(crate) fn prohibits(&self, validation_error: &ValidationError) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => match validation_error {
                ValidationError::TooLongString => true,
                ValidationError::TooLongName => true,
                ValidationError::TooLongArray => true,
                ValidationError::TooLargeFloat => true,
                ValidationError::TooLongDictionary => true,
                ValidationError::TooManyIndirectObjects => true,
                ValidationError::TooHighQNestingLevel => true,
                ValidationError::ContainsPostScript => true,
                ValidationError::MissingCMYKProfile => true,
                ValidationError::ContainsNotDefGlyph => false,
                ValidationError::InvalidCodepointMapping(_, _) => {
                    self.requires_codepoint_mappings()
                }
                ValidationError::UnicodePrivateArea(_, _) => false,
                ValidationError::NoDocumentLanguage => *self == Validator::A1_A,
                ValidationError::NoDocumentTitle => false,
                ValidationError::MissingAltText => false,
                ValidationError::MissingHeadingTitle => false,
                ValidationError::MissingDocumentOutline => false,
                ValidationError::MissingAnnotationAltText => false,
                ValidationError::Transparency => true,
                ValidationError::ImageInterpolation => true,
                // PDF/A1 doesn't strictly forbid, but it disallows the EF key,
                // which we always insert. So we just forbid it overall.
                ValidationError::EmbeddedFile(_) => true,
                ValidationError::MissingTagging => *self == Validator::A1_A,
            },
            Validator::A2_A | Validator::A2_B | Validator::A2_U => match validation_error {
                ValidationError::TooLongString => true,
                ValidationError::TooLongName => true,
                ValidationError::TooLargeFloat => false,
                ValidationError::TooLongArray => false,
                ValidationError::TooLongDictionary => false,
                ValidationError::TooManyIndirectObjects => true,
                ValidationError::TooHighQNestingLevel => true,
                ValidationError::ContainsPostScript => true,
                ValidationError::MissingCMYKProfile => true,
                ValidationError::ContainsNotDefGlyph => true,
                ValidationError::InvalidCodepointMapping(_, _) => {
                    self.requires_codepoint_mappings()
                }
                ValidationError::UnicodePrivateArea(_, _) => *self == Validator::A2_A,
                ValidationError::NoDocumentLanguage => *self == Validator::A2_A,
                ValidationError::NoDocumentTitle => false,
                ValidationError::MissingAltText => false,
                ValidationError::MissingHeadingTitle => false,
                ValidationError::MissingDocumentOutline => false,
                ValidationError::MissingAnnotationAltText => false,
                ValidationError::Transparency => false,
                ValidationError::ImageInterpolation => true,
                // Also not strictly forbidden, but we can't ensure that it is PDF/A2 compliant,
                // so we just forbid it completely.
                ValidationError::EmbeddedFile(_) => true,
                ValidationError::MissingTagging => *self == Validator::A2_A,
            },
            Validator::A3_A | Validator::A3_B | Validator::A3_U => match validation_error {
                ValidationError::TooLongString => true,
                ValidationError::TooLongName => true,
                ValidationError::TooLargeFloat => false,
                ValidationError::TooLongArray => false,
                ValidationError::TooLongDictionary => false,
                ValidationError::TooManyIndirectObjects => true,
                ValidationError::TooHighQNestingLevel => true,
                ValidationError::ContainsPostScript => true,
                ValidationError::MissingCMYKProfile => true,
                ValidationError::ContainsNotDefGlyph => true,
                ValidationError::InvalidCodepointMapping(_, _) => {
                    self.requires_codepoint_mappings()
                }
                ValidationError::UnicodePrivateArea(_, _) => *self == Validator::A3_A,
                ValidationError::NoDocumentLanguage => *self == Validator::A3_A,
                ValidationError::NoDocumentTitle => false,
                ValidationError::MissingAltText => false,
                ValidationError::MissingHeadingTitle => false,
                ValidationError::MissingDocumentOutline => false,
                ValidationError::MissingAnnotationAltText => false,
                ValidationError::Transparency => false,
                ValidationError::ImageInterpolation => true,
                ValidationError::EmbeddedFile(er) => match er {
                    EmbedError::Existence => false,
                    EmbedError::MissingDate => true,
                    EmbedError::MissingDescription => true,
                    EmbedError::MissingMimeType => true,
                },
                ValidationError::MissingTagging => *self == Validator::A3_A,
            },
            Validator::UA1 => match validation_error {
                ValidationError::TooLongString => false,
                ValidationError::TooLargeFloat => false,
                ValidationError::TooLongName => false,
                ValidationError::TooLongArray => false,
                ValidationError::TooLongDictionary => false,
                ValidationError::TooManyIndirectObjects => false,
                ValidationError::TooHighQNestingLevel => false,
                ValidationError::ContainsPostScript => false,
                ValidationError::MissingCMYKProfile => false,
                ValidationError::ContainsNotDefGlyph => true,
                ValidationError::InvalidCodepointMapping(_, _) => {
                    self.requires_codepoint_mappings()
                }
                ValidationError::UnicodePrivateArea(_, _) => false,
                ValidationError::NoDocumentLanguage => false,
                ValidationError::NoDocumentTitle => true,
                ValidationError::MissingAltText => true,
                ValidationError::MissingHeadingTitle => true,
                ValidationError::MissingDocumentOutline => true,
                ValidationError::MissingAnnotationAltText => true,
                ValidationError::Transparency => false,
                ValidationError::ImageInterpolation => false,
                ValidationError::EmbeddedFile(er) => match er {
                    EmbedError::Existence => false,
                    EmbedError::MissingDate => false,
                    EmbedError::MissingDescription => true,
                    EmbedError::MissingMimeType => false,
                },
                ValidationError::MissingTagging => true,
            },
        }
    }

    /// Check whether the validator is compatible with a specific pdf version.
    pub fn compatible_with_version(&self, pdf_version: PdfVersion) -> bool {
        match self {
            Validator::None => true,
            Validator::A1_A | Validator::A1_B => pdf_version <= PdfVersion::Pdf14,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => pdf_version <= PdfVersion::Pdf17,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => pdf_version <= PdfVersion::Pdf17,
            Validator::UA1 => pdf_version <= PdfVersion::Pdf17,
        }
    }

    /// Get the recommended PDF version of a validator.
    pub fn recommended_version(&self) -> PdfVersion {
        match self {
            Validator::None => PdfVersion::Pdf17,
            Validator::A1_A | Validator::A1_B => PdfVersion::Pdf14,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => PdfVersion::Pdf17,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => PdfVersion::Pdf17,
            Validator::UA1 => PdfVersion::Pdf17,
        }
    }

    fn is_pdf_a(&self) -> bool {
        matches!(
            self,
            Validator::A1_A
                | Validator::A1_B
                | Validator::A2_A
                | Validator::A2_B
                | Validator::A2_U
                | Validator::A3_A
                | Validator::A3_B
                | Validator::A3_U
        )
    }

    pub(crate) fn write_xmp(&self, xmp: &mut XmpWriter) {
        if self.is_pdf_a() {
            let mut extension_schemas = xmp.extension_schemas();
            extension_schemas
                .xmp_media_management()
                .properties()
                .describe_instance_id();
            extension_schemas.pdf().properties().describe_all();
            extension_schemas.finish();
        }

        match self {
            Validator::None => {}
            Validator::A1_A => {
                xmp.pdfa_part(1);
                xmp.pdfa_conformance("A");
            }
            Validator::A1_B => {
                xmp.pdfa_part(1);
                xmp.pdfa_conformance("B");
            }
            Validator::A2_A => {
                xmp.pdfa_part(2);
                xmp.pdfa_conformance("A");
            }
            Validator::A2_B => {
                xmp.pdfa_part(2);
                xmp.pdfa_conformance("B");
            }
            Validator::A2_U => {
                xmp.pdfa_part(2);
                xmp.pdfa_conformance("U");
            }
            Validator::A3_A => {
                xmp.pdfa_part(3);
                xmp.pdfa_conformance("A");
            }
            Validator::A3_B => {
                xmp.pdfa_part(3);
                xmp.pdfa_conformance("B");
            }
            Validator::A3_U => {
                xmp.pdfa_part(3);
                xmp.pdfa_conformance("U");
            }
            Validator::UA1 => {
                xmp.pdfua_part(1);
            }
        }
    }

    pub(crate) fn requires_codepoint_mappings(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => false,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => *self != Validator::A2_B,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => *self != Validator::A3_B,
            Validator::UA1 => true,
        }
    }

    pub(crate) fn requires_display_doc_title(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => false,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => false,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => false,
            Validator::UA1 => true,
        }
    }

    pub(crate) fn requires_no_device_cs(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => true,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => true,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => true,
            Validator::UA1 => false,
        }
    }

    pub(crate) fn requires_tagging(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A => true,
            Validator::A1_B => false,
            Validator::A2_A => true,
            Validator::A2_B | Validator::A2_U => false,
            Validator::A3_A => true,
            Validator::A3_B | Validator::A3_U => false,
            Validator::UA1 => true,
        }
    }

    pub(crate) fn xmp_metadata(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => true,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => true,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => true,
            Validator::UA1 => true,
        }
    }

    pub(crate) fn requires_binary_header(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => true,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => true,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => true,
            Validator::UA1 => false,
        }
    }

    pub(crate) fn output_intent(&self) -> Option<OutputIntentSubtype> {
        match self {
            Validator::None => None,
            Validator::A1_A | Validator::A1_B => Some(OutputIntentSubtype::PDFA),
            Validator::A2_A | Validator::A2_B | Validator::A2_U => Some(OutputIntentSubtype::PDFA),
            Validator::A3_A | Validator::A3_B | Validator::A3_U => Some(OutputIntentSubtype::PDFA),
            Validator::UA1 => None,
        }
    }

    /// The string representation of the validator.
    pub fn as_str(&self) -> &str {
        match self {
            Validator::None => "None",
            Validator::A1_A => "PDF/A1-A",
            Validator::A1_B => "PDF/A1-B",
            Validator::A2_A => "PDF/A2-A",
            Validator::A2_B => "PDF/A2-B",
            Validator::A2_U => "PDF/A2-U",
            Validator::A3_A => "PDF/A3-A",
            Validator::A3_B => "PDF/A3-B",
            Validator::A3_U => "PDF/A3-U",
            Validator::UA1 => "PDF/UA1",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::action::LinkAction;
    use crate::annotation::{Annotation, LinkAnnotation, Target};
    use crate::error::KrillaError;
    use crate::font::{Font, GlyphId, GlyphUnits, KrillaGlyph};
    use crate::metadata::Metadata;
    use crate::outline::Outline;
    use crate::page::Page;
    use crate::paint::{LinearGradient, SpreadMethod};
    use crate::path::{Fill, FillRule};
    use crate::surface::TextDirection;
    use crate::tagging::{ArtifactType, ContentTag, Tag, TagGroup, TagTree};
    use crate::tests::{
        cmyk_fill, rect_to_path, red_fill, stops_with_2_solid_1, youtube_link, NOTO_SANS,
    };
    use crate::validation::ValidationError;
    use crate::{Document, SerializeSettings};
    use krilla_macros::snapshot;
    use pdf_writer::types::{ListNumbering, TableHeaderScope};
    use tiny_skia_path::{Point, Rect};

    fn pdfa_document() -> Document {
        Document::new_with(SerializeSettings::settings_7())
    }

    fn q_nesting_impl(settings: SerializeSettings) -> Document {
        let mut document = Document::new_with(settings);
        let mut page = document.start_page();
        let mut surface = page.surface();

        for _ in 0..29 {
            surface.push_clip_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), &FillRule::NonZero);
        }

        for _ in 0..29 {
            surface.pop();
        }

        surface.finish();
        page.finish();

        document
    }

    #[snapshot(document, settings_7)]
    pub fn validation_pdfa_q_nesting_28(document: &mut Document) {
        let mut page = document.start_page();
        let mut surface = page.surface();

        for _ in 0..28 {
            surface.push_clip_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), &FillRule::NonZero);
        }

        for _ in 0..28 {
            surface.pop();
        }
    }

    #[test]
    pub fn validation_pdfa_q_nesting_28() {
        let document = q_nesting_impl(SerializeSettings::settings_7());
        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::TooHighQNestingLevel
            ]))
        );
    }

    #[test]
    pub fn validation_pdfa_string_length() {
        let mut document = pdfa_document();
        let metadata = Metadata::new().creator("A".repeat(32768));
        document.set_metadata(metadata);
        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::TooLongString
            ]))
        );
    }

    #[snapshot(single_page, settings_7)]
    fn validation_pdfa_annotation(page: &mut Page) {
        page.add_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
                Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
            )
            .into(),
        );
    }

    #[test]
    fn validation_pdfa_postscript() {
        let mut document = pdfa_document();
        let mut page = document.start_page();

        let gradient = LinearGradient {
            x1: 50.0,
            y1: 0.0,
            x2: 150.0,
            y2: 0.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Repeat,
            stops: stops_with_2_solid_1(),
            anti_alias: false,
        };

        let fill = Fill {
            paint: gradient.into(),
            ..Default::default()
        };

        let mut surface = page.surface();

        surface.fill_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), fill);

        surface.finish();
        page.finish();

        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::ContainsPostScript
            ]))
        )
    }

    #[test]
    pub fn validation_disabled_q_nesting_28() {
        let document = q_nesting_impl(SerializeSettings::default());
        assert!(document.finish().is_ok());
    }

    fn cmyk_document_impl(document: &mut Document) {
        let mut page = document.start_page();
        let mut surface = page.surface();

        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let fill = cmyk_fill(1.0);
        surface.fill_path(&path, fill);

        surface.finish();
        page.finish();
    }

    #[test]
    fn validation_pdfa_missing_cmyk() {
        let mut document = pdfa_document();
        cmyk_document_impl(&mut document);

        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::MissingCMYKProfile
            ]))
        )
    }

    #[test]
    fn validation_pdfa_existing_cmyk() {
        let mut document = Document::new_with(SerializeSettings::settings_8());
        cmyk_document_impl(&mut document);

        assert!(document.finish().is_ok())
    }

    #[test]
    fn validation_pdfa_notdef_glyph() {
        let mut document = pdfa_document();
        let mut page = document.start_page();
        let mut surface = page.surface();

        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0, true).unwrap();

        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            20.0,
            &[],
            "你",
            false,
            TextDirection::Auto,
        );
        surface.finish();
        page.finish();

        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::ContainsNotDefGlyph
            ]))
        )
    }

    fn validation_pdf_full_example(document: &mut Document) {
        let mut page = document.start_page();
        let mut surface = page.surface();

        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0, true).unwrap();

        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            20.0,
            &[],
            "This is some text",
            false,
            TextDirection::Auto,
        );

        surface.fill_path(&rect_to_path(30.0, 30.0, 70.0, 70.0), red_fill(1.0));

        surface.finish();
        page.finish();
    }

    pub(crate) fn validation_pdf_tagged_full_example(document: &mut Document) {
        let mut page = document.start_page();
        let mut surface = page.surface();

        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0, true).unwrap();

        let id1 = surface.start_tagged(ContentTag::Span(
            "",
            Some("Alt"),
            Some("Expanded"),
            Some("ActualText"),
        ));
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            20.0,
            &[],
            "This is some text",
            false,
            TextDirection::Auto,
        );
        surface.end_tagged();

        let id2 = surface.start_tagged(ContentTag::Artifact(ArtifactType::Header));
        surface.fill_path(&rect_to_path(30.0, 30.0, 70.0, 70.0), red_fill(1.0));
        surface.end_tagged();

        surface.finish();
        page.finish();

        let mut tag_tree = TagTree::new();
        tag_tree.push(id1);
        tag_tree.push(id2);
        document.set_tag_tree(tag_tree);

        let metadata = Metadata::new().language("en".to_string());
        document.set_metadata(metadata);
    }

    fn invalid_codepoint_impl(document: &mut Document, font: Font, text: &str) {
        let mut page = document.start_page();
        let mut surface = page.surface();

        let glyphs = vec![
            KrillaGlyph::new(GlyphId::new(3), 2048.0, 0.0, 0.0, 0.0, 0..1),
            KrillaGlyph::new(GlyphId::new(2), 2048.0, 0.0, 0.0, 0.0, 1..4),
        ];

        surface.fill_glyphs(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            &glyphs,
            font.clone(),
            text,
            20.0,
            GlyphUnits::UnitsPerEm,
            false,
        );
        surface.finish();
        page.finish();
    }

    #[test]
    fn validation_pdfu_invalid_codepoint() {
        let mut document = Document::new_with(SerializeSettings::settings_9());
        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0, true).unwrap();
        invalid_codepoint_impl(&mut document, font.clone(), "A\u{FEFF}B");

        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::InvalidCodepointMapping(font, GlyphId::new(2),)
            ]))
        )
    }

    #[test]
    fn validation_pdfa_private_unicode_codepoint() {
        let mut document = Document::new_with(SerializeSettings::settings_13());
        let metadata = Metadata::new().language("en".to_string());
        document.set_metadata(metadata);
        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0, true).unwrap();
        invalid_codepoint_impl(&mut document, font.clone(), "A\u{E022}B");

        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::UnicodePrivateArea(font, GlyphId::new(2))
            ]))
        )
    }

    #[snapshot(document, settings_20)]
    fn validation_pdfa1_a_full_example(document: &mut Document) {
        validation_pdf_tagged_full_example(document);
    }

    #[snapshot(document, settings_19)]
    fn validation_pdfa1_b_full_example(document: &mut Document) {
        validation_pdf_full_example(document);
    }

    #[snapshot(document, settings_13)]
    fn validation_pdfa2_a_full_example(document: &mut Document) {
        validation_pdf_tagged_full_example(document);
    }

    #[snapshot(document, settings_7)]
    fn validation_pdfa2_b_full_example(document: &mut Document) {
        validation_pdf_full_example(document);
    }

    #[snapshot(document, settings_9)]
    fn validation_pdfa2_u_full_example(document: &mut Document) {
        validation_pdf_full_example(document);
    }

    #[snapshot(document, settings_14)]
    fn validation_pdfa3_a_full_example(document: &mut Document) {
        validation_pdf_tagged_full_example(document);
    }

    #[snapshot(document, settings_10)]
    fn validation_pdfa3_b_full_example(document: &mut Document) {
        validation_pdf_full_example(document);
    }

    #[snapshot(document, settings_11)]
    fn validation_pdfa3_u_full_example(document: &mut Document) {
        validation_pdf_full_example(document);
    }

    #[snapshot(document, settings_15)]
    fn validation_pdfua1_full_example(document: &mut Document) {
        let mut page = document.start_page();
        let mut surface = page.surface();

        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0, true).unwrap();

        let id1 = surface.start_tagged(ContentTag::Span("", None, None, None));
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            20.0,
            &[],
            "This is some text",
            false,
            TextDirection::Auto,
        );
        surface.end_tagged();

        surface.finish();

        let annotation = page.add_tagged_annotation(Annotation::new_link(
            LinkAnnotation::new(
                Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
                Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
            ),
            Some("A link to youtube".to_string()),
        ));

        page.finish();

        let mut tag_tree = TagTree::new();
        tag_tree.push(id1);
        tag_tree.push(annotation);
        document.set_tag_tree(tag_tree);

        let metadata = Metadata::new()
            .language("en".to_string())
            .title("a nice title".to_string());
        document.set_metadata(metadata);

        let outline = Outline::new();
        document.set_outline(outline);
    }

    #[test]
    fn validation_pdfua1_missing_requirements() {
        let mut document = Document::new_with(SerializeSettings::settings_15());
        let mut page = document.start_page();
        let mut surface = page.surface();

        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0, true).unwrap();

        let id1 = surface.start_tagged(ContentTag::Span("", None, None, None));
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            20.0,
            &[],
            "Hi",
            false,
            TextDirection::Auto,
        );
        surface.end_tagged();

        surface.finish();

        let annot = page.add_tagged_annotation(Annotation::new_link(
            LinkAnnotation::new(
                Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
                Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
            ),
            None,
        ));

        page.finish();

        let mut tag_tree = TagTree::new();
        let mut tag_group = TagGroup::new(Tag::Formula(None));
        tag_group.push(id1);
        tag_group.push(annot);
        tag_tree.push(tag_group);
        document.set_tag_tree(tag_tree);

        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::MissingDocumentOutline,
                ValidationError::MissingAnnotationAltText,
                ValidationError::MissingAltText,
                ValidationError::NoDocumentTitle
            ]))
        )
    }

    #[snapshot(document, settings_15)]
    fn validation_pdfua1_attributes(document: &mut Document) {
        let mut page = document.start_page();
        let mut surface = page.surface();

        let id1 = surface.start_tagged(ContentTag::Span("", None, None, None));
        surface.fill_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), red_fill(1.0));
        surface.end_tagged();

        let id2 = surface.start_tagged(ContentTag::Other);
        surface.fill_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), red_fill(1.0));
        surface.end_tagged();

        surface.finish();
        page.finish();

        let mut tag_tree = TagTree::new();

        let mut group1 = TagGroup::new(Tag::L(ListNumbering::Circle));
        group1.push(id1);

        let mut group2 = TagGroup::new(Tag::TH(TableHeaderScope::Row));
        group2.push(id2);

        tag_tree.push(group1);
        tag_tree.push(group2);
        document.set_tag_tree(tag_tree);

        let metadata = Metadata::new()
            .language("en".to_string())
            .title("a nice title".to_string());
        document.set_metadata(metadata);

        let outline = Outline::new();
        document.set_outline(outline);
    }

    #[snapshot(document, settings_16)]
    fn pdf_version_14_tagged(document: &mut Document) {
        validation_pdf_tagged_full_example(document);
    }

    #[test]
    fn validation_pdfa1_no_transparency() {
        let mut document = Document::new_with(SerializeSettings::settings_19());
        let metadata = Metadata::new().language("en".to_string());
        document.set_metadata(metadata);
        let mut page = document.start_page();
        let mut surface = page.surface();
        surface.fill_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), red_fill(0.5));
        surface.finish();
        page.finish();

        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::Transparency
            ]))
        )
    }

    #[snapshot(document, settings_21)]
    fn validation_version_mismatch(document: &mut Document) {
        validation_pdf_full_example(document);
    }

    #[snapshot(document, settings_22)]
    fn validation_other_version(document: &mut Document) {
        validation_pdf_full_example(document);
    }

    #[test]
    fn validation_pdfa1_limits() {
        let mut document = Document::new_with(SerializeSettings::settings_19());
        let mut page = document.start_page();

        // An array can only have 8191 elements, so it must not be possible to have that many.
        for _ in 0..8193 {
            page.add_annotation(youtube_link(100.0, 100.0, 100.0, 100.0));
        }

        page.add_annotation(youtube_link(66000.1, 66000.1, 100.0, 100.0));
        page.finish();

        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::TooLargeFloat,
                ValidationError::TooLongArray,
            ]))
        )
    }

    #[test]
    fn validation_pdfa3a_no_tag_tree() {
        let mut document = Document::new_with(SerializeSettings::settings_24());
        document.set_metadata(Metadata::new().language("en".to_string()));

        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::MissingTagging
            ]))
        )
    }
}
