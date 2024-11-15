use crate::chunk_container::ChunkContainer;
use crate::color::{ColorSpace, ICCBasedColorSpace, ICCProfile, LinearRgbColorSpace, DEVICE_CMYK};
use crate::destination::{NamedDestination, XyzDestination};
use crate::error::{KrillaError, KrillaResult};
use crate::font::{Font, FontInfo};
#[cfg(feature = "raster-images")]
use crate::image::Image;
use crate::metadata::Metadata;
use crate::object::color::{CSWrapper, DEVICE_GRAY, DEVICE_RGB};
use crate::object::font::cid_font::CIDFont;
use crate::object::font::type3_font::Type3FontMapper;
use crate::object::font::FontContainer;
use crate::object::outline::Outline;
use crate::object::page::{InternalPage, PageLabelContainer};
use crate::object::Object;
use crate::page::PageLabel;
use crate::resource::Resource;
use crate::tagging::{AnnotationIdentifier, IdentifierType, PageTagIdentifier, TagTree};
use crate::util::{NameExt, SipHashable};
use crate::validation::{ValidationError, Validator};
use crate::version::PdfVersion;
#[cfg(feature = "fontdb")]
use fontdb::{Database, ID};
use pdf_writer::types::{OutputIntentSubtype, StructRole};
use pdf_writer::writers::{NameTree, NumberTree, OutputIntent, RoleMap};
use pdf_writer::{Chunk, Dict, Finish, Limits, Name, Pdf, Ref, Str, TextStr};
use skrifa::raw::TableProvider;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::Arc;
use tiny_skia_path::Size;

/// Settings that should be applied when converting a SVG.
#[derive(Copy, Clone, Debug)]
pub struct SvgSettings {
    /// Whether text should be embedded as properly selectable text. Otherwise,
    /// it will be drawn as outlined paths instead.
    pub embed_text: bool,
    /// How much filters, which will be converted to bitmaps, should be scaled. Higher values
    /// mean better quality, but also bigger file sizes. If set to None, an appropriate value
    /// will be determined automatically (this is the recommended choice).
    // We mainly have this because SVG filter tests takes ages to run in CI otherwise.
    pub filter_scale: Option<f32>,
}

impl Default for SvgSettings {
    fn default() -> Self {
        Self {
            embed_text: true,
            filter_scale: None,
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

const STR_LEN: usize = 32767;
const NAME_LEN: usize = 127;

// These only apply to PDF 1.4 and PDF/A-1.
const MAX_FLOAT: f32 = 32767.0;
const DICT_LEN: usize = 4095;
const ARRAY_LEN: usize = 8191;

impl Default for SerializeSettings {
    fn default() -> Self {
        Self {
            ascii_compatible: false,
            compress_content_streams: true,
            no_device_cs: false,
            xmp_metadata: true,
            cmyk_profile: None,
            validator: Validator::None,
            enable_tagging: true,
            pdf_version: PdfVersion::Pdf17,
        }
    }
}

pub(crate) struct PageInfo {
    /// The reference of the page in the chunk.
    pub ref_: Ref,
    /// The page size, necessary so that we can convert from PDF coordinates to
    /// krilla coordinates.
    pub surface_size: Size,
    /// The refs of the annotations that are used by that page.
    ///
    /// Note that this will be empty be default when adding a new `PageInfo` to
    /// `page_infos` in `SerializerContext`, and only once we actually serialize
    /// the page will the annotations be populated.
    pub annotations: Vec<Ref>,
}

enum StructParentElement {
    /// The index of the page and the number of marked content IDs present on that page.
    Page(usize, i32),
    /// The index of the page where the annotation is present, as well as the index of the
    /// annotation within that one page.
    Annotation(usize, usize),
}

/// The serializer context is more or less the core piece of krilla. It is passed around
/// throughout pretty much the whole conversion process, and contains all mutable state
/// that is needed when writing a PDF file. This includes for example:
/// - Storing all chunks that are produced.
/// - The mappings from OTF fonts to CID/Type 3 fonts.
/// - Annotations used in the document.
///   etc.
pub(crate) struct SerializerContext {
    font_cache: HashMap<Arc<FontInfo>, Font>,
    pub(crate) named_destinations: HashMap<NamedDestination, Ref>,
    pub(crate) used_named_destinations: BTreeSet<NamedDestination>,
    font_map: HashMap<Font, Rc<RefCell<FontContainer>>>,
    xyz_dests: Vec<(Ref, XyzDestination)>,
    page_tree_ref: Option<Ref>,
    /// Keep track of some necessary information for each page we have written so far.
    /// This is necessary because in the end, we might for example need to get the Ref
    /// of specific pages. Another use case is that we need access to the height of a page
    /// to convert from krilla coordinates to PDF coordinates.
    page_infos: Vec<PageInfo>,
    /// All the pages we have written so far. Unlike other objects, pages cannot be written
    /// as soon as they are finished, but we need defer writing to until we call `finish()`.
    /// The (one?) reason for this is that as part of serializing pages, we also serialize its
    /// annotations. However, annotations can also reference future pages which have not been
    /// written yet, and thus do not have a Ref. Because of this, this can only be done in the
    /// very end.
    pages: Vec<(Ref, InternalPage)>,
    /// This array keeps track of the values we need to write for the struct parent tree. Each
    /// element can either be from an annotation (identified by page index and annotation index) or
    /// from a page (identified by page index and the number of mcids on that page).
    struct_parents: Vec<StructParentElement>,
    /// The outline of the document, optionally set by the user.
    outline: Option<Outline>,
    /// Keep track of object hashes and their corresponding reference. This is used for
    /// caching, so that for example same images will not be embedded twice in the document.
    cached_mappings: HashMap<u128, Ref>,
    /// The tag tree of the document, optionally set by the user.
    tag_tree: Option<TagTree>,
    /// The current ref in use. All serializers should use the `new_ref` method (which indirectly
    /// is based on this field) to generate a new Ref, instead of creating one manually with
    /// `Ref::new`.
    cur_ref: Ref,
    /// Collect all chunks that are generated as part of the PDF writing process.
    chunk_container: ChunkContainer,
    /// All validation errors that are collected as part of the export process.
    validation_errors: Vec<ValidationError>,
    /// Settings used for serialization.
    serialize_settings: Arc<SerializeSettings>,
    /// The limits created as part of the serialization process. In principle, we could
    /// just keep track of this in `ChunkContainer`, where all used chunks are stored.
    /// The only reason why `SerializerContext` needs to know about them is that we also
    /// need to merge limits from postscript functions, which are not directly accessible
    /// from the chunk they are written to.
    limits: Limits,
}

impl SerializerContext {
    pub fn new(mut serialize_settings: SerializeSettings) -> Self {
        // Override flags as required by the validator
        serialize_settings.no_device_cs |= serialize_settings.validator.requires_no_device_cs();
        serialize_settings.enable_tagging |= serialize_settings.validator.requires_tagging();
        serialize_settings.xmp_metadata |= serialize_settings.validator.xmp_metadata();

        if !serialize_settings
            .validator
            .compatible_with_version(serialize_settings.pdf_version)
        {
            serialize_settings.pdf_version = serialize_settings.validator.recommended_version();
        }

        Self {
            cached_mappings: HashMap::new(),
            font_cache: HashMap::new(),
            named_destinations: HashMap::new(),
            used_named_destinations: BTreeSet::new(),
            xyz_dests: Vec::new(),
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
            serialize_settings: Arc::new(serialize_settings),
            limits: Limits::new(),
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

    pub fn add_named_destination(&mut self, nd: NamedDestination, location: XyzDestination) {
        let dest_ref = self.add_xyz_dest(location);
        self.named_destinations.insert(nd, dest_ref);
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

    pub(crate) fn serialize_settings(&self) -> Arc<SerializeSettings> {
        self.serialize_settings.clone()
    }

    pub(crate) fn register_limits(&mut self, limits: &Limits) {
        self.limits.merge(limits);
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
            ColorSpace::Srgb => CSWrapper::Ref(self.add_resource(Resource::Srgb)),
            ColorSpace::LinearRgb => CSWrapper::Ref(self.add_resource(Resource::LinearRgb)),
            ColorSpace::Luma => CSWrapper::Ref(self.add_resource(Resource::Luma)),
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
        // TODO: Deduplicate
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

    pub fn add_xyz_dest(&mut self, dest: XyzDestination) -> Ref {
        let root_ref = self.new_ref();
        self.xyz_dests.push((root_ref, dest));
        root_ref
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
                let use_type3 = font_ref.svg().is_ok()
                    || font_ref.colr().is_ok()
                    || font_ref.sbix().is_ok()
                    || font_ref.cbdt().is_ok()
                    || font_ref.ebdt().is_ok();

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
            Resource::Srgb => self.add_object(ICCBasedColorSpace(
                self.serialize_settings.pdf_version.rgb_icc(),
            )),
            Resource::LinearRgb => self.add_object(LinearRgbColorSpace),
            Resource::Luma => self.add_object(ICCBasedColorSpace(
                self.serialize_settings.pdf_version.grey_icc(),
            )),
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
                if let Some(font_info) = FontInfo::new(font_data.as_ref().as_ref(), index) {
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
        let icc_profile = self.serialize_settings.pdf_version.rgb_icc();

        oi.dest_output_profile(self.add_object(icc_profile.clone()))
            .subtype(subtype)
            .output_condition_identifier(TextStr("Custom"))
            .output_condition(TextStr("sRGB"))
            .registry_name(TextStr(""))
            .info(TextStr(
                format!(
                    "sRGB v{}.{}",
                    icc_profile.metadata().major,
                    icc_profile.metadata().minor
                )
                .as_str(),
            ));
        oi.finish();

        let mut array = chunk.indirect(root_ref).array();
        array.item(oi_ref);
        array.finish();

        chunk
    }

    pub fn finish(mut self) -> KrillaResult<Pdf> {
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
        } else {
            self.register_validation_error(ValidationError::MissingDocumentOutline);
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

        let xyz_dests = std::mem::take(&mut self.xyz_dests);
        for (ref_, dest) in &xyz_dests {
            let chunk = dest.serialize(&mut self, *ref_)?;
            self.chunk_container.destinations.push(chunk);
        }

        // It is important that we serialize the tags AFTER we have serialized the pages,
        // because page serialization will update the annotation refs of the page infos,
        // and when serializing the parent tree map we need to know the refs of the annotations
        let tag_tree = std::mem::take(&mut self.tag_tree);
        let struct_parents = std::mem::take(&mut self.struct_parents);
        if let Some(root) = &tag_tree {
            let mut parent_tree_map = HashMap::new();
            let mut id_tree_map = BTreeMap::new();
            let struct_tree_root_ref = self.new_ref();
            let (document_ref, struct_elems) = root.serialize(
                &mut self,
                &mut parent_tree_map,
                &mut id_tree_map,
                struct_tree_root_ref,
            )?;
            self.chunk_container.struct_elements = struct_elems;

            let mut chunk = Chunk::new();
            let mut tree = chunk.indirect(struct_tree_root_ref).start::<Dict>();
            tree.pair(Name(b"Type"), Name(b"StructTreeRoot"));
            let mut role_map = tree.insert(Name(b"RoleMap")).start::<RoleMap>();
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
                            refs.item(parent_tree_map.get(&rci.into()).ok_or(
                                KrillaError::UserError(
                                    "a identifier doesn't appear in the tag tree".to_string(),
                                ),
                            )?);
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

            if !id_tree_map.is_empty() {
                let mut id_tree = tree.insert(Name(b"IDTree")).start::<NameTree<Ref>>();
                let mut names = id_tree.names();

                for (name, ref_) in id_tree_map {
                    names.insert(Str(name.as_bytes()), ref_);
                }
            }

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
        let serialized = chunk_container.finish(&mut self)?;
        self.limits.merge(serialized.limits());

        if self.limits.str_len() > STR_LEN {
            self.register_validation_error(ValidationError::TooLongString);
        }

        if self.limits.name_len() > NAME_LEN {
            self.register_validation_error(ValidationError::TooLongName);
        }

        if self.limits.real() > MAX_FLOAT {
            self.register_validation_error(ValidationError::TooLargeFloat);
        }

        if self.limits.array_len() > ARRAY_LEN {
            self.register_validation_error(ValidationError::TooLongArray);
        }

        if self.limits.dict_entries() > DICT_LEN {
            self.register_validation_error(ValidationError::TooLongDictionary);
        }

        if !self.validation_errors.is_empty() {
            return Err(KrillaError::ValidationError(self.validation_errors));
        }

        // Just a sanity check.
        assert!(self.font_map.is_empty());
        assert!(self.pages.is_empty());

        Ok(serialized)
    }
}
