use crate::ext_g_state::ExtGState;
use crate::mask::Mask;
use crate::object::color_space::ColorSpace;
use crate::resource::{PdfColorSpace, PdfPattern};
use crate::shading::{ShadingFunction, ShadingPattern};
use pdf_writer::{Chunk, Pdf, Ref};
use std::collections::HashMap;
use std::hash::Hash;
// TODO: Add marker trait for cacheable object

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum CacheableObject {
    PdfColorSpace(ColorSpace),
    ExtGState(ExtGState),
    ShadingFunction(ShadingFunction),
    PdfPattern(PdfPattern),
    Mask(Mask),
}

impl ObjectSerialize for CacheableObject {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        match self {
            CacheableObject::PdfColorSpace(cs) => cs.serialize_into(sc, root_ref),
            CacheableObject::ExtGState(st) => st.serialize_into(sc, root_ref),
            CacheableObject::ShadingFunction(sf) => sf.serialize_into(sc, root_ref),
            CacheableObject::PdfPattern(pp) => pp.serialize_into(sc, root_ref),
            CacheableObject::Mask(ma) => ma.serialize_into(sc, root_ref),
        }
    }
}

#[derive(Copy, Clone)]
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
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref);

    fn serialize(self, serialize_settings: SerializeSettings) -> (Chunk, Ref) {
        let mut sc = SerializerContext::new(serialize_settings);
        let root_ref = sc.new_ref();
        self.serialize_into(&mut sc, root_ref);
        (sc.chunk, root_ref)
    }
}

pub trait PageSerialize: Sized {
    fn serialize(self, serialize_settings: SerializeSettings) -> Pdf;
}

pub struct SerializerContext {
    cached_mappings: HashMap<CacheableObject, Ref>,
    chunk: Chunk,
    cur_ref: Ref,
    serialize_settings: SerializeSettings,
}

impl SerializerContext {
    pub fn new(serialize_settings: SerializeSettings) -> Self {
        Self {
            cached_mappings: HashMap::new(),
            chunk: Chunk::new(),
            cur_ref: Ref::new(1),
            serialize_settings,
        }
    }

    pub fn add_cached(&mut self, object: CacheableObject) -> Ref {
        if let Some(_ref) = self.cached_mappings.get(&object) {
            *_ref
        } else {
            let root_ref = self.new_ref();
            self.cached_mappings.insert(object.clone(), root_ref);
            object.serialize_into(self, root_ref);
            root_ref
        }
    }

    pub fn srgb_ref(&mut self) -> Ref {
        self.add_cached(CacheableObject::PdfColorSpace(ColorSpace::SRGB))
    }

    pub fn d65gray_ref(&mut self) -> Ref {
        self.add_cached(CacheableObject::PdfColorSpace(ColorSpace::D65Gray))
    }

    pub fn new_ref(&mut self) -> Ref {
        self.cur_ref.bump()
    }

    pub fn add_uncached<T>(&mut self, o: T) -> Ref
    where
        T: ObjectSerialize,
    {
        let _ref = self.new_ref();
        o.serialize_into(self, _ref);
        _ref
    }

    pub fn current_chunk(&self) -> &Chunk {
        &self.chunk
    }

    pub fn chunk_mut(&mut self) -> &mut Chunk {
        &mut self.chunk
    }
}
