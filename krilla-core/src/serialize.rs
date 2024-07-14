use crate::resource::PdfColorSpace;
use pdf_writer::{Chunk, Pdf, Ref};
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum PdfObject {
    PdfColorSpace(PdfColorSpace),
}

pub struct RefAllocator {
    mappings: HashMap<PdfObject, Ref>,
    _ref: Ref,
}

impl RefAllocator {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
            _ref: Ref::new(1),
        }
    }

    pub fn get_ref(&mut self, object: PdfObject) -> Ref {
        let mappings = &mut self.mappings;
        let _ref = &mut self._ref;
        *mappings.entry(object.clone()).or_insert(_ref.bump())
    }
}

pub struct SerializeSettings {
    serialize_dependencies: bool,
}

impl Default for SerializeSettings {
    fn default() -> Self {
        Self {
            serialize_dependencies: false,
        }
    }
}

pub trait ObjectSerialize {
    fn serialize(&self, serialize_settings: &SerializeSettings) -> (Chunk, Ref);
}
