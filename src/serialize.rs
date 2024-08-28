use crate::font::{Font, FontIdentifier, FontInfo};
use crate::object::cid_font::CIDFont;
use crate::object::color_space::luma::SGray;
use crate::object::color_space::rgb::Srgb;
use crate::object::color_space::{DEVICE_GRAY, DEVICE_RGB};
use crate::object::outline::Outline;
use crate::object::page::{Page, PageLabelContainer};
use crate::object::type3_font::Type3FontMapper;
use crate::resource::ColorSpaceEnum;
use crate::stream::PdfFont;
use crate::util::NameExt;
use fontdb::{Database, ID};
use pdf_writer::{Chunk, Filter, Finish, Name, Pdf, Ref};
use siphasher::sip128::{Hasher128, SipHasher13};
use skrifa::instance::Location;
use skrifa::raw::TableProvider;
use skrifa::GlyphId;
use std::any::Any;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tiny_skia_path::Rect;

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
    pub force_type3_fonts: bool,
    pub svg_settings: SvgSettings,
}

impl SerializeSettings {
    #[cfg(test)]
    pub fn default_test() -> Self {
        Self {
            ascii_compatible: true,
            compress_content_streams: false,
            no_device_cs: false,
            force_type3_fonts: false,
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
            force_type3_fonts: false,
            svg_settings: SvgSettings::default(),
        }
    }
}

pub trait Object: SipHashable {
    fn chunk_container(&self, cc: &mut ChunkContainer) -> &mut Vec<ChunkMap>;

    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk;

    fn serialize(&self, sc: &mut SerializerContext) -> Chunk {
        let root_ref = sc.new_ref();
        self.serialize_into(sc, root_ref)
    }
}

pub struct PageInfo {
    pub ref_: Ref,
    pub media_box: Rect,
}

pub struct SerializerContext {
    font_cache: HashMap<Arc<FontInfo>, Font>,
    font_map: HashMap<Font, RefCell<FontContainer>>,
    page_tree_ref: Option<Ref>,
    page_infos: Vec<PageInfo>,
    pages: Vec<(Ref, Page)>,
    outline: Option<Outline>,
    cached_mappings: HashMap<u128, Ref>,
    chunks: Vec<Chunk>,
    cur_ref: Ref,
    cur_cache_ref: Ref,
    pub serialize_settings: SerializeSettings,
}

#[derive(Clone, Copy)]
pub enum PDFGlyph {
    Type3(u8),
    CID(u16),
}

impl PDFGlyph {
    pub fn get(&self) -> u16 {
        match self {
            PDFGlyph::Type3(n) => *n as u16,
            PDFGlyph::CID(n) => *n,
        }
    }

    pub fn encode_into(&self, slice: &mut Vec<u8>) {
        match self {
            PDFGlyph::Type3(cg) => slice.push(*cg),
            PDFGlyph::CID(cid) => {
                slice.push((cid >> 8) as u8);
                slice.push((cid & 0xff) as u8);
            }
        }
    }
}

const CACHE_REF_START: i32 = 50000000;

pub type ChunkMap = (Ref, Chunk);

pub struct ChunkContainer {
    pub(crate) page_label_tree: Option<ChunkMap>,
    pub(crate) page_tree: Option<ChunkMap>,
    pub(crate) outline: Option<ChunkMap>,

    pub(crate) pages: Vec<ChunkMap>,
    pub(crate) page_labels: Vec<ChunkMap>,
    pub(crate) annotations: Vec<ChunkMap>,
    pub(crate) fonts: Vec<ChunkMap>,
    pub(crate) color_spaces: Vec<ChunkMap>,
    pub(crate) destinations: Vec<ChunkMap>,
    pub(crate) ext_g_states: Vec<ChunkMap>,
    pub(crate) images: Vec<ChunkMap>,
    pub(crate) masks: Vec<ChunkMap>,
    pub(crate) x_objects: Vec<ChunkMap>,
    pub(crate) shading_functions: Vec<ChunkMap>,
    pub(crate) patterns: Vec<ChunkMap>,
}

impl ChunkContainer {
    pub fn new() -> Self {
        Self {
            page_tree: None,
            outline: None,
            page_label_tree: None,

            pages: vec![],
            page_labels: vec![],
            annotations: vec![],
            fonts: vec![],
            color_spaces: vec![],
            destinations: vec![],
            ext_g_states: vec![],
            images: vec![],
            masks: vec![],
            x_objects: vec![],
            shading_functions: vec![],
            patterns: vec![],
        }
    }

    pub fn finish(self, serialize_settings: &SerializeSettings) -> Pdf {
        let mut remapped_ref = Ref::new(1);
        let mut remapper = HashMap::new();

        macro_rules! remap_field {
            ($self:expr, $remapper:expr, $remapped_ref:expr; $($field:ident),+) => {
                $(
                    if let Some($field) = &$self.$field {
                        debug_assert!($field.0.get() >= CACHE_REF_START);

                        $remapper.insert($field.0, $remapped_ref.bump());
                    }
                )+
            };
        }

        macro_rules! remap_fields {
            ($self:expr, $remapper:expr, $remapped_ref:expr; $($field:ident),+) => {
                $(
                    for el in &$self.$field {
                        debug_assert!(el.0.get() >= CACHE_REF_START);

                        $remapper.insert(el.0, $remapped_ref.bump());
                    }
                )+
            };
        }

        // Chunk length is not an exact number because the length might change as we renumber,
        // so we add a bit of a buffer, which should hopefully always be enough
        // let mut pdf = Pdf::with_capacity((self.chunks_len as f32 * 1.1) as usize);
        let mut pdf = Pdf::new();

        if serialize_settings.ascii_compatible {
            pdf.set_binary_marker(&[b'A', b'A', b'A', b'A'])
        }

        // We only write a catalog if a page tree exists. Every valid PDF must have one
        // and krilla ensures that there always is one, but for snapshot tests, it can be
        // useful to not write a document catalog if we don't actually need it for the test.
        if self.page_tree.is_some() || self.page_label_tree.is_some() || self.outline.is_some() {
            let catalog_ref = remapped_ref.bump();

            let mut catalog = pdf.catalog(catalog_ref);
            remap_field!(self, remapper, remapped_ref; page_tree, outline, page_label_tree);

            if let Some(pt) = &self.page_tree {
                catalog.pages(pt.0);
            }

            if let Some(pl) = &self.page_label_tree {
                catalog.pair(Name(b"PageLabels"), pl.0);
            }

            if let Some(ol) = &self.outline {
                catalog.outlines(ol.0);
            }

            catalog.finish();
        }

        remap_fields!(self, remapper, remapped_ref; pages, page_labels, annotations, fonts, color_spaces, destinations, ext_g_states, images, masks, x_objects, shading_functions, patterns);

        macro_rules! write_field {
            ($self:expr, $remapper:expr, $remapped_ref:expr, $pdf:expr; $($field:ident),+) => {
                $(
                    if let Some($field) = $self.$field {
                        $field.1.renumber_into($pdf, |old| *$remapper.entry(old).or_insert_with(|| $remapped_ref.bump()));
                    }
                )+
            };
        }

        macro_rules! write_fields {
            ($self:expr, $remapper:expr, $remapped_ref:expr, $pdf:expr; $($field:ident),+) => {
                $(
                    for el in $self.$field {
                        el.1.renumber_into($pdf, |old| *$remapper.entry(old).or_insert_with(|| $remapped_ref.bump()));
                    }
                )+
            };
        }

        write_field!(self, remapper, remapped_ref, &mut pdf; page_tree, outline, page_label_tree);
        write_fields!(self, remapper, remapped_ref, &mut pdf; pages, page_labels, annotations, fonts, color_spaces, destinations, ext_g_states, images, masks, x_objects, shading_functions, patterns);

        pdf
    }
}

impl SerializerContext {
    pub fn new(serialize_settings: SerializeSettings) -> Self {
        Self {
            cached_mappings: HashMap::new(),
            font_cache: HashMap::new(),
            cur_ref: Ref::new(1),
            cur_cache_ref: Ref::new(CACHE_REF_START),
            chunks: Vec::new(),
            page_tree_ref,
            outline: None,
            page_infos: vec![],
            pages: vec![],
            font_map: HashMap::new(),
            serialize_settings,
        }
    }

    pub fn page_infos(&self) -> &[PageInfo] {
        &self.page_infos
    }

    pub fn set_outline(&mut self, outline: Outline) {
        self.outline = Some(outline);
    }

    pub fn page_tree_ref(&mut self) -> Ref {
        *self
            .page_tree_ref
            .get_or_insert_with(|| self.cur_cache_ref.bump())
    }

    pub fn add_page(&mut self, page: Page) {
        let ref_ = self.new_ref();
        self.page_infos.push(PageInfo {
            ref_,
            media_box: page.media_box,
        });
        self.pages.push((ref_, page));
    }

    pub fn new_ref(&mut self) -> Ref {
        assert!(self.cur_ref.get() < CACHE_REF_START);

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
        let hash = object.sip_hash();
        if let Some(_ref) = self.cached_mappings.get(&hash) {
            *_ref
        } else {
            let root_ref = self.new_ref();
            let chunk = object.serialize_into(self, root_ref);
            self.cached_mappings.insert(hash, root_ref);
            self.chunks_len += chunk.len();
            self.chunks.push(chunk);
            root_ref
        }
    }

    pub fn create_or_get_font_container(&mut self, font: Font) -> &RefCell<FontContainer> {
        self.font_map.entry(font.clone()).or_insert_with(|| {
            self.font_cache
                .insert(font.font_info().clone(), font.clone());

            // Right now, we decide whether to embed a font as a Type3 font
            // solely based on whether one of these tables exist (or if
            // the settings tell us to force it). This is not the most "efficient"
            // method, because it is possible a font has a `COLR` table, but
            // there are still some glyphs which are not in COLR but in `glyf`
            // or `CFF`. In this case, we would still choose a Type3 font for
            // the outlines, even though they could be embedded as a CID font.
            // For now, we make the simplifying assumption that a font is either mapped
            // to a series of Type3 fonts or to a single CID font, but not a mix of both.
            let font_ref = font.font_ref();
            let use_type3 = self.serialize_settings.force_type3_fonts
                || font_ref.svg().is_ok()
                || font_ref.colr().is_ok()
                || font_ref.sbix().is_ok()
                || font_ref.cff2().is_ok();

            if use_type3 {
                RefCell::new(FontContainer::Type3(Type3FontMapper::new(font.clone())))
            } else {
                RefCell::new(FontContainer::CIDFont(CIDFont::new(font.clone())))
            }
        })
    }

    // TODO: Don't use generics here
    pub fn add_font<T>(&mut self, object: T) -> Ref
    where
        T: RegisterableObject,
    {
        let hash = object.sip_hash();
        if let Some(_ref) = self.cached_mappings.get(&hash) {
            *_ref
        } else {
            let root_ref = self.new_ref();
            self.cached_mappings.insert(hash, root_ref);
            root_ref
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
                    let font_info = Arc::new(font_info);
                    let font = self
                        .font_cache
                        .get(&font_info.clone())
                        .cloned()
                        .unwrap_or(Font::new_with_info(font_data, font_info).unwrap());
                    map.insert(id, font);
                }
            }
        }

        map
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
        // TODO: Get rid of all the clones

        if let Some(container) = PageLabelContainer::new(
            self.pages
                .iter()
                .map(|(_, p)| p.page_label.clone())
                .collect::<Vec<_>>(),
        ) {
            self.page_labels_ref = Some(self.add(container));
        }

        if let Some(outline) = self.outline.clone() {
            let outline_ref = self.new_ref();
            self.outline_ref = Some(outline_ref);
            let chunk = outline.serialize_into(&mut self, outline_ref);
            self.push_chunk(chunk);
        }

        // Write fonts
        // TODO: Make more efficient
        let fonts = std::mem::take(&mut self.font_map);
        for font_container in fonts.values() {
            match &*font_container.borrow() {
                FontContainer::Type3(font_mapper) => {
                    for t3_font in font_mapper.fonts() {
                        let ref_ = self.add_font(t3_font.identifier());
                        let chunk = t3_font.serialize_into(&mut self, ref_);
                        self.push_chunk(chunk)
                    }
                }
                FontContainer::CIDFont(cid_font) => {
                    let ref_ = self.add_font(cid_font.identifier());
                    let chunk = cid_font.serialize_into(&mut self, ref_);
                    self.push_chunk(chunk)
                }
            }
        }

        let mut pdf = Pdf::new();

        if self.serialize_settings.ascii_compatible {
            pdf.set_binary_marker(&[b'A', b'A', b'A', b'A']);
        }

        let mut page_tree_chunk = Chunk::new();

        let pages = std::mem::take(&mut self.pages);
        for (ref_, page) in &pages {
            let chunk = page.serialize_into(&mut self, *ref_);
            self.push_chunk(chunk);
        }

        page_tree_chunk
            .pages(self.page_tree_ref)
            .count(pages.len() as i32)
            .kids(pages.iter().map(|(r, _)| *r));

        self.chunks_len += page_tree_chunk.len();

        let mut catalog = pdf.catalog(self.catalog_ref);
        catalog.pages(self.page_tree_ref);

        if let Some(plr) = self.page_labels_ref {
            catalog.pair(Name(b"PageLabels"), plr);
        }

        if let Some(olr) = self.outline_ref {
            catalog.outlines(olr);
        }

        catalog.finish();

        pdf.extend(&page_tree_chunk);

        for part_chunk in self.chunks.drain(..) {
            pdf.extend(&part_chunk);
        }

        pdf
    }
}

pub trait SipHashable {
    fn sip_hash(&self) -> u128;
}

impl<T> SipHashable for T
where
    T: Hash + ?Sized + 'static,
{
    fn sip_hash(&self) -> u128 {
        let mut state = SipHasher13::new();
        self.type_id().hash(&mut state);
        self.hash(&mut state);
        state.finish128().as_u128()
    }
}

#[derive(Copy, Clone)]
pub enum CSWrapper {
    Ref(Ref),
    Name(Name<'static>),
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
pub enum FontContainer {
    Type3(Type3FontMapper),
    CIDFont(CIDFont),
}

impl FontContainer {
    pub fn font_identifier(&self, glyph_id: GlyphId) -> Option<FontIdentifier> {
        match self {
            FontContainer::Type3(t3) => t3.id_from_glyph(glyph_id),
            FontContainer::CIDFont(cid) => cid.get_cid(glyph_id).map(|_| cid.identifier()),
        }
    }

    pub fn get_from_identifier_mut(
        &mut self,
        font_identifier: FontIdentifier,
    ) -> Option<&mut dyn PdfFont> {
        match self {
            FontContainer::Type3(t3) => {
                if let Some(t3_font) = t3.font_mut_from_id(font_identifier) {
                    return Some(t3_font);
                } else {
                    None
                }
            }
            FontContainer::CIDFont(cid) => {
                if cid.identifier() == font_identifier {
                    return Some(cid);
                } else {
                    None
                }
            }
        }
    }

    pub fn get_from_identifier(&self, font_identifier: FontIdentifier) -> Option<&dyn PdfFont> {
        match self {
            FontContainer::Type3(t3) => {
                if let Some(t3_font) = t3.font_from_id(font_identifier) {
                    return Some(t3_font);
                } else {
                    None
                }
            }
            FontContainer::CIDFont(cid) => {
                if cid.identifier() == font_identifier {
                    return Some(cid);
                } else {
                    None
                }
            }
        }
    }

    pub fn add_glyph(&mut self, glyph_id: GlyphId) -> (FontIdentifier, PDFGlyph) {
        match self {
            FontContainer::Type3(t3) => {
                let (identifier, gid) = t3.add_glyph(glyph_id);
                (identifier, PDFGlyph::Type3(gid))
            }
            FontContainer::CIDFont(cid_font) => {
                let cid = cid_font.add_glyph(glyph_id);
                (cid_font.identifier(), PDFGlyph::CID(cid))
            }
        }
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
