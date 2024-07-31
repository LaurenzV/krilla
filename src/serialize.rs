use crate::font::Font;
use crate::object::cid_font::CIDFont;
use crate::object::color_space::ColorSpace;
use crate::object::type3_font::Type3Font;
use crate::resource::FontResource;
use pdf_writer::{Chunk, Pdf, Ref};
use siphasher::sip128::{Hasher128, SipHasher13};
use skrifa::GlyphId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use tiny_skia_path::Size;

#[derive(Copy, Clone, Debug)]
pub struct SerializeSettings {
    pub serialize_dependencies: bool,
    pub compress: bool,
}

impl Default for SerializeSettings {
    fn default() -> Self {
        Self {
            serialize_dependencies: false,
            compress: false,
        }
    }
}

pub trait Object: Sized + Hash + Clone + 'static {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref);
}

pub trait RegisterableObject: Object {}

pub trait PageSerialize: Sized {
    fn serialize(self, serialize_settings: SerializeSettings, size: Size) -> Pdf;
}

#[derive(Debug)]
pub struct SerializerContext {
    fonts: HashMap<Font, FontContainer>,
    cached_mappings: HashMap<u128, Ref>,
    chunk: Chunk,
    cur_ref: Ref,
    pub serialize_settings: SerializeSettings,
    fonts_written: bool,
}

pub enum PDFGlyph {
    ColorGlyph(u8),
    CID(u16),
}

impl SerializerContext {
    pub fn new(serialize_settings: SerializeSettings) -> Self {
        Self {
            cached_mappings: HashMap::new(),
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

    pub fn map_glyph(&mut self, font: Font, glyph: GlyphId) -> (FontResource, PDFGlyph) {
        let font_container = self.fonts.entry(font.clone()).or_insert_with(|| {
            if font.is_type3_font() {
                FontContainer::Type3(Type3FontMapper::new(font.clone()))
            } else {
                FontContainer::CIDFont(CIDFont::new(font.clone()))
            }
        });

        match font_container {
            FontContainer::Type3(font_mapper) => {
                let (index, glyph_id) = font_mapper.add_glyph(glyph);

                (
                    FontResource::new(font.clone(), index),
                    PDFGlyph::ColorGlyph(glyph_id),
                )
            }
            FontContainer::CIDFont(cid) => {
                let new_gid = cid.remap(glyph);
                (
                    FontResource::new(font.clone(), 0),
                    PDFGlyph::CID(new_gid.to_u32() as u16),
                )
            }
        }
    }

    pub fn chunk_mut(&mut self) -> &mut Chunk {
        &mut self.chunk
    }

    pub fn chunk(&self) -> &Chunk {
        &self.chunk
    }

    fn write_fonts(sc: Rc<RefCell<SerializerContext>>) {
        // TODO: Make more efficient
        let fonts = sc.borrow().fonts.clone();
        for (font, font_container) in fonts {
            match font_container {
                FontContainer::Type3(font_mapper) => {
                    for (index, mapper) in font_mapper.fonts.iter().enumerate() {
                        let ref_ = sc.borrow_mut().add(FontResource::new(font.clone(), index));
                        mapper.clone().serialize_into(sc.clone(), ref_);
                    }
                }
                FontContainer::CIDFont(cid_font) => {
                    let ref_ = sc.borrow_mut().add(FontResource::new(font.clone(), 0));
                    cid_font.serialize_into(&mut sc.borrow_mut(), ref_);
                }
            }
        }
    }

    // Always needs to be called.
    pub fn finish(self) -> Chunk {
        // let chunk = self.chunk.clone();
        let celled = Rc::new(RefCell::new(self));
        Self::write_fonts(celled.clone());
        Rc::try_unwrap(celled).unwrap().into_inner().chunk
    }
}

/// Hash the item.
#[inline]
pub fn hash_item<T: Hash + ?Sized>(item: &T) -> u128 {
    // Also hash the TypeId because the type might be converted
    // through an unsized coercion.
    let mut state = SipHasher13::new();
    // TODO: Hash type ID too, like in Typst?
    item.hash(&mut state);
    state.finish128().as_u128()
}

#[derive(Clone, Debug)]
enum FontContainer {
    Type3(Type3FontMapper),
    CIDFont(CIDFont),
}

#[derive(Clone, Debug)]
pub struct Type3FontMapper {
    font: Font,
    fonts: Vec<Type3Font>,
}

impl Type3FontMapper {
    pub fn new(font: Font) -> Type3FontMapper {
        Self {
            font,
            fonts: Vec::new(),
        }
    }
}

impl Type3FontMapper {
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
