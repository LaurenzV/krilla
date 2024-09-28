use std::fmt::Debug;

pub mod dummy;
pub mod pdfa;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ValidationError {
    TooLongString,
    TooLongName,
    TooManyIndirectObjects,
    TooHighQNestingLevel,
}

pub trait Validator: Clone + Copy + Debug {
    fn strings_less_than_32767(&self) -> bool;
    fn name_less_than_127(&self) -> bool;
    fn indirect_objects_less_than_8388607(&self) -> bool;
    fn q_nesting_less_than_128(&self) -> bool;
}
