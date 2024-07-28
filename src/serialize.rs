use crate::object::color_space::ColorSpace;
use pdf_writer::{Chunk, Pdf, Ref};
use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;

use crate::font::Font;
use crate::object::type3_font::Type3Font;
use siphasher::sip128::{Hasher128, SipHasher13};
use skrifa::GlyphId;

#[derive(Copy, Clone)]
pub struct SerializeSettings {
    pub serialize_dependencies: bool,
    pub compress: bool,
}

impl Default for SerializeSettings {
    fn default() -> Self {
        Self {
            serialize_dependencies: false,
            compress: true,
        }
    }
}

pub trait Object: Sized + Hash + Clone + 'static {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref);

    fn serialize(self, serialize_settings: SerializeSettings) -> (Chunk, Ref) {
        let mut sc = SerializerContext::new(serialize_settings);
        let root_ref = sc.new_ref();
        self.serialize_into(&mut sc, root_ref);
        (sc.finish(), root_ref)
    }
}

pub trait RegisterableObject: Object {}

pub trait PageSerialize: Sized {
    fn serialize(self, serialize_settings: SerializeSettings) -> Pdf;
}

pub struct SerializerContext {
    fonts: HashMap<Font, FontMapper>,
    font_refs: HashMap<(Font, usize), Ref>,
    cached_mappings: HashMap<u128, Ref>,
    chunk: Chunk,
    cur_ref: Ref,
    serialize_settings: SerializeSettings,
    fonts_written: bool,
}

impl SerializerContext {
    pub fn new(serialize_settings: SerializeSettings) -> Self {
        Self {
            cached_mappings: HashMap::new(),
            font_refs: HashMap::new(),
            chunk: Chunk::new(),
            cur_ref: Ref::new(1),
            fonts: HashMap::new(),
            serialize_settings,
            fonts_written: false,
        }
    }

    pub fn new_ref(&mut self) -> Ref {
        self.cur_ref.bump()
    }

    pub fn srgb(&mut self) -> Ref {
        self.add(ColorSpace::SRGB)
    }

    pub fn d65_gray(&mut self) -> Ref {
        self.add(ColorSpace::D65Gray)
    }

    pub fn add<T>(&mut self, object: T) -> Ref
    where
        T: RegisterableObject,
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

    pub fn get_font(&mut self, font: Font, glyph: GlyphId) -> (usize, u8) {
        let font_mapper = self
            .fonts
            .entry(font.clone())
            .or_insert_with(|| FontMapper::new(font));
        font_mapper.add_glyph(glyph)
    }

    pub fn chunk_mut(&mut self) -> &mut Chunk {
        &mut self.chunk
    }

    pub fn chunk(&self) -> &Chunk {
        &self.chunk
    }

    // TODO: Somehow make sure that the caller has to call this, so that the fonts are always written.
    pub fn finish(self) -> Chunk {
        // TODO: Write fonts
        self.chunk
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

pub struct FontMapper {
    font: Font,
    fonts: Vec<Type3Font>,
}

impl FontMapper {
    pub fn new(font: Font) -> FontMapper {
        Self {
            font,
            fonts: Vec::new(),
        }
    }
}

impl FontMapper {
    pub fn add_glyph(&mut self, glyph_id: GlyphId) -> (usize, u8) {
        if let Some(index) = self.fonts.iter().position(|f| f.covers(glyph_id)) {
            return (index, self.fonts[index].add(glyph_id));
        }

        let glyph_id = if let Some(last_font) = self.fonts.last_mut() {
            if last_font.is_full() {
                let mut font = Type3Font::new(self.font.clone());
                let gid = font.add(glyph_id);
                self.fonts.push(font);
                gid
            } else {
                last_font.add(glyph_id)
            }
        } else {
            let mut font = Type3Font::new(self.font.clone());
            let gid = font.add(glyph_id);
            self.fonts.push(font);
            gid
        };

        (self.fonts.len() - 1, glyph_id)
    }
}
