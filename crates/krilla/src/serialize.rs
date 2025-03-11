//! Serializing PDF documents.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::Arc;

use pdf_writer::types::StructRole;
use pdf_writer::writers::{NameTree, NumberTree, OutputIntent, RoleMap};
use pdf_writer::{Chunk, Dict, Finish, Limits, Name, Pdf, Ref, Str, TextStr};
use skrifa::raw::TableProvider;
use tiny_skia_path::Size;

use crate::chunk_container::ChunkContainer;
use crate::color::{ColorSpace, ICCBasedColorSpace, ICCProfile};
use crate::configure::{Configuration, PdfVersion, ValidationError, Validator};
use crate::destination::{NamedDestination, XyzDestination};
use crate::embed::EmbeddedFile;
use crate::error::{KrillaError, KrillaResult};
use crate::font::{Font, FontInfo};
#[cfg(feature = "raster-images")]
use crate::image::Image;
use crate::metadata::Metadata;
use crate::object::font::cid_font::CIDFont;
use crate::object::font::type3_font::Type3FontMapper;
use crate::object::font::{FontContainer, FontIdentifier};
use crate::object::outline::Outline;
use crate::object::page::{InternalPage, PageLabelContainer};
use crate::object::{Cacheable, Resourceable};
use crate::page::PageLabel;
use crate::resource;
use crate::resource::Resource;
use crate::surface::Location;
use crate::tagging::{AnnotationIdentifier, IdentifierType, PageTagIdentifier, TagTree};
use crate::util::SipHashable;

/// Settings that should be applied when creating a PDF document.
#[derive(Clone, Debug)]
pub struct SerializeSettings {
    /// Whether content streams should be compressed. Leads to significantly smaller file sizes,
    /// but also longer running times. It is highly recommended that you set this to true.
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
    /// A validator and PDF version used for export.
    ///
    /// In case validation fails, export will fail, and a list of validation errors that
    /// occurred will be returned instead of the PDF.
    ///
    /// **Important**: Make sure to carefully read the documentation of the [`validate`] module
    /// before using this feature! Just setting a validator might not be enough to ensure that
    /// your output conforms to the given standard, as some requirements are semantic in nature
    /// and cannot possibly be verified by krilla!
    ///
    /// However, as long as you carefully read and follow the documentation,
    /// you can be certain that the resulting document will conform to the standard (unless there
    /// is a bug).
    ///
    /// [`validate`]: crate::configure::validate
    pub configuration: Configuration,
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
}

impl SerializeSettings {
    pub(crate) fn pdf_version(&self) -> PdfVersion {
        self.configuration.version()
    }

    pub(crate) fn validator(&self) -> Validator {
        self.configuration.validator()
    }
}

impl Default for SerializeSettings {
    fn default() -> Self {
        Self {
            ascii_compatible: false,
            compress_content_streams: true,
            no_device_cs: false,
            xmp_metadata: true,
            cmyk_profile: None,
            configuration: Configuration::new(),
            enable_tagging: true,
        }
    }
}

/// Settings that should be applied when converting a SVG.
#[derive(Copy, Clone, Debug)]
pub struct SvgSettings {
    /// Whether text should be embedded as properly selectable text. Otherwise,
    /// it will be drawn as outlined paths instead.
    pub embed_text: bool,
    /// How much filters, which will be converted to bitmaps, should be scaled. Higher values
    /// mean better quality, but also bigger file sizes.
    pub filter_scale: f32,
}

impl Default for SvgSettings {
    fn default() -> Self {
        Self {
            embed_text: true,
            filter_scale: 4.0,
        }
    }
}

pub(crate) struct PageInfo {
    /// The reference of the page in the chunk.
    pub(crate) ref_: Ref,
    /// The page size, necessary so that we can convert from PDF coordinates to
    /// krilla coordinates.
    pub(crate) surface_size: Size,
    /// The refs of the annotations that are used by that page.
    ///
    /// Note that this will be empty be default when adding a new `PageInfo` to
    /// `page_infos` in `SerializeContext`, and only once we actually serialize
    /// the page will the annotations be populated.
    pub(crate) annotations: Vec<Ref>,
}

enum StructParentElement {
    /// The index of the page and the number of marked content IDs present on that page.
    Page(usize, i32),
    /// The index of the page where the annotation is present, as well as the index of the
    /// annotation within that one page.
    Annotation(usize, usize),
}

pub(crate) enum MaybeDeviceColorSpace {
    DeviceRgb,
    DeviceGray,
    DeviceCMYK,
    ColorSpace(resource::ColorSpace),
}

/// The serializer context is more or less the core piece of krilla. It is passed around
/// throughout pretty much the whole conversion process, and contains all mutable state
/// that is needed when writing a PDF file. This includes for example:
/// - Storing all chunks that are produced.
/// - The mappings from OTF fonts to CID/Type 3 fonts.
/// - Annotations used in the document.
///   etc.
pub(crate) struct SerializeContext {
    /// A cache for mapping `FontInfo`s to existing Font objects. Is mainly used to
    /// speed up SVG conversion, so that if we convert many SVGs with the same font,
    /// we can cache the font.
    font_cache: HashMap<Arc<FontInfo>, Font>,
    /// The ref of the page tree.
    page_tree_ref: Option<Ref>,
    /// All global objects, such as PDF fonts, that are populated over time.
    pub(crate) global_objects: GlobalObjects,
    /// Information for each page written so far, index by the page index.
    page_infos: Vec<PageInfo>,
    /// Keep track of object hashes and their corresponding reference. This is used for
    /// caching, so that for example same images will not be embedded twice in the document.
    cached_mappings: HashMap<u128, Ref>,
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
    /// The only reason why `SerializeContext` needs to know about them is that we also
    /// need to merge limits from postscript functions, which are not directly accessible
    /// from the chunk they are written to.
    limits: Limits,
    /// The current location, if set.
    pub(crate) location: Option<Location>,
}

impl SerializeContext {
    pub(crate) fn new(mut serialize_settings: SerializeSettings) -> Self {
        // Override flags as required by the validator
        serialize_settings.no_device_cs |= serialize_settings.validator().requires_no_device_cs();
        serialize_settings.enable_tagging |= serialize_settings.validator().requires_tagging();
        serialize_settings.xmp_metadata |= serialize_settings.validator().xmp_metadata();

        Self {
            cached_mappings: HashMap::new(),
            font_cache: HashMap::new(),
            global_objects: GlobalObjects::default(),
            cur_ref: Ref::new(1),
            chunk_container: ChunkContainer::new(),
            page_tree_ref: None,
            page_infos: vec![],
            location: None,
            validation_errors: vec![],
            serialize_settings: Arc::new(serialize_settings),
            limits: Limits::new(),
        }
    }

    pub(crate) fn page_infos(&self) -> &[PageInfo] {
        &self.page_infos
    }

    pub(crate) fn page_infos_mut(&mut self) -> &mut [PageInfo] {
        &mut self.page_infos
    }

    pub(crate) fn set_outline(&mut self, outline: Outline) {
        // Only set it if it's not empty or if the current validator requires an
        // outline.
        if !outline.is_empty()
            || self
                .serialize_settings
                .validator()
                .prohibits(&ValidationError::MissingDocumentOutline)
        {
            self.global_objects.outline = MaybeTaken::new(Some(outline));
        }
    }

    pub(crate) fn set_location(&mut self, location: Location) {
        self.location = Some(location)
    }

    pub(crate) fn reset_location(&mut self) {
        self.location = None
    }

    pub(crate) fn set_metadata(&mut self, metadata: Metadata) {
        self.chunk_container.metadata = Some(metadata);
    }

    pub(crate) fn embed_file(&mut self, file: EmbeddedFile) -> Option<()> {
        let name = file.path.clone();
        let ref_ = self.register_cacheable(file);
        if self
            .global_objects
            .embedded_files
            .insert(name, ref_)
            .is_some()
        {
            None
        } else {
            Some(())
        }
    }

    pub(crate) fn metadata(&self) -> Option<&Metadata> {
        self.chunk_container.metadata.as_ref()
    }

    pub(crate) fn set_tag_tree(&mut self, root: TagTree) {
        // Only set the tag tree if the user actually enabled tagging.
        if self.serialize_settings.enable_tagging {
            self.global_objects.tag_tree = MaybeTaken::new(Some(root))
        }
    }

    pub(crate) fn new_ref(&mut self) -> Ref {
        self.cur_ref.bump()
    }

    pub(crate) fn serialize_settings(&self) -> Arc<SerializeSettings> {
        self.serialize_settings.clone()
    }

    pub(crate) fn page_tree_ref(&mut self) -> Ref {
        *self
            .page_tree_ref
            .get_or_insert_with(|| self.cur_ref.bump())
    }

    pub(crate) fn register_font_container(&mut self, font: Font) -> Rc<RefCell<FontContainer>> {
        self.global_objects
            .font_map
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
                let use_type3 = if !font.allow_color() {
                    false
                } else {
                    font_ref.svg().is_ok()
                        || font_ref.colr().is_ok()
                        || font_ref.sbix().is_ok()
                        || font_ref.cbdt().is_ok()
                        || font_ref.ebdt().is_ok()
                };

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

    #[cfg(feature = "svg")]
    pub(crate) fn convert_fontdb(
        &mut self,
        db: &mut fontdb::Database,
        ids: Option<Vec<fontdb::ID>>,
    ) -> HashMap<fontdb::ID, Font> {
        let mut map = HashMap::new();

        let ids = ids.unwrap_or(db.faces().map(|f| f.id).collect::<Vec<_>>());

        for id in ids {
            // What we could do is just go through each font and then create a new Font object for each of them.
            // However, this is somewhat wasteful and expensive, because we have to hash each font, which
            // can go be multiple MB. So instead, we first construct a font info object, which is much
            // cheaper, and then check whether we already have a corresponding font object in the cache.
            // If not, we still need to construct it.
            if let Some((font_data, index)) = unsafe { db.make_shared_face_data(id) } {
                if let Some(font_info) = FontInfo::new(font_data.as_ref().as_ref(), index, true) {
                    let font_info = Arc::new(font_info);
                    let font = self
                        .font_cache
                        .get(&font_info.clone())
                        .cloned()
                        .unwrap_or(Font::new_with_info(font_data.into(), font_info).unwrap());
                    map.insert(id, font);
                }
            }
        }

        map
    }

    pub(crate) fn finish(mut self) -> KrillaResult<Pdf> {
        // We need to be careful here that we serialize the objects in the right order,
        // as in some cases we use MaybeTake::take to remove an object, which means that
        // no object that is serialized afterwards must depend on it.

        // Serialize all objects that can only be written in the end.
        self.serialize_destination_profiles();
        self.serialize_page_label_tree();
        self.serialize_outline()?;
        self.serialize_fonts()?;
        self.serialize_pages()?;
        self.serialize_page_tree();
        self.serialize_xyz_destinations()?;
        // It is important that we serialize the tags AFTER we have serialized the pages,
        // because page serialization will update the annotation refs of the page infos,
        // and when serializing the parent tree map we need to know the refs of the annotations
        self.serialize_tag_tree()?;

        // Create the final PDF.
        let pdf = {
            let chunk_container = std::mem::take(&mut self.chunk_container);
            chunk_container.finish(&mut self)?
        };
        self.register_limits(pdf.limits());

        self.check_limits();

        if !self.validation_errors.is_empty() {
            // Deduplicate errors with no span.
            self.validation_errors.dedup();
            return Err(KrillaError::ValidationError(self.validation_errors));
        }

        // Just a sanity check that we've actually processed all items.
        self.global_objects.assert_all_taken();

        Ok(pdf)
    }
}

/// Various registration methods.
impl SerializeContext {
    pub(crate) fn register_validation_error(&mut self, error: ValidationError) {
        if self.serialize_settings.validator().prohibits(&error) {
            self.validation_errors.push(error);
        }
    }

    pub(crate) fn register_limits(&mut self, limits: &Limits) {
        self.limits.merge(limits);
    }

    pub(crate) fn register_page_struct_parent(
        &mut self,
        page_index: usize,
        num_mcids: i32,
    ) -> Option<i32> {
        if self.serialize_settings.enable_tagging {
            if num_mcids == 0 {
                return None;
            }

            let id = self.global_objects.struct_parents.len();
            self.global_objects
                .struct_parents
                .push(StructParentElement::Page(page_index, num_mcids));
            Some(i32::try_from(id).unwrap())
        } else {
            None
        }
    }

    pub(crate) fn register_annotation_parent(
        &mut self,
        page_index: usize,
        annotation_index: usize,
    ) -> Option<i32> {
        if self.serialize_settings.enable_tagging {
            let id = self.global_objects.struct_parents.len();
            self.global_objects
                .struct_parents
                .push(StructParentElement::Annotation(
                    page_index,
                    annotation_index,
                ));
            Some(i32::try_from(id).unwrap())
        } else {
            None
        }
    }

    pub(crate) fn register_named_destination(&mut self, nd: NamedDestination) {
        let dest_ref = self.register_xyz_destination((*nd.xyz_dest).clone());
        self.global_objects.named_destinations.insert(nd, dest_ref);
    }

    pub(crate) fn register_page(&mut self, page: InternalPage) {
        let ref_ = self.new_ref();
        self.page_infos.push(PageInfo {
            ref_,
            surface_size: page.page_settings.surface_size(),
            // Will be populated when the page is serialized.
            annotations: vec![],
        });
        self.global_objects.pages.push((ref_, page));
    }

    fn register_cached<T: SipHashable>(
        &mut self,
        item: T,
        mut func: impl FnMut(&mut Self, T, Ref),
    ) -> Ref {
        let hash = item.sip_hash();
        if let Some(_ref) = self.cached_mappings.get(&hash) {
            *_ref
        } else {
            let root_ref = self.new_ref();
            func(self, item, root_ref);
            self.cached_mappings.insert(hash, root_ref);
            root_ref
        }
    }

    pub(crate) fn register_cacheable<T>(&mut self, object: T) -> Ref
    where
        T: Cacheable,
    {
        self.register_cached(object, |sc, object, root_ref| {
            let mut chunk_container_fn = object.chunk_container();
            let chunk = object.serialize(sc, root_ref);
            chunk_container_fn(&mut sc.chunk_container).push(chunk);
        })
    }

    pub(crate) fn register_resourceable<T>(&mut self, object: T) -> T::Resource
    where
        T: Resourceable,
    {
        Resource::new(self.register_cacheable(object))
    }

    #[cfg(feature = "raster-images")]
    pub(crate) fn register_image(&mut self, image: Image) -> Ref {
        self.register_cached(image, |sc, object, root_ref| {
            let chunk = object.serialize(sc, root_ref);
            sc.chunk_container.images.push(chunk);
        })
    }

    pub(crate) fn register_xyz_destination(&mut self, dest: XyzDestination) -> Ref {
        self.register_cached(dest, |sc, object, root_ref| {
            sc.global_objects.xyz_destinations.push((root_ref, object));
        })
    }

    pub(crate) fn register_page_label(&mut self, page_label: PageLabel) -> Ref {
        let ref_ = self.new_ref();
        let chunk = page_label.serialize(ref_);
        self.chunk_container.page_labels.push(chunk);
        ref_
    }

    pub(crate) fn register_font_identifier(&mut self, f: FontIdentifier) -> resource::Font {
        let hash = f.sip_hash();
        if let Some(_ref) = self.cached_mappings.get(&hash) {
            resource::Font::new(*_ref)
        } else {
            let root_ref = self.new_ref();
            self.cached_mappings.insert(hash, root_ref);
            resource::Font::new(root_ref)
        }
    }

    pub(crate) fn register_colorspace(&mut self, cs: ColorSpace) -> MaybeDeviceColorSpace {
        match cs {
            ColorSpace::Srgb => MaybeDeviceColorSpace::ColorSpace(self.register_resourceable(
                ICCBasedColorSpace(self.serialize_settings.pdf_version().rgb_icc()),
            )),
            ColorSpace::Luma => MaybeDeviceColorSpace::ColorSpace(self.register_resourceable(
                ICCBasedColorSpace(self.serialize_settings.pdf_version().grey_icc()),
            )),
            ColorSpace::Cmyk(cs) => {
                MaybeDeviceColorSpace::ColorSpace(self.register_resourceable(cs))
            }
            ColorSpace::DeviceGray => MaybeDeviceColorSpace::DeviceGray,
            ColorSpace::DeviceRgb => MaybeDeviceColorSpace::DeviceRgb,
            ColorSpace::DeviceCmyk => MaybeDeviceColorSpace::DeviceCMYK,
        }
    }
}

/// Various serialization methods.
/// All methods are supposed to only be called once in `SerializeContext::finish`!
impl SerializeContext {
    fn serialize_destination_profiles(&mut self) {
        let validator = self.serialize_settings.validator();
        self.chunk_container.destination_profiles = validator.output_intent().map(|subtype| {
            let root_ref = self.new_ref();
            let mut chunk = Chunk::new();

            let oi_ref = self.new_ref();
            let mut oi = chunk.indirect(oi_ref).start::<OutputIntent>();
            let icc_profile = self.serialize_settings.pdf_version().rgb_icc();

            oi.dest_output_profile(self.register_cacheable(icc_profile.clone()))
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

            (root_ref, chunk)
        });
    }

    fn serialize_page_label_tree(&mut self) {
        if let Some(container) = PageLabelContainer::new(
            &self
                .global_objects
                .pages
                .iter()
                .map(|(_, p)| p.page_settings.page_label().clone())
                .collect::<Vec<_>>(),
        ) {
            let page_label_tree_ref = self.new_ref();
            let chunk = container.serialize(self, page_label_tree_ref);
            self.chunk_container.page_label_tree = Some((page_label_tree_ref, chunk));
        }
    }

    fn serialize_outline(&mut self) -> KrillaResult<()> {
        let outline = self.global_objects.outline.take();
        if let Some(outline) = &outline {
            let outline_ref = self.new_ref();
            let chunk = outline.serialize(self, outline_ref)?;
            self.chunk_container.outline = Some((outline_ref, chunk));
        } else {
            self.register_validation_error(ValidationError::MissingDocumentOutline);
        }

        Ok(())
    }

    fn serialize_fonts(&mut self) -> KrillaResult<()> {
        let fonts = self.global_objects.font_map.take();
        for font_container in fonts.values() {
            match &*font_container.borrow() {
                FontContainer::Type3(font_mapper) => {
                    for t3_font in font_mapper.fonts() {
                        let f = self.register_font_identifier(t3_font.identifier());
                        let chunk = t3_font.serialize(self, f.get_ref());
                        self.chunk_container.fonts.push(chunk);
                    }
                }
                FontContainer::CIDFont(cid_font) => {
                    let f = self.register_font_identifier(cid_font.identifier());
                    let chunk = cid_font.serialize(self, f.get_ref())?;
                    self.chunk_container.fonts.push(chunk);
                }
            }
        }

        Ok(())
    }

    fn serialize_pages(&mut self) -> KrillaResult<()> {
        let pages = self.global_objects.pages.take();
        for (ref_, page) in pages {
            let chunk = page.serialize(self, ref_)?;
            self.chunk_container.pages.push(chunk);
        }

        Ok(())
    }

    fn serialize_page_tree(&mut self) {
        if let Some(page_tree_ref) = self.page_tree_ref {
            let mut page_tree_chunk = Chunk::new();
            page_tree_chunk
                .pages(page_tree_ref)
                .count(self.page_infos.len() as i32)
                .kids(self.page_infos.iter().map(|i| i.ref_));
            self.chunk_container.page_tree = Some((page_tree_ref, page_tree_chunk));
        }
    }

    fn serialize_xyz_destinations(&mut self) -> KrillaResult<()> {
        let xyz_destinations = self.global_objects.xyz_destinations.take();
        for (ref_, dest) in &xyz_destinations {
            let chunk = dest.serialize(self, *ref_)?;
            self.chunk_container.destinations.push(chunk);
        }

        Ok(())
    }

    fn serialize_tag_tree(&mut self) -> KrillaResult<()> {
        let tag_tree = self.global_objects.tag_tree.take();
        let struct_parents = self.global_objects.struct_parents.take();
        if let Some(root) = &tag_tree {
            let mut parent_tree_map = HashMap::new();
            let mut id_tree_map = BTreeMap::new();
            let struct_tree_root_ref = self.new_ref();
            let (document_ref, struct_elems) = root.serialize(
                self,
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
                                    "an identifier doesn't appear in the tag tree".to_string(),
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
        } else {
            self.register_validation_error(ValidationError::MissingTagging);
        }

        Ok(())
    }

    fn check_limits(&mut self) {
        const STR_LEN: usize = 32767;
        const NAME_LEN: usize = 127;

        // These only apply to PDF 1.4 and PDF/A-1.
        const MAX_FLOAT: f32 = 32767.0;
        const DICT_LEN: usize = 4095;
        const ARRAY_LEN: usize = 8191;

        if self.cur_ref > Ref::new(8388607) {
            self.register_validation_error(ValidationError::TooManyIndirectObjects)
        }

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
    }
}

/// This struct is essentially a thin wrapper around `std::mem::replace`. When finishing the
/// document, we need to take ownership of many of the items in `GlobalObjects` in order to
/// prevent having to clone them. However, the problem is that we cannot easily take ownership
/// of them, because they are part of the SerializeContext. Because of this, what we
/// do is that we `std::mem::replace` the elements step by step and then serialize them.
/// The `MaybeTaken` struct helps us to ensure that once we have taken a value, we do not
/// accidentally attempt to write/read it again.
pub(crate) struct MaybeTaken<T>(Option<T>);

impl<T> MaybeTaken<T> {
    pub(crate) fn new(item: T) -> Self {
        Self(Some(item))
    }

    pub(crate) fn is_taken(&self) -> bool {
        self.0.is_none()
    }
}

impl<T> MaybeTaken<T> {
    #[track_caller]
    pub(crate) fn take(&mut self) -> T {
        self.0.take().expect("value was already taken before")
    }
}

impl<T: Default> Default for MaybeTaken<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Deref for MaybeTaken<T> {
    type Target = T;

    #[track_caller]
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("value was taken")
    }
}

impl<T> DerefMut for MaybeTaken<T> {
    #[track_caller]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("value was taken")
    }
}

#[derive(Default)]
pub(crate) struct GlobalObjects {
    /// All named destinations that have been registered, including a Ref to their destination.
    // Needs to be pub(crate) because writing of named destinations happens in `ChunkContainer`.
    pub(crate) named_destinations: MaybeTaken<HashMap<NamedDestination, Ref>>,
    /// A map from fonts to font container.
    font_map: MaybeTaken<HashMap<Font, Rc<RefCell<FontContainer>>>>,
    /// All XYZ destinations used in the document. The reason we need to store them
    /// separately is that we can only serialize them in the very end, once all pages
    /// have been written, so that we know the Ref of the page they belong to.
    xyz_destinations: MaybeTaken<Vec<(Ref, XyzDestination)>>,
    /// All pages and their corresponding chunks. Similarly to destinations, they need
    /// to be written in the very end, because pages might contain annotations which in turn
    /// depend on future pages (not written yet), so pages must also only be written in the
    /// very end.
    pages: MaybeTaken<Vec<(Ref, InternalPage)>>,
    /// Stores the struct parent elements.
    struct_parents: MaybeTaken<Vec<StructParentElement>>,
    /// Stores the document outline.
    outline: MaybeTaken<Option<Outline>>,
    /// Stores the tag tree.
    tag_tree: MaybeTaken<Option<TagTree>>,
    /// Stores the association of the names of embedded files to their refs,
    /// for the catalog dictionary.
    pub(crate) embedded_files: MaybeTaken<BTreeMap<String, Ref>>,
}

impl GlobalObjects {
    pub(crate) fn assert_all_taken(&self) {
        assert!(self.named_destinations.is_taken());
        assert!(self.font_map.is_taken());
        assert!(self.xyz_destinations.is_taken());
        assert!(self.pages.is_taken());
        assert!(self.struct_parents.is_taken());
        assert!(self.outline.is_taken());
        assert!(self.tag_tree.is_taken());
        assert!(self.embedded_files.is_taken());
    }
}
