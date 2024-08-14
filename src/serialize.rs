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
use pdf_writer::{Chunk, Filter, Pdf, Ref};
use siphasher::sip128::{Hasher128, SipHasher13};
use skrifa::instance::Location;
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

#[derive(Copy, Clone, Debug)]
pub struct SvgSettings {
    pub raster_scale: f32,
    pub embed_text: bool,
}

impl Default for SvgSettings {
    fn default() -> Self {
        Self {
            raster_scale: 1.5,
            embed_text: true,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SerializeSettings {
    pub ascii_compatible: bool,
    pub compress_content_streams: bool,
    pub no_device_cs: bool,
    pub svg_settings: SvgSettings,
}

impl SerializeSettings {
    #[cfg(test)]
    pub fn default_test() -> Self {
        Self {
            ascii_compatible: true,
            compress_content_streams: false,
            no_device_cs: false,
            svg_settings: SvgSettings::default(),
        }
    }
}

impl Default for SerializeSettings {
    fn default() -> Self {
        Self {
            ascii_compatible: true,
            compress_content_streams: true,
            no_device_cs: false,
            svg_settings: SvgSettings::default(),
        }
    }
}

pub trait Object: Sized + 'static {
    fn serialize_into(self, sc: &mut SerializerContext) -> (Ref, Chunk);

    fn serialize_chunk(self, sc: &mut SerializerContext) -> Chunk {
        let (_, chunk) = self.serialize_into(sc);
        chunk
    }
}

pub trait RegisterableObject: Object + Hash {}

#[derive(Debug)]
pub struct SerializerContext {
    font_cache: HashMap<Arc<FontInfo>, Font>,
    font_map: HashMap<Font, FontContainer>,
    catalog_ref: Ref,
    page_tree_ref: Ref,
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
        let catalog_ref = cur_ref.bump();
        let page_tree_ref = cur_ref.bump();
        Self {
            cached_mappings: HashMap::new(),
            font_cache: HashMap::new(),
            cur_ref,
            chunks: Vec::new(),
            page_tree_ref,
            catalog_ref,
            page_refs: vec![],
            chunks_len: 0,
            font_map: HashMap::new(),
            serialize_settings,
        }
    }

    pub fn page_tree_ref(&self) -> Ref {
        self.page_tree_ref
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

    pub fn map_glyph(&mut self, font: Font, glyph: Glyph) -> (FontResource, PDFGlyph) {
        let font_container = self.font_map.entry(font.clone()).or_insert_with(|| {
            self.font_cache
                .insert(font.font_info().clone(), font.clone());

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
                    FontResource::new(font, pdf_index),
                    PDFGlyph::ColorGlyph(glyph_id),
                )
            }
            FontContainer::CIDFont(cid) => {
                let new_gid = cid.register(&glyph);
                (
                    FontResource::new(font, 0),
                    PDFGlyph::CID(new_gid.to_u32() as u16),
                )
            }
        }
    }

    pub fn convert_fontdb(&mut self, db: &mut Database, ids: Option<Vec<ID>>) -> HashMap<ID, Font> {
        let mut map = HashMap::new();

        let ids = ids.unwrap_or(db.faces().map(|f| f.id).collect::<Vec<_>>());

        for id in ids {
            // What we could do is just go through each font and then create a new Font object for each of them.
            // However, this is somewhat wasteful and expensive, because we have to hash each font, which
            // can go be multiple MB. So instead, we first construct a font info object, which is much
            // cheaper, and then check whether we already have a corresponding font object in the cache.
            // If not, we still need to construct it.
            if let Some((font_data, index)) = unsafe { db.make_shared_face_data(id) } {
                let location = Location::default();

                if let Some(font_info) =
                    FontInfo::new(font_data.as_ref().as_ref(), index, location.clone())
                {
                    // TODO: Prevent font info from being computed twice?
                    let font = self
                        .font_cache
                        .get(&Arc::new(font_info))
                        .cloned()
                        .unwrap_or(Font::new(font_data, index, location).unwrap());
                    map.insert(id, font);
                }
            }
        }

        map
    }

    pub fn get_pdf_font(&self, font_resource: &FontResource) -> Option<PdfFont> {
        self.font_map.get(&font_resource.font).map(|f| match f {
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
        if self.serialize_settings.ascii_compatible {
            (hex_encode(stream), Filter::AsciiHexDecode)
        } else {
            (deflate(stream), Filter::FlateDecode)
        }
    }

    // Always needs to be called.
    pub fn finish(mut self) -> Pdf {
        // Write fonts
        // TODO: Make more efficient
        let fonts = std::mem::take(&mut self.font_map);
        for (font, font_container) in fonts {
            match font_container {
                FontContainer::Type3(font_mapper) => {
                    for (pdf_index, mapper) in font_mapper.fonts.into_iter().enumerate() {
                        let ref_ = self.add_font(FontResource::new(font.clone(), pdf_index));
                        let chunk = mapper.serialize_into(&mut self, ref_);
                        self.push_chunk(chunk)
                    }
                }
                FontContainer::CIDFont(cid_font) => {
                    let ref_ = self.add_font(FontResource::new(font, 0));
                    let chunk = cid_font.serialize_into(&mut self, ref_);
                    self.push_chunk(chunk)
                }
            }
        }

        let mut pdf = Pdf::new();

        if self.serialize_settings.ascii_compatible {
            pdf.set_binary_marker(&[b'A', b'A', b'A', b'A']);
        }

        // This basically just exists so that for unit tests, we don't print the catalog
        // and page tree.
        if !self.page_refs.is_empty() {
            let mut page_tree_chunk = Chunk::new();

            page_tree_chunk
                .pages(self.page_tree_ref)
                .count(self.page_refs.len() as i32)
                .kids(self.page_refs);

            self.chunks_len += page_tree_chunk.len();
            pdf.catalog(self.catalog_ref).pages(self.page_tree_ref);
            pdf.extend(&page_tree_chunk);
        }

        for part_chunk in self.chunks.drain(..) {
            pdf.extend(&part_chunk);
        }

        pdf
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
