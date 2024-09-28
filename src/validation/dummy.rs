use crate::validation::Validator;

#[derive(Clone, Copy, Debug)]
pub struct DummyValidator;

impl Validator for DummyValidator {
    fn strings_less_than_32767(&self) -> bool {
        false
    }

    fn name_less_than_127(&self) -> bool {
        false
    }

    fn indirect_objects_less_than_8388607(&self) -> bool {
        false
    }

    fn q_nesting_less_than_128(&self) -> bool {
        false
    }
}
