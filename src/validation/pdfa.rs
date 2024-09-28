use crate::validation::Validator;

#[derive(Debug, Clone, Copy)]
pub enum ConformanceLevel {
    A,
    B,
    U,
}

#[derive(Debug, Clone, Copy)]
pub struct PdfA2Validator(ConformanceLevel);

impl PdfA2Validator {
    pub fn new(conformance_level: ConformanceLevel) -> Self {
        Self(conformance_level)
    }
}

impl Validator for PdfA2Validator {
    fn strings_less_than_32767(&self) -> bool {
        true
    }

    fn name_less_than_127(&self) -> bool {
        true
    }

    fn indirect_objects_less_than_8388607(&self) -> bool {
        true
    }

    fn q_nesting_less_than_128(&self) -> bool {
        true
    }
}
