use crate::chunk_container::ChunkContainer;
use crate::color::{ColorSpace, ICCProfile, DEVICE_CMYK};
use crate::content::PdfFont;
use crate::error::KrillaResult;
use crate::font::{Font, FontIdentifier, FontInfo};
#[cfg(feature = "raster-images")]
use crate::image::Image;
use crate::metadata::Metadata;
use crate::object::cid_font::CIDFont;
use crate::object::color::{DEVICE_GRAY, DEVICE_RGB};
use crate::object::outline::Outline;
use crate::object::page::{InternalPage, PageLabelContainer};
use crate::object::type3_font::{CoveredGlyph, Type3FontMapper};
use crate::object::Object;
use crate::page::PageLabel;
use crate::resource::{Resource, GREY_ICC, SRGB_ICC};
use crate::util::{NameExt, SipHashable};
#[cfg(feature = "fontdb")]
use fontdb::{Database, ID};
use pdf_writer::{Array, Chunk, Dict, Name, Pdf, Ref};
use skrifa::raw::TableProvider;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::Arc;
use tiny_skia_path::Size;

/// Settings that should be applied when converting a SVG.
#[derive(Copy, Clone, Debug)]
pub struct SvgSettings {
    /// How much filters, which will be converted to bitmaps, should be scaled. Higher values
    /// mean better quality, but also bigger file sizes.
    pub filter_scale: f32,
    /// Whether text should be embedded as properly selectable text. Otherwise,
    /// it will be drawn as outlined paths instead.
    pub embed_text: bool,
}

impl Default for SvgSettings {
    fn default() -> Self {
        Self {
            filter_scale: 1.5,
            embed_text: true,
        }
    }
}

/// Settings that should be applied when creating a PDF document.
#[derive(Clone, Debug)]
pub struct SerializeSettings {
    /// Whether content streams should be compressed.
    pub compress_content_streams: bool,
    /// Whether device-independent colors should be used instead of
    /// device-dependent ones.
    ///
    /// CMYK colors are currently not affected by this setting.
    pub no_device_cs: bool,
    /// Whether the PDF should be ASCII-compatible, i.e. only consist of
    /// characters in the ASCII range.
    pub ascii_compatible: bool,
    /// Whether the PDF should contain XMP metadata.
    pub xmp_metadata: bool,
    /// Whether all fonts should be embedded as Type3 fonts.
    pub force_type3_fonts: bool,
    /// The ICC profile that should be used for CMYK colors
    /// when `no_device_cs` is enabled.
    pub cmyk_profile: Option<ICCProfile>,
}

#[cfg(test)]
impl SerializeSettings {
    pub(crate) fn settings_1() -> Self {
        Self {
            ascii_compatible: true,
            compress_content_streams: false,
            no_device_cs: false,
            xmp_metadata: false,
            force_type3_fonts: false,
            cmyk_profile: None,
        }
    }

    pub(crate) fn settings_2() -> Self {
        Self {
            no_device_cs: true,
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_4() -> Self {
        Self {
            force_type3_fonts: true,
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_5() -> Self {
        Self {
            xmp_metadata: true,
            ..Self::settings_1()
        }
    }
}

impl Default for SerializeSettings {
    fn default() -> Self {
        Self {
            ascii_compatible: false,
            compress_content_streams: true,
            no_device_cs: false,
            xmp_metadata: true,
            force_type3_fonts: false,
            cmyk_profile: None,
        }
    }
}

pub(crate) struct PageInfo {
    pub ref_: Ref,
    pub surface_size: Size,
}

pub(crate) struct SerializerContext {
    font_cache: HashMap<Arc<FontInfo>, Font>,
    font_map: HashMap<Font, Rc<RefCell<FontContainer>>>,
    page_tree_ref: Option<Ref>,
    page_infos: Vec<PageInfo>,
    pages: Vec<(Ref, InternalPage)>,
    outline: Option<Outline>,
    cached_mappings: HashMap<u128, Ref>,
    cur_ref: Ref,
    chunk_container: ChunkContainer,
    pub(crate) serialize_settings: SerializeSettings,
}

#[derive(Clone, Copy)]
pub(crate) enum PDFGlyph {
    Type3(u8),
    Cid(u16),
}

impl PDFGlyph {
    pub fn encode_into(&self, slice: &mut Vec<u8>) {
        match self {
            PDFGlyph::Type3(cg) => slice.push(*cg),
            PDFGlyph::Cid(cid) => {
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

    pub fn set_metadata(&mut self, metadata: Metadata) {
        self.chunk_container.metadata = Some(metadata);
    }

    pub fn page_tree_ref(&mut self) -> Ref {
        *self
            .page_tree_ref
            .get_or_insert_with(|| self.cur_ref.bump())
    }

    pub fn add_page(&mut self, page: InternalPage) {
        let ref_ = self.new_ref();
        self.page_infos.push(PageInfo {
            ref_,
            surface_size: page.page_settings.surface_size(),
        });
        self.pages.push((ref_, page));
    }

    pub fn new_ref(&mut self) -> Ref {
        self.cur_ref.bump()
    }

    pub fn add_cs(&mut self, cs: ColorSpace) -> CSWrapper {
        match cs {
            ColorSpace::Srgb => CSWrapper::Ref(self.add_resource(Resource::Srgb)),
            ColorSpace::SGray => CSWrapper::Ref(self.add_resource(Resource::SGray)),
            ColorSpace::IccCmyk(cs) => CSWrapper::Ref(self.add_resource(Resource::IccCmyk(cs))),
            ColorSpace::DeviceGray => CSWrapper::Name(DEVICE_GRAY.to_pdf_name()),
            ColorSpace::DeviceRgb => CSWrapper::Name(DEVICE_RGB.to_pdf_name()),
            ColorSpace::DeviceCmyk => CSWrapper::Name(DEVICE_CMYK.to_pdf_name()),
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
            let mut chunk_container_fn = object.chunk_container();
            let chunk = object.serialize(self, root_ref);
            self.cached_mappings.insert(hash, root_ref);
            chunk_container_fn(&mut self.chunk_container).push(chunk);
            root_ref
        }
    }

    #[cfg(feature = "raster-images")]
    pub fn add_image(&mut self, image: Image) -> Ref {
        let hash = image.sip_hash();
        if let Some(_ref) = self.cached_mappings.get(&hash) {
            *_ref
        } else {
            let root_ref = self.new_ref();
            let chunk = image.serialize(self, root_ref);
            self.cached_mappings.insert(hash, root_ref);
            self.chunk_container.images.push(chunk);
            root_ref
        }
    }

    pub fn add_page_label(&mut self, page_label: PageLabel) -> Ref {
        let ref_ = self.new_ref();
        let chunk = page_label.serialize(ref_);
        self.chunk_container.page_labels.push(chunk);
        ref_
    }

    pub fn create_or_get_font_container(&mut self, font: Font) -> Rc<RefCell<FontContainer>> {
        self.font_map
            .entry(font.clone())
            .or_insert_with(|| {
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
                    || !font.location_ref().coords().is_empty()
                    || font_ref.svg().is_ok()
                    || font_ref.colr().is_ok()
                    || font_ref.sbix().is_ok()
                    || font_ref.cbdt().is_ok()
                    || font_ref.ebdt().is_ok()
                    || font_ref.cff2().is_ok();

                if use_type3 {
                    Rc::new(RefCell::new(FontContainer::Type3(Type3FontMapper::new(
                        font.clone(),
                    ))))
                } else {
                    Rc::new(RefCell::new(FontContainer::CIDFont(CIDFont::new(
                        font.clone(),
                    ))))
                }
            })
            .clone()
    }

    pub(crate) fn add_resource(&mut self, resource: impl Into<Resource>) -> Ref {
        match resource.into() {
            Resource::XObject(x) => self.add_object(x),
            #[cfg(feature = "raster-images")]
            Resource::Image(i) => self.add_image(i),
            Resource::ShadingPattern(sp) => self.add_object(sp),
            Resource::TilingPattern(tp) => self.add_object(tp),
            Resource::ExtGState(e) => self.add_object(e),
            Resource::Srgb => self.add_object(SRGB_ICC.clone()),
            Resource::SGray => self.add_object(GREY_ICC.clone()),
            Resource::IccCmyk(profile) => self.add_object(profile),
            Resource::ShadingFunction(s) => self.add_object(s),
            Resource::FontIdentifier(f) => {
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

    #[cfg(feature = "fontdb")]
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
                if let Some(font_info) = FontInfo::new(font_data.as_ref().as_ref(), index, vec![]) {
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

    pub fn finish(mut self) -> KrillaResult<Pdf> {
        // We need to be careful here that we serialize the objects in the right order,
        // as in some cases we use `std::mem::take` to remove an object, which means that
        // no object that is serialized afterwards must depend on it.

        if let Some(container) = PageLabelContainer::new(
            &self
                .pages
                .iter()
                .map(|(_, p)| p.page_settings.page_label().clone())
                .collect::<Vec<_>>(),
        ) {
            let page_label_tree_ref = self.new_ref();
            let chunk = container.serialize(&mut self, page_label_tree_ref);
            self.chunk_container.page_label_tree = Some((page_label_tree_ref, chunk));
        }

        let outline = std::mem::take(&mut self.outline);
        if let Some(outline) = &outline {
            let outline_ref = self.new_ref();
            let chunk = outline.serialize(&mut self, outline_ref)?;
            self.chunk_container.outline = Some((outline_ref, chunk));
        }

        let fonts = std::mem::take(&mut self.font_map);
        for font_container in fonts.values() {
            match &*font_container.borrow() {
                FontContainer::Type3(font_mapper) => {
                    for t3_font in font_mapper.fonts() {
                        let ref_ = self.add_resource(t3_font.identifier());
                        let chunk = t3_font.serialize(&mut self, ref_);
                        self.chunk_container.fonts.push(chunk);
                    }
                }
                FontContainer::CIDFont(cid_font) => {
                    let ref_ = self.add_resource(cid_font.identifier());
                    let chunk = cid_font.serialize(&mut self, ref_)?;
                    self.chunk_container.fonts.push(chunk);
                }
            }
        }

        let pages = std::mem::take(&mut self.pages);
        for (ref_, page) in &pages {
            let chunk = page.serialize(&mut self, *ref_);
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

        Ok(self.chunk_container.finish(self.serialize_settings))
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
pub(crate) enum FontContainer {
    Type3(Type3FontMapper),
    CIDFont(CIDFont),
}

impl FontContainer {
    pub fn font_identifier(&self, glyph: CoveredGlyph) -> Option<FontIdentifier> {
        match self {
            FontContainer::Type3(t3) => t3.id_from_glyph(&glyph.to_owned()),
            FontContainer::CIDFont(cid) => cid.get_cid(glyph.glyph_id).map(|_| cid.identifier()),
        }
    }

    pub fn get_from_identifier_mut(
        &mut self,
        font_identifier: FontIdentifier,
    ) -> Option<&mut dyn PdfFont> {
        match self {
            FontContainer::Type3(t3) => {
                if let Some(t3_font) = t3.font_mut_from_id(font_identifier) {
                    Some(t3_font)
                } else {
                    None
                }
            }
            FontContainer::CIDFont(cid) => {
                if cid.identifier() == font_identifier {
                    Some(cid)
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
                    Some(t3_font)
                } else {
                    None
                }
            }
            FontContainer::CIDFont(cid) => {
                if cid.identifier() == font_identifier {
                    Some(cid)
                } else {
                    None
                }
            }
        }
    }

    pub fn add_glyph(&mut self, glyph: CoveredGlyph) -> (FontIdentifier, PDFGlyph) {
        match self {
            FontContainer::Type3(t3) => {
                let (identifier, gid) = t3.add_glyph(glyph.to_owned());
                (identifier, PDFGlyph::Type3(gid))
            }
            FontContainer::CIDFont(cid_font) => {
                let cid = cid_font.add_glyph(glyph.glyph_id);
                (cid_font.identifier(), PDFGlyph::Cid(cid))
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum StreamFilter {
    FlateDecode,
    AsciiHexDecode,
}

impl StreamFilter {
    pub(crate) fn to_name(self) -> Name<'static> {
        match self {
            Self::AsciiHexDecode => Name(b"ASCIIHexDecode"),
            Self::FlateDecode => Name(b"FlateDecode"),
        }
    }
}

impl StreamFilter {
    pub fn apply(&self, content: &[u8]) -> Vec<u8> {
        match self {
            StreamFilter::FlateDecode => deflate_encode(content),
            StreamFilter::AsciiHexDecode => hex_encode(content),
        }
    }
}

// Allows us to keep track of the filters that a stream has and
// apply them in an orderly fashion.
#[derive(Debug, Clone)]
pub enum StreamFilters {
    None,
    Single(StreamFilter),
    Multiple(Vec<StreamFilter>),
}

impl StreamFilters {
    pub fn add(&mut self, stream_filter: StreamFilter) {
        match self {
            StreamFilters::None => *self = StreamFilters::Single(stream_filter),
            StreamFilters::Single(cur) => {
                *self = StreamFilters::Multiple(vec![*cur, stream_filter])
            }
            StreamFilters::Multiple(cur) => cur.push(stream_filter),
        }
    }
}

pub struct FilterStream<'a> {
    content: Cow<'a, [u8]>,
    filters: StreamFilters,
}

impl<'a> FilterStream<'a> {
    fn empty(content: &'a [u8]) -> Self {
        Self {
            content: Cow::Borrowed(content),
            filters: StreamFilters::None,
        }
    }

    pub fn new_from_content_stream(
        content: &'a [u8],
        serialize_settings: &SerializeSettings,
    ) -> Self {
        let mut filter_stream = Self::empty(content);

        if serialize_settings.compress_content_streams {
            filter_stream.add_filter(StreamFilter::FlateDecode);

            if serialize_settings.ascii_compatible {
                filter_stream.add_filter(StreamFilter::AsciiHexDecode);
            }
        }

        filter_stream
    }

    pub fn new_from_binary_data(content: &'a [u8], serialize_settings: &SerializeSettings) -> Self {
        let mut filter_stream = Self::empty(content);
        filter_stream.add_filter(StreamFilter::FlateDecode);

        if serialize_settings.ascii_compatible {
            filter_stream.add_filter(StreamFilter::AsciiHexDecode);
        }

        filter_stream
    }

    pub fn add_filter(&mut self, filter: StreamFilter) {
        self.content = Cow::Owned(filter.apply(&self.content));
        self.filters.add(filter);
    }

    pub fn encoded_data(&self) -> &[u8] {
        &self.content
    }

    pub fn write_filters<'b, T>(&self, mut dict: T)
    where
        T: DerefMut<Target = Dict<'b>>,
    {
        match &self.filters {
            StreamFilters::None => {}
            StreamFilters::Single(filter) => {
                dict.deref_mut().pair(Name(b"Filter"), filter.to_name());
            }
            StreamFilters::Multiple(filters) => {
                dict.deref_mut()
                    .insert(Name(b"Filter"))
                    .start::<Array>()
                    .items(filters.iter().map(|f| f.to_name()).rev());
            }
        }
    }
}

fn deflate_encode(data: &[u8]) -> Vec<u8> {
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
