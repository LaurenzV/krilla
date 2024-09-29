use crate::font::Font;
use pdf_writer::types::OutputIntentSubtype;
use skrifa::GlyphId;
use std::fmt::Debug;
use xmp_writer::XmpWriter;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValidationError {
    TooLongString,
    TooManyIndirectObjects,
    TooHighQNestingLevel,
    ContainsPostScript,
    MissingCMYKProfile,
    ContainsNotDefGlyph,
    InvalidCodepointMapping(Font, GlyphId, Option<String>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Validator {
    Dummy,
    PdfA2A,
    PdfA2B,
    PdfA2U,
}

impl Validator {
    pub fn prohibits(&self, validation_error: &ValidationError) -> bool {
        match self {
            Validator::Dummy => false,
            Validator::PdfA2A | Validator::PdfA2B | Validator::PdfA2U => match validation_error {
                ValidationError::TooLongString => true,
                ValidationError::TooManyIndirectObjects => true,
                ValidationError::TooHighQNestingLevel => true,
                ValidationError::ContainsPostScript => true,
                ValidationError::MissingCMYKProfile => true,
                ValidationError::ContainsNotDefGlyph => true,
                // Only applies for PDF/A2-U and PDF/A2-A
                ValidationError::InvalidCodepointMapping(_, _, _) => *self != Validator::PdfA2B,
            },
        }
    }

    pub fn write_xmp(&self, xmp: &mut XmpWriter) {
        match self {
            Validator::Dummy => {}
            Validator::PdfA2A => {
                xmp.pdfa_part("2");
                xmp.pdfa_conformance("A");
            }
            Validator::PdfA2B => {
                xmp.pdfa_part("2");
                xmp.pdfa_conformance("B");
            }
            Validator::PdfA2U => {
                xmp.pdfa_part("2");
                xmp.pdfa_conformance("U");
            }
        }
    }

    pub fn annotation_ap_stream(&self) -> bool {
        match self {
            Validator::Dummy => false,
            Validator::PdfA2A | Validator::PdfA2B | Validator::PdfA2U => true,
        }
    }

    pub fn no_device_cs(&self) -> bool {
        match self {
            Validator::Dummy => false,
            Validator::PdfA2A | Validator::PdfA2B | Validator::PdfA2U => true,
        }
    }

    pub fn xmp_metadata(&self) -> bool {
        match self {
            Validator::Dummy => false,
            Validator::PdfA2A | Validator::PdfA2B | Validator::PdfA2U => true,
        }
    }

    pub fn requires_binary_header(&self) -> bool {
        match self {
            Validator::Dummy => false,
            Validator::PdfA2A | Validator::PdfA2B | Validator::PdfA2U => true,
        }
    }

    pub fn output_intent(&self) -> Option<OutputIntentSubtype> {
        match self {
            Validator::Dummy => None,
            Validator::PdfA2A | Validator::PdfA2B | Validator::PdfA2U => {
                Some(OutputIntentSubtype::PDFA)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::action::LinkAction;
    use crate::annotation::{LinkAnnotation, Target};
    use crate::error::KrillaError;
    use crate::font::{Font, GlyphId, GlyphUnits, KrillaGlyph};
    use crate::metadata::Metadata;
    use crate::page::Page;
    use crate::paint::{LinearGradient, SpreadMethod};
    use crate::path::{Fill, FillRule};
    use crate::surface::TextDirection;
    use crate::tests::{cmyk_fill, rect_to_path, red_fill, stops_with_2_solid_1, NOTO_SANS};
    use crate::validation::ValidationError;
    use crate::{Document, SerializeSettings};
    use krilla_macros::snapshot;
    use std::iter;
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
        let metadata = Metadata::new().creator(iter::repeat("A").take(32768).collect());
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
            LinkAnnotation {
                rect: Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
                target: Target::Action(
                    LinkAction::new("https://www.youtube.com".to_string()).into(),
                ),
            }
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
        let font = Font::new(font_data, 0, vec![]).unwrap();

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
        let font = Font::new(font_data, 0, vec![]).unwrap();

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

        surface.fill_path(&rect_to_path(30.0, 30.0, 70.0, 70.0), red_fill(0.5));

        surface.finish();
        page.finish();
    }

    #[snapshot(document, settings_7)]
    fn validation_pdfa2_b_full_example(document: &mut Document) {
        validation_pdf_full_example(document);
    }

    #[test]
    fn validation_pdfu_invalid_codepoint() {
        let mut document = Document::new_with(SerializeSettings::settings_9());
        let mut page = document.start_page();
        let mut surface = page.surface();

        let font_data = NOTO_SANS.clone();
        let font = Font::new(font_data, 0, vec![]).unwrap();

        let glyphs = vec![
            KrillaGlyph::new(GlyphId::new(3), 2048.0, 0.0, 0.0, 0.0, 0..1),
            KrillaGlyph::new(GlyphId::new(2), 2048.0, 0.0, 0.0, 0.0, 1..4),
        ];

        surface.fill_glyphs(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            &glyphs,
            font.clone(),
            "A\u{FEFF}B",
            20.0,
            GlyphUnits::UnitsPerEm,
            false,
        );
        surface.finish();
        page.finish();

        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::InvalidCodepointMapping(
                    font,
                    GlyphId::new(2),
                    Some("\u{FEFF}".to_string())
                )
            ]))
        )
    }

    #[snapshot(document, settings_9)]
    fn validation_pdfa2_u_full_example(document: &mut Document) {
        validation_pdf_full_example(document);
    }
}
