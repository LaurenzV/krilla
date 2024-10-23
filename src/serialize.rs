use crate::chunk_container::ChunkContainer;
use crate::color::{ColorSpace, ICCBasedColorSpace, ICCProfile, DEVICE_CMYK};
use crate::content::PdfFont;
use crate::error::{KrillaError, KrillaResult};
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
use crate::resource::{grey_icc, rgb_icc, Resource};
use crate::tagging::{AnnotationIdentifier, IdentifierType, PageTagIdentifier, TagTree};
use crate::util::{NameExt, SipHashable};
use crate::validation::{ValidationError, Validator};
use crate::version::PdfVersion;
#[cfg(feature = "fontdb")]
use fontdb::{Database, ID};
use pdf_writer::types::{OutputIntentSubtype, StructRole};
use pdf_writer::writers::{NumberTree, OutputIntent, RoleMap};
use pdf_writer::{Array, Chunk, Dict, Finish, Name, Pdf, Ref, Str, TextStr};
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
    /// Note that this value might be overridden depending on which validator
    /// you use. For example, when exporting to PDF/A, this value will be set to
    /// true, regardless of what value will be passed.
    pub no_device_cs: bool,
    /// Whether the PDF should be ASCII-compatible, i.e. only consist of
    /// characters in the ASCII range.
    ///
    /// Note that this only on a best-effort basis. For example, XMP metadata always
    /// contains a binary marker. In addition to that, some validators,
    /// like PDF/A, require that the file header be a binary marker, meaning
    /// that the header itself will not be ASCII-compatible.
    ///
    /// Binary streams will always be hex encoded and thus are ascii compatible, though.
    pub ascii_compatible: bool,
    /// Whether the PDF should contain XMP metadata.
    ///
    /// Note that this value might be overridden depending on which validator
    /// you use. For example, when exporting to PDF/A, this value will be set to
    /// true, regardless of what value will be passed.
    pub xmp_metadata: bool,
    /// Whether all fonts should be embedded as Type3 fonts.
    pub force_type3_fonts: bool,
    /// The ICC profile that should be used for CMYK colors
    /// when `no_device_cs` is enabled.
    ///
    /// This is usually not required, but it is for example required when exporting
    /// to PDF/A and using a CMYK color, since they have to be device-independent.
    pub cmyk_profile: Option<ICCProfile<4>>,
    /// A validator that allows for exporting to a specific substandard of PDF.
    ///
    /// In case validation fails, export will fail, and a list of validation errors that
    /// occurred will be returned instead of the PDF.
    ///
    /// **Important**: Make sure to carefully read the documentation of the [`validation`] module
    /// before using this feature! Just setting a validator might not be enough to ensure that
    /// your output conforms to the given standard, as some requirements are semantic in nature
    /// and cannot possibly be verified by krilla!
    ///
    /// However, as long as you carefully read and follow the documentation,
    /// you can be certain that the resulting document will conform to the standard (unless there
    /// is a bug).
    ///
    /// [`validation`]: crate::validation
    pub validator: Validator,
    /// Whether to enable the creation of tagged documents. See the module documentation
    /// of [`tagging`] for more information about tagged PDF documents.
    ///
    /// Note that enabling this does not automatically make your documents tagged, as tagging implies
    /// enriching the document with semantic information, which krilla cannot do
    /// for you, since it's content-agnostic. All this setting does is to allow you
    /// to dynamically disable tagging if you wish to do so. This allows you to write
    /// your code primarily with tagging in mind, but still allows you to
    /// disable it dynamically, without having to make any changes to your code.
    ///
    /// Note that this value might be overridden depending on which validator
    /// you use. For example, when exporting with PDF-UA, this value will always
    /// be set to `true`.
    ///
    /// [`tagging`]: crate::tagging
    pub enable_tagging: bool,
    /// The PDF version that should be used for export.
    pub pdf_version: PdfVersion,
}

const STR_BYTE_LEN: usize = 32767;

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
            validator: Validator::Dummy,
            enable_tagging: true,
            pdf_version: PdfVersion::Pdf17,
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

    pub(crate) fn settings_6() -> Self {
        Self {
            no_device_cs: true,
            cmyk_profile: Some(ICCProfile::new(Arc::new(
                std::fs::read(crate::tests::ASSETS_PATH.join("icc/eciCMYK_v2.icc")).unwrap(),
            ))),
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_7() -> Self {
        Self {
            validator: Validator::A2_B,
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_8() -> Self {
        Self {
            validator: Validator::A2_B,
            cmyk_profile: Some(ICCProfile::new(Arc::new(
                std::fs::read(crate::tests::ASSETS_PATH.join("icc/eciCMYK_v2.icc")).unwrap(),
            ))),
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_9() -> Self {
        Self {
            validator: Validator::A2_U,
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_10() -> Self {
        Self {
            validator: Validator::A3_B,
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_11() -> Self {
        Self {
            validator: Validator::A3_U,
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_12() -> Self {
        Self {
            enable_tagging: false,
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_13() -> Self {
        Self {
            // Just to check that krilla enables tagging
            // for this validator even if explicitly disabled.
            enable_tagging: false,
            validator: Validator::A2_A,
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_14() -> Self {
        Self {
            validator: Validator::A3_A,
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_15() -> Self {
        Self {
            pdf_version: PdfVersion::Pdf14,
            xmp_metadata: true,
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_16() -> Self {
        Self {
            pdf_version: PdfVersion::Pdf14,
            ..Self::settings_1()
        }
    }

    pub(crate) fn settings_17() -> Self {
        Self {
            pdf_version: PdfVersion::Pdf14,
            no_device_cs: true,
            ..Self::settings_1()
        }
    }

    // TODO: Add test for version mismatch
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
            validator: Validator::Dummy,
            enable_tagging: true,
            pdf_version: PdfVersion::Pdf17,
        }
    }
}

pub(crate) struct PageInfo {
    pub ref_: Ref,
    pub surface_size: Size,
    // The refs of the annotations that are used by that page.
    //
    // Note that this will be empty be default, and only once we have serialized the pages
    // will these values be set.
    pub annotations: Vec<Ref>,
}

enum StructParentElement {
    Page(usize, i32),
    Annotation(usize, usize),
}

pub(crate) struct SerializerContext {
    font_cache: HashMap<Arc<FontInfo>, Font>,
    font_map: HashMap<Font, Rc<RefCell<FontContainer>>>,
    page_tree_ref: Option<Ref>,
    page_infos: Vec<PageInfo>,
    pages: Vec<(Ref, InternalPage)>,
    struct_parents: Vec<StructParentElement>,
    outline: Option<Outline>,
    cached_mappings: HashMap<u128, Ref>,
    tag_tree: Option<TagTree>,
    cur_ref: Ref,
    chunk_container: ChunkContainer,
    validation_errors: Vec<ValidationError>,
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
    pub fn new(mut serialize_settings: SerializeSettings) -> Self {
        // If the validator requires/prefers no device color spaces
        // set it to true, even if the user didn't set it.
        serialize_settings.no_device_cs |= serialize_settings.validator.requires_no_device_cs();
        serialize_settings.enable_tagging |= serialize_settings.validator.requires_tagging();
        serialize_settings.xmp_metadata |= serialize_settings.validator.xmp_metadata();

        Self {
            cached_mappings: HashMap::new(),
            font_cache: HashMap::new(),
            cur_ref: Ref::new(1),
            chunk_container: ChunkContainer::new(),
            page_tree_ref: None,
            struct_parents: vec![],
            outline: None,
            page_infos: vec![],
            pages: vec![],
            tag_tree: None,
            font_map: HashMap::new(),
            validation_errors: vec![],
            serialize_settings,
        }
    }

    pub fn get_page_struct_parent(&mut self, page_index: usize, num_mcids: i32) -> Option<i32> {
        if self.serialize_settings.enable_tagging {
            if num_mcids == 0 {
                return None;
            }

            let id = self.struct_parents.len();
            self.struct_parents
                .push(StructParentElement::Page(page_index, num_mcids));
            Some(i32::try_from(id).unwrap())
        } else {
            None
        }
    }

    pub fn get_annotation_parent(
        &mut self,
        page_index: usize,
        annotation_index: usize,
    ) -> Option<i32> {
        if self.serialize_settings.enable_tagging {
            let id = self.struct_parents.len();
            self.struct_parents.push(StructParentElement::Annotation(
                page_index,
                annotation_index,
            ));
            Some(i32::try_from(id).unwrap())
        } else {
            None
        }
    }

    pub fn page_infos(&self) -> &[PageInfo] {
        &self.page_infos
    }

    pub fn page_infos_mut(&mut self) -> &mut [PageInfo] {
        &mut self.page_infos
    }

    pub fn set_outline(&mut self, outline: Outline) {
        self.outline = Some(outline);
    }

    pub fn set_metadata(&mut self, metadata: Metadata) {
        self.chunk_container.metadata = Some(metadata);
    }

    pub fn set_tag_tree(&mut self, root: TagTree) {
        // Only set the tag tree if the user actually enabled tagging.
        if self.serialize_settings.enable_tagging {
            self.tag_tree = Some(root)
        }
    }

    pub(crate) fn register_validation_error(&mut self, error: ValidationError) {
        if self.serialize_settings.validator.prohibits(&error) {
            self.validation_errors.push(error);
        }
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
            // Will be populated when the page is serialized.
            annotations: vec![],
        });
        self.pages.push((ref_, page));
    }

    pub fn new_ref(&mut self) -> Ref {
        self.cur_ref.bump()
    }

    pub fn add_cs(&mut self, cs: ColorSpace) -> CSWrapper {
        match cs {
            ColorSpace::Rgb => CSWrapper::Ref(self.add_resource(Resource::Rgb)),
            ColorSpace::Gray => CSWrapper::Ref(self.add_resource(Resource::Gray)),
            ColorSpace::Cmyk(cs) => CSWrapper::Ref(self.add_resource(Resource::Cmyk(cs))),
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
        let chunk = page_label.serialize(self, ref_);
        self.chunk_container.page_labels.push(chunk);
        ref_
    }

    pub fn new_text_str<'a>(&mut self, text: &'a str) -> TextStr<'a> {
        if text.as_bytes().len() > STR_BYTE_LEN {
            self.register_validation_error(ValidationError::TooLongString);
        }

        TextStr(text)
    }

    pub fn new_str<'a>(&mut self, str: &'a [u8]) -> Str<'a> {
        if str.len() > STR_BYTE_LEN {
            self.register_validation_error(ValidationError::TooLongString);
        }

        Str(str)
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
            Resource::Rgb => self.add_object(ICCBasedColorSpace(rgb_icc(&self.serialize_settings))),
            Resource::Gray => self.add_object(ICCBasedColorSpace(grey_icc(&self.serialize_settings))),
            // Unwrap is safe, because we only emit `IccCmyk`
            // if a profile has been set in the first place.
            Resource::Cmyk(cs) => self.add_object(cs),
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

    fn get_output_intents(&mut self, subtype: OutputIntentSubtype, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        let oi_ref = self.new_ref();
        let mut oi = chunk.indirect(oi_ref).start::<OutputIntent>();
        oi.dest_output_profile(self.add_object(rgb_icc(&self.serialize_settings)))
            .subtype(subtype)
            .output_condition_identifier(TextStr("Custom"))
            .output_condition(TextStr("sRGB"))
            .registry_name(TextStr(""))
            .info(TextStr("sRGB v4.2"));
        oi.finish();

        let mut array = chunk.indirect(root_ref).array();
        array.item(oi_ref);
        array.finish();

        chunk
    }

    pub fn finish(mut self) -> KrillaResult<Pdf> {
        if !self
            .serialize_settings
            .validator
            .compatible_with(self.serialize_settings.pdf_version)
        {
            return Err(KrillaError::UserError(format!(
                "{} is not compatible with export mode {}",
                self.serialize_settings.pdf_version.as_str(),
                self.serialize_settings.validator.as_str()
            )));
        }

        // We need to be careful here that we serialize the objects in the right order,
        // as in some cases we use `std::mem::take` to remove an object, which means that
        // no object that is serialized afterwards must depend on it.

        // Write output intent, if required by the validator.
        let validator = self.serialize_settings.validator;
        self.chunk_container.destination_profiles = validator.output_intent().map(|subtype| {
            let root_ref = self.new_ref();
            let chunk = self.get_output_intents(subtype, root_ref);
            (root_ref, chunk)
        });

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
            let chunk = page.serialize(&mut self, *ref_)?;
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

        // It is important that we serialize the tags AFTER we have serialized the pages,
        // because page serialization will update the annotation refs of the page infos,
        // and when serializing the parent tree map we need to know the refs of the annotations
        let tag_tree = std::mem::take(&mut self.tag_tree);
        let struct_parents = std::mem::take(&mut self.struct_parents);
        if let Some(root) = &tag_tree {
            let mut parent_tree_map = HashMap::new();
            let struct_tree_root_ref = self.new_ref();
            let (document_ref, struct_elems) =
                root.serialize(&mut self, &mut parent_tree_map, struct_tree_root_ref);
            self.chunk_container.struct_elements = struct_elems;

            let mut chunk = Chunk::new();
            let mut tree = chunk.indirect(struct_tree_root_ref).start::<Dict>();
            tree.pair(Name(b"Type"), Name(b"StructTreeRoot"));
            let mut role_map = tree.insert(Name(b"RoleMap")).start::<RoleMap>();
            role_map.insert(Name(b"Image"), StructRole::Figure);
            role_map.insert(Name(b"Datetime"), StructRole::Span);
            role_map.insert(Name(b"Terms"), StructRole::Part);
            role_map.insert(Name(b"Title"), StructRole::H1);
            role_map.finish();
            tree.insert(Name(b"K")).array().item(document_ref);

            let mut sub_chunks = vec![];
            let mut parent_tree = tree.insert(Name(b"ParentTree")).start::<NumberTree<Ref>>();
            let mut tree_nums = parent_tree.nums();

            for (index, struct_parent) in struct_parents.iter().enumerate() {
                match *struct_parent {
                    StructParentElement::Page(index, num_mcids) => {
                        let mut list_chunk = Chunk::new();
                        let list_ref = self.new_ref();

                        let mut refs = list_chunk.indirect(list_ref).array();

                        for mcid in 0..num_mcids {
                            let rci = PageTagIdentifier::new(index, mcid);
                            // TODO: Graceful handling
                            refs.item(parent_tree_map.get(&rci.into()).unwrap());
                        }

                        refs.finish();

                        sub_chunks.push(list_chunk);
                        tree_nums.insert(index as i32, list_ref);
                    }
                    StructParentElement::Annotation(page_index, annot_index) => {
                        let it = IdentifierType::AnnotationIdentifier(AnnotationIdentifier::new(
                            page_index,
                            annot_index,
                        ));
                        let ref_ = parent_tree_map.get(&it).unwrap();
                        tree_nums.insert(index as i32, *ref_);
                    }
                }
            }

            tree_nums.finish();
            parent_tree.finish();

            tree.pair(Name(b"ParentTreeNextKey"), struct_parents.len() as i32);

            tree.finish();

            for sub_chunk in sub_chunks {
                chunk.extend(&sub_chunk);
            }

            self.chunk_container.struct_tree_root = Some((struct_tree_root_ref, chunk));
        }

        if self.cur_ref > Ref::new(8388607) {
            self.register_validation_error(ValidationError::TooManyIndirectObjects)
        }

        let chunk_container = std::mem::take(&mut self.chunk_container);
        let serialized = chunk_container.finish(&mut self);

        if !self.validation_errors.is_empty() {
            return Err(KrillaError::ValidationError(self.validation_errors));
        }

        // Just a sanity check.
        assert!(self.font_map.is_empty());
        assert!(self.pages.is_empty());
        // TODO: add check that chunk container is empty

        Ok(serialized)
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
