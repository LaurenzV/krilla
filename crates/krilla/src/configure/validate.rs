//! Exporting with a specific PDF conformance level.
//!
//! PDF defines a number of additional conformance levelx that restrict the features of PDF that
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
//!   requirements are fulfilled.
//!   You can find them under **Requirements** for each [`Validator`].
//!
//! [`Configuration`]: crate::configure::Configuration

use std::fmt::Debug;

use pdf_writer::types::OutputIntentSubtype;
use pdf_writer::Finish;
use xmp_writer::XmpWriter;

use crate::configure::PdfVersion;
use crate::interchange::embed::EmbedError;
use crate::surface::Location;
use crate::tagging::TagId;
use crate::text::Font;
use crate::text::GlyphId;

/// An error that occurred during validation/
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
    /// The `.notdef` glyph was used, which is forbidden by some export formats.
    ///
    /// Can occur if a glyph could not be found in the font for a corresponding codepoint
    /// in the input text, or if it was explicitly mapped that way.
    ///
    /// The third argument contains the text range of the glyph.
    ContainsNotDefGlyph(Font, Option<Location>, String),
    /// A glyph was mapped either to the codepoint 0x0, 0xFEFF or 0xFFFE, or no codepoint at all,
    /// which is forbidden by some standards.
    ///
    /// Can occur if those codepoints appeared in the input text, or were explicitly
    /// mapped to that glyph.
    ///
    /// If the third argument is `None`, the glyph was mapped to no codepoint at all (i.e.
    /// an empty string). Otherwise, it was mapped to that codepoint.
    InvalidCodepointMapping(Font, GlyphId, Option<char>, Option<Location>),
    /// A glyph was mapped to a codepoint in the Unicode private use area, which is forbidden
    /// by some standards, like for example PDF/A2-A.
    // Note that the standard doesn't explicitly forbid it, but instead requires an ActualText
    // attribute to be present. But we just completely forbid it, for simplicity.
    UnicodePrivateArea(Font, GlyphId, char, Option<Location>),
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
    /// The date of the document is missing.
    // We need this because for some standards we need to add the
    // xmp:History attribute.
    MissingDocumentDate,
    /// The PDF contains transparency, which is forbidden by some standards (e.g. PDF/A1).
    Transparency(Option<Location>),
    /// The PDF contains an image with `interpolate` set to `true`.
    ImageInterpolation(Option<Location>),
    /// The PDF contains an embedded file.
    EmbeddedFile(EmbedError, Option<Location>),
    /// The PDF contains no tagging.
    MissingTagging,
    /// A duplicate [`Tag::id`] was provided.
    ///
    /// [`Tag::id`]: crate::interchange::tagging::Tag::id
    DuplicateTagId(TagId, Option<Location>),
    /// An id of [`TableHeaderRefs`] was not found in the [`TagTree`].
    ///
    /// [`TableHeaderRefs`]: crate::interchange::tagging::TableHeaderRefs
    /// [`TagTree`]: crate::interchange::tagging::TagTree
    UnknownHeaderTagId(TagId, Option<Location>),
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
    /// NOTE: THIS EXPORT MODE IS EXPERIMENTAL AND SHOULDN'T BE USED IN PRODUCTION YET!
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
    /// - You should make use of the `Alt`, `ActualText`, `Lang` and `Expansion` attributes
    ///   whenever possible.
    /// - Usually, you can provide an empty string as `Lang` to indicate that a language is unknown.
    ///   You should not do that in PDF-UA.
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
    /// [`Tag`]: crate::interchange::tagging::Tag
    UA1,
    /// The validator for the PDF/A4 standard.
    ///
    /// **Requirements**:
    /// - While not required, it's recommended to enable tagging.
    A4,
    /// The validator for the PDF/A4f standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A4
    A4F,
    /// The validator for the PDF/A4e standard.
    ///
    /// **Requirements**:
    /// - All requirements of PDF/A4
    A4E,
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
                ValidationError::ContainsPostScript(_) => true,
                ValidationError::MissingCMYKProfile => true,
                ValidationError::ContainsNotDefGlyph(_, _, _) => false,
                ValidationError::InvalidCodepointMapping(_, _, _, _) => {
                    self.requires_codepoint_mappings()
                }
                ValidationError::UnicodePrivateArea(_, _, _, _) => false,
                ValidationError::NoDocumentLanguage => *self == Validator::A1_A,
                ValidationError::NoDocumentTitle => false,
                ValidationError::MissingAltText => false,
                ValidationError::MissingHeadingTitle => false,
                ValidationError::MissingDocumentOutline => false,
                ValidationError::MissingAnnotationAltText => false,
                ValidationError::Transparency(_) => true,
                ValidationError::ImageInterpolation(_) => true,
                // PDF/A1 doesn't strictly forbid, but it disallows the EF key,
                // which we always insert. So we just forbid it overall.
                ValidationError::EmbeddedFile(e, _) => match e {
                    EmbedError::Existence => true,
                    // Since existence is forbidden in the first place,
                    // we can just set the others to `false` to prevent unnecessary
                    // validation errors.
                    EmbedError::MissingDate => false,
                    EmbedError::MissingDescription => false,
                    EmbedError::MissingMimeType => false,
                },
                ValidationError::MissingTagging => *self == Validator::A1_A,
                ValidationError::MissingDocumentDate => true,
                ValidationError::DuplicateTagId(_, _) => true,
                ValidationError::UnknownHeaderTagId(_, _) => true,
            },
            Validator::A2_A | Validator::A2_B | Validator::A2_U => match validation_error {
                ValidationError::TooLongString => true,
                ValidationError::TooLongName => true,
                ValidationError::TooLargeFloat => false,
                ValidationError::TooLongArray => false,
                ValidationError::TooLongDictionary => false,
                ValidationError::TooManyIndirectObjects => true,
                ValidationError::TooHighQNestingLevel => true,
                ValidationError::ContainsPostScript(_) => true,
                ValidationError::MissingCMYKProfile => true,
                ValidationError::ContainsNotDefGlyph(_, _, _) => true,
                ValidationError::InvalidCodepointMapping(_, _, _, _) => {
                    self.requires_codepoint_mappings()
                }
                ValidationError::UnicodePrivateArea(_, _, _, _) => *self == Validator::A2_A,
                ValidationError::NoDocumentLanguage => *self == Validator::A2_A,
                ValidationError::NoDocumentTitle => false,
                ValidationError::MissingAltText => false,
                ValidationError::MissingHeadingTitle => false,
                ValidationError::MissingDocumentOutline => false,
                ValidationError::MissingAnnotationAltText => false,
                ValidationError::Transparency(_) => false,
                ValidationError::ImageInterpolation(_) => true,
                // Also not strictly forbidden, but we can't ensure that it is PDF/A2 compliant,
                // so we just forbid it completely.
                ValidationError::EmbeddedFile(e, _) => match e {
                    EmbedError::Existence => true,
                    // Since existence is forbidden in the first place,
                    // we can just set the others to `false` to prevent unnecessary
                    // validation errors.
                    EmbedError::MissingDate => false,
                    EmbedError::MissingDescription => false,
                    EmbedError::MissingMimeType => false,
                },
                ValidationError::MissingTagging => *self == Validator::A2_A,
                ValidationError::MissingDocumentDate => true,
                ValidationError::DuplicateTagId(_, _) => true,
                ValidationError::UnknownHeaderTagId(_, _) => true,
            },
            Validator::A3_A | Validator::A3_B | Validator::A3_U => match validation_error {
                ValidationError::TooLongString => true,
                ValidationError::TooLongName => true,
                ValidationError::TooLargeFloat => false,
                ValidationError::TooLongArray => false,
                ValidationError::TooLongDictionary => false,
                ValidationError::TooManyIndirectObjects => true,
                ValidationError::TooHighQNestingLevel => true,
                ValidationError::ContainsPostScript(_) => true,
                ValidationError::MissingCMYKProfile => true,
                ValidationError::ContainsNotDefGlyph(_, _, _) => true,
                ValidationError::InvalidCodepointMapping(_, _, _, _) => {
                    self.requires_codepoint_mappings()
                }
                ValidationError::UnicodePrivateArea(_, _, _, _) => *self == Validator::A3_A,
                ValidationError::NoDocumentLanguage => *self == Validator::A3_A,
                ValidationError::NoDocumentTitle => false,
                ValidationError::MissingAltText => false,
                ValidationError::MissingHeadingTitle => false,
                ValidationError::MissingDocumentOutline => false,
                ValidationError::MissingAnnotationAltText => false,
                ValidationError::Transparency(_) => false,
                ValidationError::ImageInterpolation(_) => true,
                ValidationError::EmbeddedFile(er, _) => match er {
                    EmbedError::Existence => false,
                    EmbedError::MissingDate => true,
                    EmbedError::MissingDescription => true,
                    EmbedError::MissingMimeType => true,
                },
                ValidationError::MissingTagging => *self == Validator::A3_A,
                ValidationError::MissingDocumentDate => true,
                ValidationError::DuplicateTagId(_, _) => true,
                ValidationError::UnknownHeaderTagId(_, _) => true,
            },
            Validator::A4 | Validator::A4F | Validator::A4E => match validation_error {
                ValidationError::TooLongString => false,
                ValidationError::TooLongName => false,
                ValidationError::TooLongArray => false,
                ValidationError::TooLongDictionary => false,
                ValidationError::TooLargeFloat => false,
                ValidationError::TooManyIndirectObjects => false,
                ValidationError::TooHighQNestingLevel => false,
                ValidationError::ContainsPostScript(_) => false,
                ValidationError::MissingCMYKProfile => true,
                ValidationError::ContainsNotDefGlyph(_, _, _) => true,
                ValidationError::InvalidCodepointMapping(_, _, _, _) => true,
                // Not strictly forbidden if we surround with actual text, but
                // easier to just forbid it.
                ValidationError::UnicodePrivateArea(_, _, _, _) => true,
                ValidationError::NoDocumentLanguage => false,
                ValidationError::NoDocumentTitle => false,
                ValidationError::MissingAltText => false,
                ValidationError::MissingHeadingTitle => false,
                ValidationError::MissingDocumentOutline => false,
                ValidationError::MissingAnnotationAltText => false,
                ValidationError::Transparency(_) => false,
                ValidationError::ImageInterpolation(_) => true,
                ValidationError::EmbeddedFile(e, _) => match e {
                    EmbedError::Existence => matches!(self, Validator::A4),
                    // Since existence is forbidden in the first place for A4,
                    // we can just set the others to `false` to prevent
                    // unnecessary validation errors.
                    EmbedError::MissingDate => false,
                    EmbedError::MissingDescription => {
                        matches!(self, Validator::A4E | Validator::A4F)
                    }
                    EmbedError::MissingMimeType => false,
                },
                // Only recommended, not required.
                ValidationError::MissingTagging => false,
                ValidationError::MissingDocumentDate => true,
                ValidationError::DuplicateTagId(_, _) => true,
                ValidationError::UnknownHeaderTagId(_, _) => true,
            },
            Validator::UA1 => match validation_error {
                ValidationError::TooLongString => false,
                ValidationError::TooLargeFloat => false,
                ValidationError::TooLongName => false,
                ValidationError::TooLongArray => false,
                ValidationError::TooLongDictionary => false,
                ValidationError::TooManyIndirectObjects => false,
                ValidationError::TooHighQNestingLevel => false,
                ValidationError::ContainsPostScript(_) => false,
                ValidationError::MissingCMYKProfile => false,
                ValidationError::ContainsNotDefGlyph(_, _, _) => true,
                ValidationError::InvalidCodepointMapping(_, _, _, _) => {
                    self.requires_codepoint_mappings()
                }
                ValidationError::UnicodePrivateArea(_, _, _, _) => false,
                ValidationError::NoDocumentLanguage => false,
                ValidationError::NoDocumentTitle => true,
                ValidationError::MissingAltText => true,
                ValidationError::MissingHeadingTitle => true,
                ValidationError::MissingDocumentOutline => true,
                ValidationError::MissingAnnotationAltText => true,
                ValidationError::Transparency(_) => false,
                ValidationError::ImageInterpolation(_) => false,
                ValidationError::EmbeddedFile(er, _) => match er {
                    EmbedError::Existence => false,
                    EmbedError::MissingDate => false,
                    EmbedError::MissingDescription => true,
                    EmbedError::MissingMimeType => false,
                },
                ValidationError::MissingTagging => true,
                ValidationError::MissingDocumentDate => false,
                ValidationError::DuplicateTagId(_, _) => true,
                ValidationError::UnknownHeaderTagId(_, _) => true,
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
            // It can be any 2.x version, but we're not there yet.
            Validator::A4 | Validator::A4F | Validator::A4E => pdf_version == PdfVersion::Pdf20,
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
            Validator::A4 | Validator::A4F | Validator::A4E => PdfVersion::Pdf20,
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
                | Validator::A4
                | Validator::A4F
                | Validator::A4E
        )
    }

    pub(crate) fn write_xmp(&self, xmp: &mut XmpWriter) {
        // TODO: Also needed for PDF/UA?
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
            Validator::A4 => {
                xmp.pdfa_part(4);
                xmp.pdfa_rev(2020);
            }
            Validator::A4F => {
                xmp.pdfa_part(4);
                xmp.pdfa_rev(2020);
                xmp.pdfa_conformance("F");
            }
            Validator::A4E => {
                xmp.pdfa_part(4);
                xmp.pdfa_rev(2020);
                xmp.pdfa_conformance("E");
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
            Validator::A4 | Validator::A4F | Validator::A4E => true,
            Validator::UA1 => true,
        }
    }

    pub(crate) fn requires_display_doc_title(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => false,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => false,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => false,
            Validator::A4 | Validator::A4F | Validator::A4E => false,
            Validator::UA1 => true,
        }
    }

    pub(crate) fn requires_no_device_cs(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => true,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => true,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => true,
            Validator::A4 | Validator::A4F | Validator::A4E => true,
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
            Validator::A4 | Validator::A4F | Validator::A4E => false,
            Validator::UA1 => true,
        }
    }

    pub(crate) fn xmp_metadata(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => true,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => true,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => true,
            Validator::A4 | Validator::A4F | Validator::A4E => true,
            Validator::UA1 => true,
        }
    }

    pub(crate) fn requires_binary_header(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => true,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => true,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => true,
            Validator::A4 | Validator::A4F | Validator::A4E => true,
            Validator::UA1 => false,
        }
    }

    pub(crate) fn requires_file_provenance_information(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => true,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => true,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => true,
            Validator::A4 | Validator::A4F | Validator::A4E => true,
            Validator::UA1 => false,
        }
    }

    pub(crate) fn prohibits_instance_id_in_xmp_metadata(&self) -> bool {
        match self {
            Validator::None => false,
            Validator::A1_A | Validator::A1_B => true,
            Validator::A2_A | Validator::A2_B | Validator::A2_U => false,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => false,
            Validator::A4 | Validator::A4F | Validator::A4E => false,
            Validator::UA1 => false,
        }
    }

    pub(crate) fn output_intent(&self) -> Option<OutputIntentSubtype> {
        match self {
            Validator::None => None,
            Validator::A1_A | Validator::A1_B => Some(OutputIntentSubtype::PDFA),
            Validator::A2_A | Validator::A2_B | Validator::A2_U => Some(OutputIntentSubtype::PDFA),
            Validator::A3_A | Validator::A3_B | Validator::A3_U => Some(OutputIntentSubtype::PDFA),
            Validator::A4 | Validator::A4F | Validator::A4E => Some(OutputIntentSubtype::PDFA),
            Validator::UA1 => None,
        }
    }

    pub(crate) fn allows_info_dict(&self) -> bool {
        match self {
            Validator::None
            | Validator::A1_A
            | Validator::A1_B
            | Validator::A2_A
            | Validator::A2_B
            | Validator::A2_U
            | Validator::A3_A
            | Validator::A3_B
            | Validator::A3_U
            | Validator::UA1 => true,
            Validator::A4 | Validator::A4F | Validator::A4E => false,
        }
    }

    pub(crate) fn write_embedded_files(&self, is_empty: bool) -> bool {
        match self {
            Validator::None
            | Validator::A1_A
            | Validator::A1_B
            | Validator::A2_A
            | Validator::A2_B
            | Validator::A2_U
            | Validator::A3_A
            | Validator::A3_B
            | Validator::A3_U
            | Validator::A4
            | Validator::A4E
            | Validator::UA1 => !is_empty,
            // For this one we always need to write an `EmbeddedFiles` entry,
            // even if empty.
            Validator::A4F => true,
        }
    }

    pub(crate) fn allows_associated_files(&self) -> bool {
        match self {
            // PDF 2.0 _does_ support associated files. However, in this case the document has to
            // provide a modification date, since it's a required field. Therefore, it's easier to
            // just use the associated files feature, apart from PDF/A3.
            Validator::None => false,
            Validator::A3_A | Validator::A3_B | Validator::A3_U => true,
            Validator::A4 | Validator::A4F | Validator::A4E => true,
            Validator::A1_A
            | Validator::A1_B
            | Validator::A2_A
            | Validator::A2_B
            | Validator::A2_U
            | Validator::UA1 => false,
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
            Validator::A4 => "PDF/A4",
            Validator::A4F => "PDF/A4f",
            Validator::A4E => "PDF/A4e",
            Validator::UA1 => "PDF/UA1",
        }
    }
}
