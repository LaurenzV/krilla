use crate::chunk_container::ChunkContainer;
use crate::font::{Font, FontIdentifier, FontInfo};
use crate::object::cid_font::CIDFont;
use crate::object::color_space::luma::SGray;
use crate::object::color_space::rgb::Srgb;
use crate::object::color_space::{DEVICE_GRAY, DEVICE_RGB};
use crate::object::outline::Outline;
use crate::object::page::{Page, PageLabelContainer};
use crate::object::type3_font::Type3FontMapper;
use crate::page::PageLabel;
use crate::resource::{ColorSpaceResource, Resource};
use crate::stream::PdfFont;
use crate::util::NameExt;
use fontdb::{Database, ID};
use pdf_writer::{Chunk, Filter, Name, Pdf, Ref};
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

#[cfg(test)]
impl SerializeSettings {
    pub fn settings_1() -> Self {
        Self {
            ascii_compatible: true,
            compress_content_streams: false,
            no_device_cs: false,
            force_type3_fonts: false,
            svg_settings: SvgSettings::default(),
        }
    }

    pub fn settings_2() -> Self {
        Self {
            no_device_cs: true,
            ..Self::settings_1()
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
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk>;

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
    cur_ref: Ref,
    chunk_container: ChunkContainer,
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

impl SerializerContext {
    pub fn new(serialize_settings: SerializeSettings) -> Self {
        Self {
            cached_mappings: HashMap::new(),
            font_cache: HashMap::new(),
            cur_ref: Ref::new(1),
            chunk_container: ChunkContainer::new(),
            page_tree_ref: None,
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
            .get_or_insert_with(|| self.cur_ref.bump())
    }

    pub fn add_page(&mut self, page: Page) {
        let ref_ = self.new_ref();
        self.page_infos.push(PageInfo {
            ref_,
            media_box: page.media_box,
        });
        self.pages.push((ref_, page));
    }

    pub fn has_pages(&self) -> bool {
        !self.page_infos.is_empty()
    }

    pub fn new_ref(&mut self) -> Ref {
        self.cur_ref.bump()
    }

    pub fn rgb(&mut self) -> CSWrapper {
        if self.serialize_settings.no_device_cs {
            CSWrapper::Ref(self.add_object(ColorSpaceResource::Srgb(Srgb)))
        } else {
            CSWrapper::Name(DEVICE_RGB.to_pdf_name())
        }
    }

    pub fn gray(&mut self) -> CSWrapper {
        if self.serialize_settings.no_device_cs {
            CSWrapper::Ref(self.add_object(ColorSpaceResource::SGray(SGray)))
        } else {
            CSWrapper::Name(DEVICE_GRAY.to_pdf_name())
        }
    }

    pub fn add_object<T>(&mut self, object: T) -> Ref
    where
        T: Object,
    {
        let hash = object.sip_hash();
        if let Some(_ref) = self.cached_mappings.get(&hash) {
            *_ref
        } else {
            let root_ref = self.new_ref();
            let chunk = object.serialize_into(self, root_ref);
            self.cached_mappings.insert(hash, root_ref);
            object
                .chunk_container(&mut self.chunk_container)
                .push(chunk);
            root_ref
        }
    }

    pub fn add_page_label(&mut self, page_label: PageLabel) -> Ref {
        let ref_ = self.new_ref();
        let chunk = page_label.serialize_into(ref_);
        self.chunk_container.page_labels.push(chunk);
        ref_
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

    pub(crate) fn add_resource(&mut self, resource: impl Into<Resource>) -> Ref {
        match resource.into() {
            Resource::XObject(xr) => self.add_object(xr),
            Resource::Pattern(pr) => self.add_object(pr),
            Resource::ExtGState(e) => self.add_object(e),
            Resource::ColorSpace(csr) => self.add_object(csr),
            Resource::Shading(s) => self.add_object(s),
            Resource::Font(f) => {
                let hash = f.sip_hash();
                if let Some(_ref) = self.cached_mappings.get(&hash) {
                    *_ref
                } else {
                    let root_ref = self.new_ref();
                    self.cached_mappings.insert(hash, root_ref);
                    root_ref
                }
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
            &self
                .pages
                .iter()
                .map(|(_, p)| p.page_label.clone())
                .collect::<Vec<_>>(),
        ) {
            let page_label_tree_ref = self.new_ref();
            let chunk = container.serialize_into(&mut self, page_label_tree_ref);
            self.chunk_container.page_label_tree = Some((page_label_tree_ref, chunk));
        }

        let outline = std::mem::take(&mut self.outline);
        if let Some(outline) = &outline {
            let outline_ref = self.new_ref();
            let chunk = outline.serialize_into(&mut self, outline_ref);
            self.chunk_container.outline = Some((outline_ref, chunk));
        }

        let fonts = std::mem::take(&mut self.font_map);
        for font_container in fonts.values() {
            match &*font_container.borrow() {
                FontContainer::Type3(font_mapper) => {
                    for t3_font in font_mapper.fonts() {
                        let ref_ = self.add_resource(t3_font.identifier());
                        let chunk = t3_font.serialize_into(&mut self, ref_);
                        self.chunk_container.fonts.push(chunk);
                    }
                }
                FontContainer::CIDFont(cid_font) => {
                    let ref_ = self.add_resource(cid_font.identifier());
                    let chunk = cid_font.serialize_into(&mut self, ref_);
                    self.chunk_container.fonts.push(chunk);
                }
            }
        }

        let pages = std::mem::take(&mut self.pages);
        for (ref_, page) in &pages {
            let chunk = page.serialize_into(&mut self, *ref_);
            self.chunk_container.pages.push(chunk);
        }

        if let Some(page_tree_ref) = self.page_tree_ref {
            let mut page_tree_chunk = Chunk::new();
            page_tree_chunk
                .pages(page_tree_ref)
                .count(pages.len() as i32)
                .kids(pages.iter().map(|(r, _)| *r));
            self.chunk_container.page_tree = Some((page_tree_ref, page_tree_chunk));
        }

        // Just a sanity check.
        assert!(self.font_map.is_empty());
        assert!(self.pages.is_empty());

        self.chunk_container.finish(self.serialize_settings)
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
