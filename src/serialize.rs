use crate::font::{Font, FontInfo, Glyph};
use crate::object::cid_font::CIDFont;
use crate::object::color_space::luma::SGray;
use crate::object::color_space::rgb::Srgb;
use crate::object::color_space::{DEVICE_GRAY, DEVICE_RGB};
use crate::object::type3_font::Type3Font;
use crate::resource::{ColorSpaceEnum, FontResource};
use crate::stream::PdfFont;
use crate::util::NameExt;
use fontdb::{Database, ID};
use pdf_writer::{Chunk, Filter, Name, Pdf, Ref};
use siphasher::sip128::{Hasher128, SipHasher13};
use skrifa::instance::Location;
use skrifa::FontRef;
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

#[derive(Copy, Clone, Debug)]
pub struct SerializeSettings {
    pub hex_encode_binary_streams: bool,
    pub compress_content_streams: bool,
    pub no_device_cs: bool,
}

impl Default for SerializeSettings {
    fn default() -> Self {
        Self {
            hex_encode_binary_streams: true,
            compress_content_streams: true,
            no_device_cs: false,
        }
    }
}

pub trait Object: Sized + Hash + 'static {
    fn serialize_into(self, sc: &mut SerializerContext) -> (Ref, Chunk);

    fn serialize_chunk(self, sc: &mut SerializerContext) -> Chunk {
        let (_, chunk) = self.serialize_into(sc);
        chunk
    }
}

pub trait RegisterableObject: Object {}

#[derive(Debug)]
pub struct SerializerContext {
    font_info_to_id: HashMap<Arc<FontInfo>, ID>,
    fonts: HashMap<ID, FontContainer>,
    catalog_ref: Option<Ref>,
    page_tree_ref: Option<Ref>,
    page_refs: Vec<Ref>,
    cached_mappings: HashMap<u128, Ref>,
    chunks: Vec<Chunk>,
    chunks_len: usize,
    cur_ref: Ref,
    pub serialize_settings: SerializeSettings,
}

pub enum PDFGlyph {
    ColorGlyph(u8),
    CID(u16),
}

impl PDFGlyph {
    pub fn get(&self) -> u16 {
        match self {
            PDFGlyph::ColorGlyph(n) => *n as u16,
            PDFGlyph::CID(n) => *n,
        }
    }
}

impl SerializerContext {
    pub fn new(serialize_settings: SerializeSettings) -> Self {
        let mut cur_ref = Ref::new(1);
        let catalog_ref = Some(cur_ref.bump());
        let page_tree_ref = Some(cur_ref.bump());
        Self {
            cached_mappings: HashMap::new(),
            font_info_to_id: HashMap::new(),
            cur_ref,
            chunks: Vec::new(),
            page_tree_ref,
            catalog_ref,
            page_refs: vec![],
            chunks_len: 0,
            fonts: HashMap::new(),
            serialize_settings,
        }
    }

    pub fn page_tree_ref(&self) -> Ref {
        self.page_tree_ref.unwrap()
    }

    pub fn add_page_ref(&mut self, ref_: Ref) {
        self.page_refs.push(ref_);
    }

    pub fn new_ref(&mut self) -> Ref {
        self.cur_ref.bump()
    }

    pub fn rgb(&mut self) -> CSWrapper {
        if self.serialize_settings.no_device_cs {
            CSWrapper::Ref(self.add(ColorSpaceEnum::Srgb(Srgb)))
        } else {
            CSWrapper::Name(DEVICE_RGB.to_pdf_name())
        }
    }

    pub fn gray(&mut self) -> CSWrapper {
        if self.serialize_settings.no_device_cs {
            CSWrapper::Ref(self.add(ColorSpaceEnum::SGray(SGray)))
        } else {
            CSWrapper::Name(DEVICE_GRAY.to_pdf_name())
        }
    }

    pub fn add<T>(&mut self, object: T) -> Ref
    where
        T: RegisterableObject,
    {
        let hash = hash_item(&object);
        if let Some(_ref) = self.cached_mappings.get(&hash) {
            *_ref
        } else {
            let (root_ref, chunk) = object.serialize_into(self);
            self.cached_mappings.insert(hash, root_ref);
            self.chunks_len += chunk.len();
            self.chunks.push(chunk);
            root_ref
        }
    }

    pub fn add_font<T>(&mut self, object: T) -> Ref
    where
        T: RegisterableObject,
    {
        let hash = hash_item(&object);
        if let Some(_ref) = self.cached_mappings.get(&hash) {
            *_ref
        } else {
            let root_ref = self.new_ref();
            self.cached_mappings.insert(hash, root_ref);
            root_ref
        }
    }

    pub fn map_glyph(
        &mut self,
        font_id: ID,
        fontdb: &mut Database,
        glyph: Glyph,
    ) -> (FontResource, PDFGlyph) {
        let font_container = self.fonts.entry(font_id).or_insert_with(|| {
            let (font_ref, index) = unsafe { fontdb.make_shared_face_data(font_id).unwrap() };
            let font = Font::new(font_ref, index, Location::default()).unwrap();
            // TODO: Overthink this, does it really work? Do we need to expose font info directly?
            self.font_info_to_id
                .insert(font.font_info().clone(), font_id);

            if font.is_type3_font() {
                FontContainer::Type3(Type3FontMapper::new(font.clone()))
            } else {
                FontContainer::CIDFont(CIDFont::new(font.clone()))
            }
        });

        match font_container {
            FontContainer::Type3(font_mapper) => {
                let (pdf_index, glyph_id) = font_mapper.add_glyph(glyph);

                (
                    FontResource::new(font_id, pdf_index),
                    PDFGlyph::ColorGlyph(glyph_id),
                )
            }
            FontContainer::CIDFont(cid) => {
                let new_gid = cid.remap(&glyph);
                (
                    FontResource::new(font_id, 0),
                    PDFGlyph::CID(new_gid.to_u32() as u16),
                )
            }
        }
    }

    pub fn get_pdf_font(&self, font_resource: &FontResource) -> Option<PdfFont> {
        self.fonts.get(&font_resource.font_id).map(|f| match f {
            FontContainer::Type3(fm) => PdfFont::Type3(&fm.fonts[font_resource.pdf_index]),
            FontContainer::CIDFont(cid) => PdfFont::CID(cid),
        })
    }

    fn push_chunk(&mut self, chunk: Chunk) {
        self.chunks_len += chunk.len();
        self.chunks.push(chunk);
    }

    pub fn get_content_stream<'a>(&self, stream: &'a [u8]) -> (Cow<'a, [u8]>, Option<Filter>) {
        if !self.serialize_settings.compress_content_streams {
            (Cow::Borrowed(stream), None)
        } else {
            let (stream, filter) = self.get_binary_stream(stream);
            (Cow::Owned(stream), Some(filter))
        }
    }

    pub fn get_binary_stream(&self, stream: &[u8]) -> (Vec<u8>, Filter) {
        if self.serialize_settings.hex_encode_binary_streams {
            (hex_encode(stream), Filter::AsciiHexDecode)
        } else {
            (deflate(stream), Filter::FlateDecode)
        }
    }

    // Always needs to be called.
    pub fn finish(mut self, fontdb: &Database) -> Pdf {
        // Write fonts
        // TODO: Make more efficient
        let fonts = std::mem::take(&mut self.fonts);
        for (font_id, font_container) in fonts {
            fontdb
                .with_face_data(font_id, |data, index| {
                    let font_ref = FontRef::from_index(data, index).unwrap();

                    match font_container {
                        FontContainer::Type3(font_mapper) => {
                            for (pdf_index, mapper) in font_mapper.fonts.into_iter().enumerate() {
                                let ref_ = self.add_font(FontResource::new(font_id, pdf_index));
                                let chunk = mapper.serialize_into(&mut self, &font_ref, ref_);
                                self.push_chunk(chunk)
                            }
                        }
                        FontContainer::CIDFont(cid_font) => {
                            let ref_ = self.add_font(FontResource::new(font_id, 0));
                            let chunk = cid_font.serialize_into(&mut self, &font_ref, ref_);
                            self.push_chunk(chunk)
                        }
                    }
                })
                .unwrap();
        }

        let mut pdf = Pdf::new();

        if let (Some(page_tree_ref), Some(catalog_ref)) = (self.page_tree_ref, self.catalog_ref) {
            let mut page_tree_chunk = Chunk::new();

            page_tree_chunk
                .pages(page_tree_ref)
                .count(self.page_refs.len() as i32)
                .kids(self.page_refs);

            self.chunks_len += page_tree_chunk.len();

            pdf.catalog(catalog_ref).pages(page_tree_ref);
            pdf.extend(&page_tree_chunk);
        }

        for part_chunk in self.chunks.drain(..) {
            pdf.extend(&part_chunk);
        }

        pdf
    }

    #[cfg(test)]
    pub(crate) fn new_unit_test() -> Self {
        let ss = SerializeSettings {
            hex_encode_binary_streams: true,
            compress_content_streams: false,
            no_device_cs: false,
        };

        let cur_ref = Ref::new(1);
        Self {
            cached_mappings: HashMap::new(),
            font_info_to_id: HashMap::new(),
            cur_ref,
            chunks: Vec::new(),
            // Just a dummy.
            page_tree_ref: None,
            // Just a dummy.
            catalog_ref: None,
            page_refs: vec![],
            chunks_len: 0,
            fonts: HashMap::new(),
            serialize_settings: ss,
        }
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

#[derive(Copy, Clone)]
pub enum CSWrapper {
    Ref(pdf_writer::Ref),
    Name(pdf_writer::Name<'static>),
}

impl pdf_writer::Primitive for CSWrapper {
    fn write(self, buf: &mut Vec<u8>) {
        match self {
            CSWrapper::Ref(r) => r.write(buf),
            CSWrapper::Name(n) => n.write(buf),
        }
    }
}

#[derive(Debug)]
enum FontContainer {
    Type3(Type3FontMapper),
    CIDFont(CIDFont),
}

#[derive(Debug)]
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
    pub fn add_glyph(&mut self, glyph: Glyph) -> (usize, u8) {
        if let Some(index) = self.fonts.iter().position(|f| f.covers(glyph.glyph_id)) {
            return (index, self.fonts[index].add(&glyph));
        }

        let glyph_id = if let Some(last_font) = self.fonts.last_mut() {
            if last_font.is_full() {
                let mut font = Type3Font::new(self.font.clone());
                let gid = font.add(&glyph);
                self.fonts.push(font);
                gid
            } else {
                last_font.add(&glyph)
            }
        } else {
            let mut font = Type3Font::new(self.font.clone());
            let gid = font.add(&glyph);
            self.fonts.push(font);
            gid
        };

        (self.fonts.len() - 1, glyph_id)
    }

    pub fn index(&self) -> u32 {
        self.font.index()
    }
}

fn deflate(data: &[u8]) -> Vec<u8> {
    const COMPRESSION_LEVEL: u8 = 6;
    miniz_oxide::deflate::compress_to_vec_zlib(data, COMPRESSION_LEVEL)
}

fn hex_encode(data: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(index, byte)| {
            let mut formatted = format!("{:02X}", byte);
            if index % 35 == 34 {
                formatted.push('\n');
            }
            formatted
        })
        .collect::<String>()
        .into_bytes()
}
