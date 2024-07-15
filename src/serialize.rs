use crate::ext_g_state::ExtGState;
use crate::function::{ExponentialFunction, PdfFunction, StitchingFunction};
use crate::resource::PdfColorSpace;
use pdf_writer::{Chunk, Pdf, Ref};
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum PdfObject {
    PdfColorSpace(PdfColorSpace),
    ExtGState(ExtGState),
    ExponentialFunction(ExponentialFunction),
    StitchingFunction(StitchingFunction),
    PdfFunction(PdfFunction),
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

    pub fn cached_ref(&mut self, object: PdfObject) -> Ref {
        let mappings = &mut self.mappings;
        let _ref = &mut self._ref;
        *mappings.entry(object.clone()).or_insert(_ref.bump())
    }

    pub fn new_ref(&mut self) -> Ref {
        self._ref.bump()
    }
}

pub struct SerializeSettings {
    pub serialize_dependencies: bool,
}

impl Default for SerializeSettings {
    fn default() -> Self {
        Self {
            serialize_dependencies: false,
        }
    }
}

pub trait ObjectSerialize: Sized {
    fn serialize_into(
        self,
        chunk: &mut Chunk,
        ref_allocator: &mut RefAllocator,
        serialize_settings: &SerializeSettings,
    ) -> Ref;

    fn serialize(self, serialize_settings: &SerializeSettings) -> (Chunk, Ref) {
        let mut chunk = Chunk::new();
        let mut ref_allocator = RefAllocator::new();
        let _ref = self.serialize_into(&mut chunk, &mut ref_allocator, serialize_settings);
        (chunk, _ref)
    }
}

pub trait PageSerialize: Sized {
    fn serialize(self, serialize_settings: &SerializeSettings) -> Pdf;
}
