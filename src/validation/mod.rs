use std::fmt::Debug;

#[derive(Debug, Clone, Copy)]
pub enum ConformanceLevel {
    A,
    B,
    U,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ValidationError {
    TooLongString,
    TooManyIndirectObjects,
    TooHighQNestingLevel,
}

#[derive(Copy, Clone, Debug)]
pub enum Validator {
    Dummy,
    PdfA2(ConformanceLevel),
}

impl Validator {
    pub fn prohibits(&self, validation_error: ValidationError) -> bool {
        match self {
            Validator::Dummy => false,
            Validator::PdfA2(_) => match validation_error {
                ValidationError::TooLongString => true,
                ValidationError::TooManyIndirectObjects => true,
                ValidationError::TooHighQNestingLevel => true,
            },
        }
    }

    pub fn annotation_ap_stream(&self) -> bool {
        match self {
            Validator::Dummy => false,
            Validator::PdfA2(_) => true
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::KrillaError;
    use crate::metadata::Metadata;
    use crate::path::FillRule;
    use crate::tests::rect_to_path;
    use crate::validation::ValidationError;
    use crate::{Document, SerializeSettings};
    use krilla_macros::snapshot;
    use std::iter;
    use tiny_skia_path::Rect;
    use crate::action::LinkAction;
    use crate::annotation::{LinkAnnotation, Target};
    use crate::page::Page;

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
        let mut document = Document::new_with(SerializeSettings::settings_7());
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
    pub fn validation_disabled_q_nesting_28() {
        let document = q_nesting_impl(SerializeSettings::default());
        assert!(document.finish().is_ok());
    }
}
