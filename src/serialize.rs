use crate::object::color_space::ColorSpace;
use pdf_writer::{Chunk, Pdf, Ref};
use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;

use siphasher::sip128::{Hasher128, SipHasher13};

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

pub trait Object: Sized + Hash + 'static {
    const CACHED: bool;

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
    cached_mappings: HashMap<u128, Ref>,
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

    pub fn new_ref(&mut self) -> Ref {
        self.cur_ref.bump()
    }

    fn add_cached<T>(&mut self, object: T) -> Ref
    where
        T: Object,
    {
        let hash = hash_item(&object);
        if let Some(_ref) = self.cached_mappings.get(&hash) {
            *_ref
        } else {
            let root_ref = self.new_ref();
            self.cached_mappings.insert(hash, root_ref);
            object.serialize_into(self, root_ref);
            root_ref
        }
    }

    pub fn srgb(&mut self) -> Ref {
        self.add(ColorSpace::SRGB)
    }

    pub fn d65_gray(&mut self) -> Ref {
        self.add(ColorSpace::D65Gray)
    }

    fn add_uncached<T>(&mut self, object: T) -> Ref
    where
        T: Object,
    {
        let _ref = self.new_ref();
        object.serialize_into(self, _ref);
        _ref
    }

    pub fn add<T>(&mut self, object: T) -> Ref
    where
        T: Object,
    {
        if T::CACHED {
            self.add_cached(object)
        } else {
            self.add_uncached(object)
        }
    }

    pub fn chunk_mut(&mut self) -> &mut Chunk {
        &mut self.chunk
    }
}

/// Hash the item.
#[inline]
fn hash_item<T: Hash + ?Sized + 'static>(item: &T) -> u128 {
    // Also hash the TypeId because the type might be converted
    // through an unsized coercion.
    let mut state = SipHasher13::new();
    item.type_id().hash(&mut state);
    item.hash(&mut state);
    state.finish128().as_u128()
}
