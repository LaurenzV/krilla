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
}

#[cfg(test)]
mod tests {
    use crate::error::KrillaError;
    use crate::path::FillRule;
    use crate::tests::rect_to_path;
    use crate::validation::ValidationError;
    use crate::{Document, SerializeSettings};
    use krilla_macros::snapshot;

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
        let mut document = Document::new_with(SerializeSettings::settings_7());
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
        assert_eq!(
            document.finish(),
            Err(KrillaError::ValidationError(vec![
                ValidationError::TooHighQNestingLevel
            ]))
        );
    }
}
