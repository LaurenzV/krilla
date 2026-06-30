use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::num::NonZeroU16;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::Arc;

use indexmap::IndexMap;
use pdf_writer::types::{OutputIntentSubtype, StructRole, StructRole2};
use pdf_writer::writers::{FileSpec, OutputIntent, StructTreeRoot};
use pdf_writer::{Chunk, Content, Finish, Limits, Name, Pdf, Ref, Settings, Str, TextStr};

use crate::chunk_container::ChunkContainer;
use crate::color::{CieBasedColorSpace, DeviceColorSpace, SpecialColorSpace};
use crate::configure::validate::ValidationStore;
use crate::configure::{Configuration, PdfVersion, ValidationError, Validators};
use crate::error::{KrillaError, KrillaResult, LimitError};
use crate::geom::Size;
use crate::graphics::color::{rgb, ColorSpace};
use crate::graphics::icc::{GenericICCProfile, ICCBasedColorSpace, ICCColorSpace, ICCProfile};
#[cfg(feature = "raster-images")]
use crate::graphics::image::Image;
use crate::graphics::separation::SeparationColorSpace;
use crate::interactive::destination::{NamedDestination, XyzDestination};
use crate::interchange::embed::EmbeddedFile;
use crate::interchange::outline::Outline;
use crate::interchange::tagging::{AnnotationIdentifier, PageTagIdentifier, TagTree};
use crate::page::{InternalPage, PageLabel, PageLabelContainer};
#[cfg(feature = "pdf")]
use crate::pdf::{PdfDocument, PdfSerializerContext};
use crate::resource;
use crate::resource::{Resource, Resourceable};
use crate::surface::{Location, Surface};
use crate::text::GlyphId;
use crate::text::{Font, FontContainer, FontIdentifier};
use crate::util::SipHashable;

const STR_LEN: usize = 32767;
const NAME_LEN: usize = 127;

// These only apply to PDF 1.4 and PDF/A-1.
const MAX_FLOAT: f32 = 32767.0;
const DICT_LEN: usize = 4095;
const ARRAY_LEN: usize = 8191;

/// Settings that should be applied when creating a PDF document.
#[derive(Clone, Debug)]
pub struct SerializeSettings {
    /// Whether to write PDFs in a way that is easier to inspect manually. This
    /// will result in larger file sizes.
    pub pretty: bool,
    /// Whether content streams should be compressed. Leads to significantly smaller file sizes,
    /// but also longer running times. It is highly recommended that you set this to `true`.
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
    /// that the header itself will not be ASCII-compatible. Finally, embedded PDFs will
    /// be embedded as is and not re-encoded with ASCII-compatible encoding.
    pub ascii_compatible: bool,
    /// Whether the PDF should include XMP metadata.
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
    ///
    /// For PDF/X variants that embed their output intent (PDF/X-1a, PDF/X-3,
    /// PDF/X-4, PDF/X-6), this profile is also used as the embedded
    /// printer/output profile for the PDF/X output intent.
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
    /// you use. For example, when exporting with PDF/UA, this value will always
    /// be set to `true`.
    ///
    /// [`tagging`]: crate::interchange::tagging
    pub enable_tagging: bool,
    /// A function that should be used to render SVG glyphs. If you don't need this, yu can
    /// just use the default function which doesn't render them at all. If you do want this, it
    /// is recommended that you use the function provided by the `krilla-svg` crate.
    pub render_svg_glyph_fn: RenderSvgGlyphFn,
    /// An external ICC profile reference used by PDF/X-4p and PDF/X-6p.
    ///
    /// This setting is required when exporting with [`Prepress::X4P`] or
    /// [`Prepress::X6P`]. In those modes, the PDF/X output intent references
    /// the ICC profile externally instead of embedding it in the PDF.
    ///
    /// Supplying this setting when no PDF/X-4p or PDF/X-6p validator is active
    /// is rejected during validation.
    ///
    /// [`Prepress::X4P`]: crate::configure::Prepress::X4P
    /// [`Prepress::X6P`]: crate::configure::Prepress::X6P
    pub external_output_profile: Option<ExternalOutputProfile>,
}

pub type RenderSvgGlyphFn = fn(&[u8], rgb::Color, GlyphId, (f32, f32), &mut Surface) -> Option<()>;

/// A reference to an externally hosted output profile for PDF/X-4p and
/// PDF/X-6p.
///
/// Construction validates the required fields eagerly; the type guarantees by
/// construction that at least one non-empty URL, a non-empty output condition
/// identifier, and a non-empty informational string are present.
#[derive(Clone, Debug)]
pub struct ExternalOutputProfile {
    urls: Vec<String>,
    profile: GenericICCProfile,
    output_condition_identifier: String,
    output_condition: Option<String>,
    registry_name: Option<String>,
    info: String,
}

/// Reason construction of an [`ExternalOutputProfile`] failed.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ExternalOutputProfileError {
    /// The `urls` vector was empty or contained only empty/whitespace strings.
    EmptyUrls,
    /// The output condition identifier was empty or only whitespace.
    EmptyIdentifier,
    /// The information string was empty or only whitespace.
    EmptyInfo,
    /// The profile's ICC data colour space is not the one implied by the
    /// constructor — i.e. not `GRAY` for [`ExternalOutputProfile::luma`], `RGB `
    /// for [`ExternalOutputProfile::rgb`], or `CMYK` for
    /// [`ExternalOutputProfile::cmyk`]. A PDF/X output-intent profile must have a
    /// `GRAY`/`RGB `/`CMYK` data colour space (ISO 15930-7 §6.4.1, Annex A.2);
    /// a same-channel-count profile with a different signature (e.g. `Lab `,
    /// `1CLR`, `4CLR`/DeviceN) is rejected.
    WrongColorSpace,
}

impl core::fmt::Display for ExternalOutputProfileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let reason = match self {
            ExternalOutputProfileError::EmptyUrls => "at least one non-empty URL must be provided",
            ExternalOutputProfileError::EmptyIdentifier => {
                "the output condition identifier must be non-empty"
            }
            ExternalOutputProfileError::EmptyInfo => "the informational string must be non-empty",
            ExternalOutputProfileError::WrongColorSpace => {
                "the profile's data colour space must be GRAY, RGB or CMYK and match the constructor"
            }
        };
        f.write_str(reason)
    }
}

impl std::error::Error for ExternalOutputProfileError {}

impl ExternalOutputProfile {
    /// Create an external RGB output profile reference.
    ///
    /// # Errors
    ///
    /// Returns [`ExternalOutputProfileError::EmptyUrls`],
    /// [`ExternalOutputProfileError::EmptyIdentifier`], or
    /// [`ExternalOutputProfileError::EmptyInfo`] if any of `urls`,
    /// `output_condition_identifier`, or `info` is empty (or contains only
    /// whitespace) after trimming; or [`ExternalOutputProfileError::WrongColorSpace`]
    /// if the profile's ICC data colour space is not the one implied by the
    /// constructor (`GRAY` for [`luma`](Self::luma), `RGB ` for
    /// [`rgb`](Self::rgb), `CMYK` for [`cmyk`](Self::cmyk)).
    pub fn rgb(
        profile: ICCProfile<3>,
        urls: Vec<String>,
        output_condition_identifier: String,
        info: String,
    ) -> Result<Self, ExternalOutputProfileError> {
        Self::new(
            GenericICCProfile::Rgb(profile),
            urls,
            output_condition_identifier,
            info,
        )
    }

    /// Create an external grayscale output profile reference.
    ///
    /// # Errors
    ///
    /// See [`ExternalOutputProfile::rgb`].
    pub fn luma(
        profile: ICCProfile<1>,
        urls: Vec<String>,
        output_condition_identifier: String,
        info: String,
    ) -> Result<Self, ExternalOutputProfileError> {
        Self::new(
            GenericICCProfile::Luma(profile),
            urls,
            output_condition_identifier,
            info,
        )
    }

    /// Create an external CMYK output profile reference.
    ///
    /// # Errors
    ///
    /// See [`ExternalOutputProfile::rgb`].
    pub fn cmyk(
        profile: ICCProfile<4>,
        urls: Vec<String>,
        output_condition_identifier: String,
        info: String,
    ) -> Result<Self, ExternalOutputProfileError> {
        Self::new(
            GenericICCProfile::Cmyk(profile),
            urls,
            output_condition_identifier,
            info,
        )
    }

    fn new(
        profile: GenericICCProfile,
        urls: Vec<String>,
        output_condition_identifier: String,
        info: String,
    ) -> Result<Self, ExternalOutputProfileError> {
        // ISO 15930-7 §6.4.1 / Annex A.2: a PDF/X output-intent profile shall
        // have a GRAY, RGB or CMYK data colour space. The typed constructors fix
        // the channel count, but a same-channel-count profile can still carry a
        // different signature (e.g. a 3-channel Lab profile), so verify it here.
        let expected = match &profile {
            GenericICCProfile::Luma(_) => ICCColorSpace::Gray,
            GenericICCProfile::Rgb(_) => ICCColorSpace::Rgb,
            GenericICCProfile::Cmyk(_) => ICCColorSpace::Cmyk,
        };
        if profile.metadata().color_space != expected {
            return Err(ExternalOutputProfileError::WrongColorSpace);
        }
        let urls = trim_url_list(urls).ok_or(ExternalOutputProfileError::EmptyUrls)?;
        let output_condition_identifier = trim_required(output_condition_identifier)
            .ok_or(ExternalOutputProfileError::EmptyIdentifier)?;
        let info = trim_required(info).ok_or(ExternalOutputProfileError::EmptyInfo)?;
        Ok(Self {
            urls,
            profile,
            output_condition_identifier,
            output_condition: None,
            registry_name: None,
            info,
        })
    }

    /// Set a human-readable output condition string. Empty or whitespace-only
    /// values are discarded.
    pub fn with_output_condition(mut self, output_condition: String) -> Self {
        self.output_condition = normalize_optional_string(output_condition);
        self
    }

    /// Set the registry name for the output condition identifier. Empty or
    /// whitespace-only values are discarded.
    pub fn with_registry_name(mut self, registry_name: String) -> Self {
        self.registry_name = normalize_optional_string(registry_name);
        self
    }

    /// Return the referenced profile URLs.
    pub fn urls(&self) -> &[String] {
        &self.urls
    }

    /// Return the output condition identifier.
    pub fn output_condition_identifier(&self) -> &str {
        &self.output_condition_identifier
    }

    /// Return the optional human-readable output condition string.
    pub fn output_condition(&self) -> Option<&str> {
        self.output_condition.as_deref()
    }

    /// Return the optional registry name.
    pub fn registry_name(&self) -> Option<&str> {
        self.registry_name.as_deref()
    }

    /// Return the informational string for the output condition.
    pub fn info(&self) -> &str {
        &self.info
    }

    pub(crate) fn profile(&self) -> &GenericICCProfile {
        &self.profile
    }
}

fn normalize_optional_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn trim_required(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn trim_url_list(urls: Vec<String>) -> Option<Vec<String>> {
    let trimmed: Vec<String> = urls
        .into_iter()
        .filter_map(|url| {
            let t = url.trim();
            (!t.is_empty()).then(|| t.to_string())
        })
        .collect();
    (!trimmed.is_empty()).then_some(trimmed)
}

/// Whether an ICC profile version is too new to be a PDF/X output-intent
/// profile for the given PDF version.
///
/// The PDF 1.4-based levels (PDF/X-1a, PDF/X-3) admit only ICC v2. The PDF
/// 1.6-based levels (PDF/X-4, PDF/X-4p) admit ICC v4 up to v4.2 (ISO 15930-7
/// §6.4.2.1, citing ISO 15076-1:2005). The PDF 2.0-based levels (PDF/X-6,
/// PDF/X-6p) admit ICC v4 up to v4.3 (ISO 15930-9, citing ISO 15076-1:2010).
fn output_profile_version_too_new(pdf_version: PdfVersion, major: u8, minor: u8) -> bool {
    match pdf_version {
        PdfVersion::Pdf14 => major > 2,
        PdfVersion::Pdf15 => major > 4,
        PdfVersion::Pdf16 | PdfVersion::Pdf17 => major > 4 || (major == 4 && minor > 2),
        PdfVersion::Pdf20 => major > 4 || (major == 4 && minor > 3),
    }
}

impl SerializeSettings {
    pub(crate) fn pdf_version(&self) -> PdfVersion {
        self.configuration.version()
    }

    pub(crate) fn validators(&self) -> Validators {
        self.configuration.validators()
    }

    /// Whether the `/AF` key is supported, accounting for the PDF version and active standards.
    pub(crate) fn supports_associated_files(&self) -> bool {
        self.configuration.version().specifies_associated_files()
            || self.configuration.validators().specifies_associated_files()
    }

    /// Whether the PDF/X (`GTS_PDFX`) output intent's profile is a CMYK device
    /// profile, which is required to characterize the DeviceCMYK content krilla
    /// emits under PDF/X. `None` if no output-target profile is configured; a
    /// missing profile is reported separately (`MissingCMYKProfile` /
    /// `MissingExternalOutputProfile`).
    pub(crate) fn pdfx_output_intent_is_cmyk(&self) -> Option<bool> {
        if self.validators().requires_external_output_profile() {
            // The wrapper variant now matches the ICC data colour space
            // (validated in `ExternalOutputProfile::new`), so an RGB or
            // grayscale external intent simply reports a non-CMYK colour space.
            self.external_output_profile
                .as_ref()
                .map(|p| p.profile().metadata().color_space == ICCColorSpace::Cmyk)
        } else {
            self.cmyk_profile
                .as_ref()
                .map(|p| p.metadata().color_space == ICCColorSpace::Cmyk)
        }
    }

    /// Whether the PDF/X (`GTS_PDFX`) output intent's profile is an RGB profile.
    /// Only the external (`-p`) variants can have an RGB output intent; the
    /// embedded variants always use the 4-channel `cmyk_profile`.
    pub(crate) fn pdfx_output_intent_is_rgb(&self) -> bool {
        self.validators().requires_external_output_profile()
            && self
                .external_output_profile
                .as_ref()
                .is_some_and(|p| p.profile().metadata().color_space == ICCColorSpace::Rgb)
    }
}

impl Default for SerializeSettings {
    fn default() -> Self {
        Self {
            pretty: false,
            ascii_compatible: false,
            compress_content_streams: true,
            no_device_cs: false,
            xmp_metadata: true,
            cmyk_profile: None,
            configuration: Configuration::default(),
            enable_tagging: true,
            render_svg_glyph_fn: |_, _, _, _, _| None,
            external_output_profile: None,
        }
    }
}

pub(crate) enum PageInfo {
    /// A page built with krilla.
    Krilla {
        /// The reference of the page in the chunk.
        ref_: Ref,
        /// The page size, necessary so that we can convert from PDF coordinates to
        /// krilla coordinates.
        surface_size: Size,
        /// The refs of the annotations that are used by that page, and optionally
        /// a ref to their struct parent in the tag tree.
        ///
        /// Note that this will be empty be default when adding a new `PageInfo` to
        /// `page_infos` in `SerializeContext`, and only once we actually serialize
        /// the page will the annotations be populated.
        annotations: Vec<(Ref, OnceCell<Ref>)>,
        /// The page label of the page.
        page_label: PageLabel,
    },
    /// A page embedded from an external PDF file.
    #[allow(dead_code)]
    Pdf {
        ref_: Ref,
        size: Size,
        page_label: PageLabel,
    },
}

impl PageInfo {
    pub(crate) fn ref_(&self) -> Ref {
        match self {
            PageInfo::Krilla { ref_, .. } => *ref_,
            PageInfo::Pdf { ref_, .. } => *ref_,
        }
    }

    pub(crate) fn size(&self) -> Size {
        match self {
            PageInfo::Krilla { surface_size, .. } => *surface_size,
            PageInfo::Pdf { size, .. } => *size,
        }
    }

    pub(crate) fn page_label(&self) -> &PageLabel {
        match self {
            PageInfo::Krilla { page_label, .. } => page_label,
            PageInfo::Pdf { page_label, .. } => page_label,
        }
    }

    pub(crate) fn annotations(&self) -> &[(Ref, OnceCell<Ref>)] {
        match self {
            PageInfo::Krilla { annotations, .. } => annotations,
            PageInfo::Pdf { .. } => &[],
        }
    }

    pub(crate) fn annotations_mut(&mut self) -> &mut [(Ref, OnceCell<Ref>)] {
        match self {
            PageInfo::Krilla { annotations, .. } => annotations,
            PageInfo::Pdf { .. } => &mut [],
        }
    }
}

enum StructParentElement {
    /// The index of the page and the number of marked content IDs present on that page.
    Page(usize, i32),
    /// The index of the page where the annotation is present, as well as the index of the
    /// annotation within that one page.
    Annotation(AnnotationIdentifier),
}

#[derive(Debug)]
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
    /// The ref of the page tree.
    page_tree_ref: Ref,
    /// PDF 2.0 namespaces.
    pub(crate) pdf2_ns: Pdf2Namespaces,
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
    pub(crate) cur_ref: Ref,
    /// All validation errors that are collected as part of the export process
    /// alongside the validators that raised the error.
    validation_errors: Vec<(ValidationError, Validators)>,
    /// Settings used for serialization.
    serialize_settings: Arc<SerializeSettings>,
    /// Settings used for all PDF object chunks.
    chunk_settings: Settings,
    /// The limits created as part of the serialization process. In principle, we could
    /// just keep track of this in `ChunkContainer`, where all used chunks are stored.
    /// The only reason why `SerializeContext` needs to know about them is that we also
    /// need to merge limits from postscript functions, which are not directly accessible
    /// from the chunk they are written to.
    limits: Limits,
    /// Additional information stored during serialization that allows us to
    /// raise standards errors later.
    validation_store: ValidationStore,
    /// The current location, if set.
    pub(crate) location: Option<Location>,
}

impl SerializeContext {
    pub(crate) fn new(mut serialize_settings: SerializeSettings) -> Self {
        // Override flags as required by the validator
        serialize_settings.no_device_cs |= serialize_settings.validators().requires_no_device_cs();
        serialize_settings.enable_tagging |= serialize_settings.validators().requires_tagging();
        serialize_settings.xmp_metadata |= serialize_settings.validators().requires_xmp_metadata();

        let mut cur_ref = Ref::new(1);
        let page_tree_ref = cur_ref.bump();
        let pdf2_ns = Pdf2Namespaces {
            ssn_ref: cur_ref.bump(),
            krilla_ref: cur_ref.bump(),
        };

        let chunk_settings = Settings {
            pretty: serialize_settings.pretty,
        };

        // An external output profile is only meaningful for PDF/X-4p and
        // PDF/X-6p. If one was supplied but no active validator makes use of
        // it, record a validation error.
        let unsupported_external_output_profile =
            serialize_settings.external_output_profile.is_some()
                && !serialize_settings
                    .validators()
                    .requires_external_output_profile();

        let mut ctx = Self {
            cached_mappings: HashMap::new(),
            pdf2_ns,
            global_objects: GlobalObjects::default(),
            cur_ref,
            page_tree_ref,
            page_infos: vec![],
            location: None,
            validation_errors: vec![],
            serialize_settings: Arc::new(serialize_settings),
            chunk_settings,
            limits: Limits::new(),
            validation_store: ValidationStore::new(),
        };

        if unsupported_external_output_profile {
            ctx.register_validation_error(
                ValidationError::ExternalOutputProfileUnsupportedByValidator,
            );
        }

        ctx
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
                .validators()
                .prohibits(&ValidationError::MissingDocumentOutline)
                .is_some()
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

    pub(crate) fn embed_file(
        &mut self,
        chunk_container: &mut ChunkContainer,
        file: EmbeddedFile,
    ) -> Option<()> {
        let name = file.path.clone();
        let ref_ = self.register_cacheable(chunk_container, file);
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

    // IMPORTANT: DO NEVER CALL `Chunk::new`, `Pdf::new` or `Content::new` directly! Instead,
    // always make sure to use the methods on `SerializeContext`, to ensure the
    // flags are applied consistently across all chunks.

    pub(crate) fn new_chunk(&self) -> Chunk {
        Chunk::with_settings(self.chunk_settings)
    }

    pub(crate) fn new_content(&self) -> Content {
        Content::with_settings(self.chunk_settings)
    }

    pub(crate) fn new_pdf_with_capacity(&self, capacity: usize) -> Pdf {
        Pdf::with_settings_and_capacity(self.chunk_settings, capacity)
    }

    #[cfg(feature = "pdf")]
    pub(crate) fn chunk_settings(&self) -> Settings {
        self.chunk_settings
    }

    #[cfg(feature = "pdf")]
    pub(crate) fn embed_pdf_pages(&mut self, pdf: &PdfDocument, page_indices: &[usize]) {
        for page_idx in page_indices {
            let page_ref = self.new_ref();
            let size = pdf
                .pages()
                .get(*page_idx)
                .and_then(|p| {
                    let (x, y) = p.render_dimensions();
                    Size::from_wh(x, y)
                })
                // In case the page doesn't exist, we will catch the error later, so just use
                // a dummy size.
                .unwrap_or(Size::from_wh(1.0, 1.0).unwrap());
            self.global_objects
                .pdf_ctx
                .add_page(pdf, *page_idx, page_ref, self.location);
            self.page_infos.push(PageInfo::Pdf {
                ref_: page_ref,
                size,
                // TODO: Maybe this should be configurable.
                page_label: PageLabel::default(),
            });
        }
    }

    #[cfg(feature = "pdf")]
    pub(crate) fn embed_pdf_page_as_xobject(&mut self, pdf: &PdfDocument, page_idx: usize) -> Ref {
        let xobj_ref = self.new_ref();

        // Note that `add_xobject` might return a different ref than the one we created.
        self.global_objects
            .pdf_ctx
            .add_xobject(pdf, page_idx, xobj_ref, self.location)
    }

    pub(crate) fn page_tree_ref(&mut self) -> Ref {
        self.page_tree_ref
    }

    pub(crate) fn register_font_container(&mut self, font: Font) -> Rc<RefCell<FontContainer>> {
        self.global_objects
            .font_map
            .entry(font.clone())
            .or_insert_with(|| Rc::new(RefCell::new(FontContainer::new(font.clone()))))
            .clone()
    }

    pub(crate) fn validation_store(&mut self) -> &mut ValidationStore {
        &mut self.validation_store
    }

    pub(crate) fn finish(mut self, mut chunk_container: ChunkContainer) -> KrillaResult<Pdf> {
        // We need to be careful here that we serialize the objects in the right order,
        // as in some cases we use MaybeTake::take to remove an object, which means that
        // no object that is serialized afterwards must depend on it.

        // Serialize all objects that can only be written in the end.
        self.serialize_destination_profiles(&mut chunk_container);
        self.serialize_page_label_tree(&mut chunk_container);
        self.serialize_outline(&mut chunk_container);
        self.serialize_fonts(&mut chunk_container)?;
        self.serialize_pages(&mut chunk_container)?;
        self.serialize_page_tree(&mut chunk_container);
        #[cfg(feature = "pdf")]
        self.serialize_embedded_pdfs(&mut chunk_container)?;
        self.serialize_xyz_destinations(&mut chunk_container)?;
        // It is important that we serialize the tags AFTER we have serialized the pages,
        // because page serialization will update the annotation refs of the page infos,
        // and when serializing the parent tree map we need to know the refs of the annotations
        self.serialize_tag_tree(&mut chunk_container)?;

        // Create the final PDF.
        let pdf = chunk_container.finish(&mut self)?;
        self.register_limits(pdf.limits());

        self.check_validator_limits();

        if !self.validation_errors.is_empty() {
            // Deduplicate errors, while still preserving order.
            let mut errors = vec![];
            let mut seen = HashSet::new();

            for error in self.validation_errors {
                if !seen.contains(&error) {
                    seen.insert(error.clone());
                    errors.push(error);
                }
            }

            return Err(KrillaError::Validation(errors));
        }

        if let Some(limit_error) = self.check_version_limits() {
            return Err(KrillaError::Limit(limit_error));
        }

        // Just a sanity check that we've actually processed all items.
        self.global_objects.assert_all_taken();

        Ok(pdf)
    }
}

/// Various registration methods.
impl SerializeContext {
    pub(crate) fn register_validation_error(&mut self, error: ValidationError) {
        if let Some(validators) = self.serialize_settings().validators().prohibits(&error) {
            self.validation_errors.push((error, validators))
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

    /// Register the struct parent integer in the parent tree.
    /// The annotation parent must be later set using [`Self::set_annotation_parent`].
    pub(crate) fn register_annotation_parent(&mut self, ai: AnnotationIdentifier) -> Option<i32> {
        if self.serialize_settings.enable_tagging {
            let id = self.global_objects.struct_parents.len();
            self.global_objects
                .struct_parents
                .push(StructParentElement::Annotation(ai));
            Some(i32::try_from(id).unwrap())
        } else {
            None
        }
    }

    pub(crate) fn register_named_destination(&mut self, nd: NamedDestination) -> Option<Ref> {
        if let Some((dest_ref, existing)) =
            self.global_objects.named_destinations.get(nd.name.as_ref())
        {
            return (existing == nd.xyz_dest.as_ref()).then_some(*dest_ref);
        }

        let dest_ref = self.register_xyz_destination((*nd.xyz_dest).clone());
        self.global_objects
            .named_destinations
            .insert(nd.name.clone(), (dest_ref, (*nd.xyz_dest).clone()));
        Some(dest_ref)
    }

    pub(crate) fn register_page(&mut self, page: InternalPage) {
        let ref_ = self.new_ref();
        self.page_infos.push(PageInfo::Krilla {
            ref_,
            surface_size: page.page_settings.surface_size(),
            // Will be populated when the page is serialized.
            annotations: vec![],
            page_label: page.page_settings.page_label().clone(),
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

    pub(crate) fn register_cacheable<T>(
        &mut self,
        chunk_container: &mut ChunkContainer,
        object: T,
    ) -> Ref
    where
        T: Cacheable,
    {
        self.register_cached(object, |sc, object, root_ref| {
            object.serialize(sc, chunk_container, root_ref);
        })
    }

    pub(crate) fn register_resourceable<T>(
        &mut self,
        chunk_container: &mut ChunkContainer,
        object: T,
    ) -> T::Resource
    where
        T: Resourceable,
    {
        Resource::new(self.register_cacheable(chunk_container, object))
    }

    #[cfg(feature = "raster-images")]
    pub(crate) fn register_image(
        &mut self,
        chunk_container: &mut ChunkContainer,
        image: Image,
    ) -> Ref {
        self.register_cached(image, |sc, object, root_ref| {
            object.serialize(sc, chunk_container, root_ref);
        })
    }

    pub(crate) fn register_xyz_destination(&mut self, dest: XyzDestination) -> Ref {
        self.register_cached(dest, |sc, dest, root_ref| {
            sc.global_objects.xyz_destinations.push((root_ref, dest));
        })
    }

    pub(crate) fn register_page_label(
        &mut self,
        chunk_container: &mut ChunkContainer,
        page_label: PageLabel,
    ) -> Ref {
        let ref_ = self.new_ref();
        page_label.serialize(chunk_container, ref_);
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

    pub(crate) fn register_colorspace(
        &mut self,
        chunk_container: &mut ChunkContainer,
        cs: ColorSpace,
    ) -> MaybeDeviceColorSpace {
        match cs {
            ColorSpace::CieBased(CieBasedColorSpace::Srgb) => {
                MaybeDeviceColorSpace::ColorSpace(self.register_resourceable(
                    chunk_container,
                    ICCBasedColorSpace(self.serialize_settings.pdf_version().rgb_icc()),
                ))
            }
            ColorSpace::CieBased(CieBasedColorSpace::Luma) => {
                MaybeDeviceColorSpace::ColorSpace(self.register_resourceable(
                    chunk_container,
                    ICCBasedColorSpace(self.serialize_settings.pdf_version().grey_icc()),
                ))
            }
            ColorSpace::CieBased(CieBasedColorSpace::Cmyk(cs)) => {
                MaybeDeviceColorSpace::ColorSpace(self.register_resourceable(chunk_container, cs))
            }
            ColorSpace::Device(DeviceColorSpace::Gray) => MaybeDeviceColorSpace::DeviceGray,
            ColorSpace::Device(DeviceColorSpace::Rgb) => MaybeDeviceColorSpace::DeviceRgb,
            ColorSpace::Device(DeviceColorSpace::Cmyk) => MaybeDeviceColorSpace::DeviceCMYK,
            ColorSpace::Special(SpecialColorSpace::Separation(s)) => {
                MaybeDeviceColorSpace::ColorSpace(
                    self.register_resourceable(chunk_container, SeparationColorSpace::new(s)),
                )
            }
        }
    }
}

/// Various serialization methods.
/// All methods are supposed to only be called once in `SerializeContext::finish`!
impl SerializeContext {
    fn serialize_destination_profiles(&mut self, chunk_container: &mut ChunkContainer) {
        let validators = self.serialize_settings.validators();
        let subtypes = validators.output_intents();

        if subtypes.is_empty() {
            return;
        }

        let root_ref = self.new_ref();
        let mut chunk = self.new_chunk();
        let mut oi_refs = Vec::new();

        for subtype in subtypes {
            let oi_ref = self.new_ref();

            // PDF/X-4p and PDF/X-6p reference the output profile externally
            // instead of embedding it.
            if validators.requires_external_output_profile() && subtype == OutputIntentSubtype::PDFX
            {
                let Some(external_profile) =
                    self.serialize_settings.external_output_profile.clone()
                else {
                    self.register_validation_error(ValidationError::MissingExternalOutputProfile);
                    continue;
                };

                // `ExternalOutputProfile` guarantees non-empty URLs / identifier / info
                // at construction time, so no runtime validation is needed here.
                let metadata = external_profile.profile().metadata();
                // Annex A.1 → §6.4.2.1: the referenced profile must characterize
                // an output device (Device Class `prtr`) and use an admissible
                // ICC version (v2, or v4 up to v4.2). Its colour space is
                // constrained to GRAY/RGB/CMYK by the `ExternalOutputProfile`
                // constructors.
                if !metadata.is_output_rendering_device() {
                    self.register_validation_error(
                        ValidationError::InvalidOutputProfileDeviceClass(None),
                    );
                }
                if output_profile_version_too_new(
                    self.serialize_settings.pdf_version(),
                    metadata.major,
                    metadata.minor,
                ) {
                    self.register_validation_error(
                        ValidationError::IncompatibleOutputProfileVersion(None),
                    );
                }
                let mut dict = chunk.indirect(oi_ref).dict();
                dict.pair(Name(b"Type"), Name(b"OutputIntent"));
                dict.pair(Name(b"S"), Name(b"GTS_PDFX"));
                dict.pair(
                    Name(b"OutputConditionIdentifier"),
                    TextStr(external_profile.output_condition_identifier()),
                );
                if let Some(output_condition) = external_profile.output_condition() {
                    dict.pair(Name(b"OutputCondition"), TextStr(output_condition));
                }
                if let Some(registry_name) = external_profile.registry_name() {
                    dict.pair(Name(b"RegistryName"), TextStr(registry_name));
                }
                dict.pair(Name(b"Info"), TextStr(external_profile.info()));

                {
                    let mut profile_ref = dict.insert(Name(b"DestOutputProfileRef")).dict();
                    profile_ref.pair(Name(b"CheckSum"), Str(&metadata.checksum));
                    profile_ref.pair(Name(b"ICCVersion"), Str(&metadata.version_bytes));
                    profile_ref.pair(Name(b"ProfileCS"), Str(&metadata.color_space_signature));
                    // ProfileName is required; fall back to the always-present
                    // output-condition info when the profile carries no parseable
                    // description tag.
                    let profile_name = metadata
                        .profile_name
                        .as_deref()
                        .unwrap_or_else(|| external_profile.info());
                    profile_ref.pair(Name(b"ProfileName"), TextStr(profile_name));

                    let mut urls = profile_ref.insert(Name(b"URLs")).array();
                    for url in external_profile.urls() {
                        let mut file_spec = urls.push().start::<FileSpec>();
                        file_spec
                            .file_system(Name(b"URL"))
                            .path(Str(url.as_bytes()));
                    }
                }

                dict.finish();
                oi_refs.push(oi_ref);
                continue;
            }

            let cmyk_desc = if validators.uses_cmyk_output_profile_for_subtype(subtype) {
                match self.serialize_settings.cmyk_profile.clone() {
                    Some(profile) => {
                        // The output-intent profile's ICC version must not exceed
                        // what the target PDF version admits (v2 for PDF 1.4, v4.2
                        // for PDF 1.6, v4.3 for PDF 2.0 — see
                        // `output_profile_version_too_new`). The output intent is
                        // mandatory, so a too-new version is an error (unlike an
                        // image profile, which is simply dropped).
                        let m = profile.metadata();
                        if output_profile_version_too_new(
                            self.serialize_settings.pdf_version(),
                            m.major,
                            m.minor,
                        ) {
                            self.register_validation_error(
                                ValidationError::IncompatibleOutputProfileVersion(None),
                            );
                        }
                        // A PDF/X output intent must characterize an output device
                        // (Device Class `prtr`).
                        if !m.is_output_rendering_device() {
                            self.register_validation_error(
                                ValidationError::InvalidOutputProfileDeviceClass(None),
                            );
                        }
                        // ISO 15930-7 §6.4.1 / ISO 15930-9 §6.6.1: the
                        // characterized printing condition must have a
                        // GRAY/RGB/CMYK data colour space. The embedded
                        // `cmyk_profile` is the CMYK output target, so a
                        // four-channel but non-`'CMYK'` profile (e.g. `'4CLR'`
                        // DeviceN) is not acceptable. The external (-p) path
                        // performs the equivalent check at construction time.
                        if m.color_space != ICCColorSpace::Cmyk {
                            self.register_validation_error(
                                ValidationError::InvalidOutputProfileColorSpace(None),
                            );
                        }
                        let major = m.major;
                        let minor = m.minor;
                        let profile_ref = self.register_cacheable(chunk_container, profile);
                        Some((profile_ref, major, minor))
                    }
                    None => {
                        // PDF/X requires a CMYK output intent profile. Fall back
                        // to sRGB so we still produce a structurally valid PDF
                        // while registering the validation error.
                        self.register_validation_error(ValidationError::MissingCMYKProfile);
                        None
                    }
                }
            } else {
                None
            };

            let mut oi = chunk.indirect(oi_ref).start::<OutputIntent>();
            if let Some((profile_ref, major, minor)) = cmyk_desc {
                oi.dest_output_profile(profile_ref)
                    .subtype(subtype)
                    // No RegistryName: ISO 15930-7 §6.4.2.1 requires that key
                    // only when the printing condition is registry-defined, which
                    // an embedded (unregistered "Custom") profile is not.
                    .output_condition_identifier(TextStr("Custom"))
                    .output_condition(TextStr("CMYK"))
                    .info(TextStr(format!("CMYK v{major}.{minor}").as_str()));
            } else {
                // sRGB output intent: PDF/A, or the fallback when a CMYK profile
                // was required but not supplied.
                let icc_profile = self.serialize_settings.pdf_version().rgb_icc();
                let major = icc_profile.metadata().major;
                let minor = icc_profile.metadata().minor;
                let profile_ref = self.register_cacheable(chunk_container, icc_profile);
                oi.dest_output_profile(profile_ref)
                    .subtype(subtype)
                    .output_condition_identifier(TextStr("Custom"))
                    .output_condition(TextStr("sRGB"))
                    .info(TextStr(format!("sRGB v{major}.{minor}").as_str()));
            }

            oi.finish();
            oi_refs.push(oi_ref);
        }

        if oi_refs.is_empty() {
            return;
        }

        let mut array = chunk.indirect(root_ref).array();
        for oi_ref in oi_refs {
            array.item(oi_ref);
        }
        array.finish();

        chunk_container.non_stream.destination_profiles = Some((root_ref, chunk));
    }

    fn serialize_page_label_tree(&mut self, chunk_container: &mut ChunkContainer) {
        if let Some(container) = PageLabelContainer::new(
            &self
                .page_infos
                .iter()
                .map(|page| page.page_label().clone())
                .collect::<Vec<_>>(),
        ) {
            let page_label_tree_ref = self.new_ref();
            container.serialize(self, chunk_container, page_label_tree_ref);
        }
    }

    fn serialize_outline(&mut self, chunk_container: &mut ChunkContainer) {
        let outline = self.global_objects.outline.take();
        if let Some(outline) = &outline {
            let outline_ref = self.new_ref();
            outline.serialize(self, chunk_container, outline_ref);
        } else {
            self.register_validation_error(ValidationError::MissingDocumentOutline);
        }
    }

    #[cfg(feature = "pdf")]
    fn serialize_embedded_pdfs(
        &mut self,
        chunk_container: &mut ChunkContainer,
    ) -> KrillaResult<()> {
        let pdf_ctx = self.global_objects.pdf_ctx.take();

        pdf_ctx.serialize(self, chunk_container)
    }

    fn serialize_fonts(&mut self, chunk_container: &mut ChunkContainer) -> KrillaResult<()> {
        let fonts = self.global_objects.font_map.take();
        for font_container in fonts.values() {
            let borrowed = font_container.borrow();

            if !borrowed.type3_mapper().is_empty() {
                for t3_font in borrowed.type3_mapper().fonts() {
                    let f = self.register_font_identifier(t3_font.identifier());
                    t3_font.serialize(self, chunk_container, f.get_ref());
                }
            }

            if !borrowed.cid_font().is_empty() {
                let f = self.register_font_identifier(borrowed.cid_font().identifier());
                borrowed
                    .cid_font()
                    .serialize(self, chunk_container, f.get_ref())?;
            }
        }

        Ok(())
    }

    fn serialize_pages(&mut self, chunk_container: &mut ChunkContainer) -> KrillaResult<()> {
        let pages = self.global_objects.pages.take();
        for (ref_, page) in pages {
            page.serialize(self, chunk_container, ref_)?;
        }

        Ok(())
    }

    fn serialize_page_tree(&mut self, chunk_container: &mut ChunkContainer) {
        let mut page_tree_chunk = self.new_chunk();
        page_tree_chunk
            .pages(self.page_tree_ref)
            .count(self.page_infos.len() as i32)
            .kids(self.page_infos.iter().map(|i| i.ref_()));
        chunk_container.non_stream.page_tree = Some((self.page_tree_ref, page_tree_chunk));
    }

    fn serialize_xyz_destinations(
        &mut self,
        chunk_container: &mut ChunkContainer,
    ) -> KrillaResult<()> {
        let xyz_destinations = self.global_objects.xyz_destinations.take();
        for (ref_, dest) in &xyz_destinations {
            dest.serialize(self, chunk_container, *ref_);
        }

        Ok(())
    }

    fn serialize_tag_tree(&mut self, chunk_container: &mut ChunkContainer) -> KrillaResult<()> {
        let tag_tree = self.global_objects.tag_tree.take();
        let struct_parents = self.global_objects.struct_parents.take();
        if let Some(root) = &tag_tree {
            let mut parent_tree_map = HashMap::new();
            let mut id_tree_map = BTreeMap::new();
            let struct_tree_root_ref = self.new_ref();
            let document_ref = root.serialize(
                self,
                chunk_container,
                &mut parent_tree_map,
                &mut id_tree_map,
                struct_tree_root_ref,
            )?;

            root.validate(&id_tree_map)?;

            let mut chunk = self.new_chunk();
            let mut tree = chunk
                .indirect(struct_tree_root_ref)
                .start::<StructTreeRoot>();

            let mut sub_chunks = vec![];

            if self.serialize_settings.pdf_version() < PdfVersion::Pdf20 {
                let mut role_map = tree.role_map();
                // Custom structure elements.
                role_map.insert(Name(b"Datetime"), StructRole::Span);
                role_map.insert(Name(b"Terms"), StructRole::Part);

                // PDF 2.0 exclusive structure elements.
                role_map.insert(Name(b"Title"), StructRole::P);
                role_map.insert(Name(b"Strong"), StructRole::Span);
                role_map.insert(Name(b"Em"), StructRole::Span);
                for level in self.global_objects.custom_heading_roles.iter() {
                    let role2 = StructRole2::Heading(*level);
                    role_map.insert(role2.to_name(&mut [0; 6]), StructRole::P);
                }
            } else {
                let mut namespaces = tree.namespaces();

                // PDF 2.0 standard structure namespace
                namespaces.item(self.pdf2_ns.ssn_ref);
                let mut ns_chunk = self.new_chunk();
                ns_chunk.namespace(self.pdf2_ns.ssn_ref).pdf_2_ns();
                sub_chunks.push(ns_chunk);

                // Custom krilla namspace
                namespaces.item(self.pdf2_ns.krilla_ref);
                let mut ns_chunk = self.new_chunk();
                let mut ns = ns_chunk.namespace(self.pdf2_ns.krilla_ref);
                ns.ns(TextStr("https://github.com/LaurenzV/krilla"));

                // Custom structure elements.
                ns.role_map_ns()
                    .to_pdf_2_0(Name(b"Datetime"), StructRole2::Span, self.pdf2_ns.ssn_ref)
                    .to_pdf_2_0(Name(b"Terms"), StructRole2::Part, self.pdf2_ns.ssn_ref);

                ns.finish();
                sub_chunks.push(ns_chunk);
            }
            tree.children().item(document_ref);

            if !struct_parents.is_empty() {
                let mut parent_tree = tree.parent_tree();
                let mut tree_nums = parent_tree.nums();

                for (index, struct_parent) in struct_parents.iter().enumerate() {
                    match *struct_parent {
                        StructParentElement::Page(page_index, num_mcids) => {
                            let mut list_chunk = self.new_chunk();
                            let list_ref = self.new_ref();

                            let mut refs = list_chunk.indirect(list_ref).array();

                            for mcid in 0..num_mcids {
                                let rci = PageTagIdentifier::new(page_index, mcid);
                                refs.item(parent_tree_map.get(&rci.into()).unwrap_or_else(|| {
                                    panic!(
                                        "page tag identifier {rci:?} doesn't appear in the tag tree"
                                    )
                                }));
                            }

                            refs.finish();

                            sub_chunks.push(list_chunk);
                            tree_nums.insert(index as i32, list_ref);
                        }
                        StructParentElement::Annotation(ai) => {
                            // Write a reference to the parent structure element.
                            // From the PDF 1.7 spec (14.7.5.4 Finding structure elements from content items):
                            // > For an object identified as a content item by means of an object reference
                            // > (see 14.7.5.3, "PDF objects as content items"), the value shall be an
                            // > indirect reference to the parent structure element.
                            let page_annotations = &self.page_infos[ai.page_index].annotations();
                            let parent_ref =
                                *page_annotations[ai.annot_index].1.get().unwrap_or_else(|| {
                                    panic!("annotation identifier {ai:?} doesn't appear in the tag tree")
                                });
                            tree_nums.insert(index as i32, parent_ref);
                        }
                    }
                }

                tree_nums.finish();
                parent_tree.finish();
            }

            if !id_tree_map.is_empty() {
                let mut id_tree = tree.id_tree();
                let mut names = id_tree.names();

                for (name, ref_) in id_tree_map {
                    names.insert(Str(name.as_bytes()), ref_);
                }
            }

            if !struct_parents.is_empty() {
                tree.parent_tree_next_key(struct_parents.len() as i32);
            }
            tree.finish();

            for sub_chunk in sub_chunks {
                chunk.extend(&sub_chunk);
            }

            chunk_container.non_stream.struct_tree_root = Some((struct_tree_root_ref, chunk));
        } else {
            self.register_validation_error(ValidationError::MissingTagging);
        }

        Ok(())
    }

    fn check_validator_limits(&mut self) {
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

    fn check_version_limits(&self) -> Option<LimitError> {
        if self.serialize_settings.pdf_version() != PdfVersion::Pdf14 {
            return None;
        }

        if self.limits.real() > MAX_FLOAT {
            return Some(LimitError::TooLargeFloat);
        }

        if self.limits.array_len() > ARRAY_LEN {
            return Some(LimitError::TooLongArray);
        }

        if self.limits.dict_entries() > DICT_LEN {
            return Some(LimitError::TooLongDictionary);
        }

        None
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

pub(crate) struct Pdf2Namespaces {
    /// The ref of the PDF 2.0 standard structure namspace (`https://www.iso.org/pdf2/ssn`).
    pub(crate) ssn_ref: Ref,
    /// The ref of the custom krilla namespace used for role mapping.
    pub(crate) krilla_ref: Ref,
}

#[derive(Default)]
pub(crate) struct GlobalObjects {
    /// All named destinations that have been registered, including a Ref to their destination and
    /// the destination itself.
    // Needs to be pub(crate) because writing of named destinations happens in `ChunkContainer`.
    pub(crate) named_destinations: MaybeTaken<HashMap<Arc<String>, (Ref, XyzDestination)>>,
    /// A map from fonts to font container.
    font_map: MaybeTaken<IndexMap<Font, Rc<RefCell<FontContainer>>>>,
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
    /// A list of custom headings numbers used in the document.
    pub(crate) custom_heading_roles: BTreeSet<NonZeroU16>,
    /// The context tracking all of the pdfs and their pages that have been inserted.
    #[cfg(feature = "pdf")]
    pub(crate) pdf_ctx: MaybeTaken<PdfSerializerContext>,
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
        #[cfg(feature = "pdf")]
        assert!(self.pdf_ctx.is_taken());
    }
}

pub(crate) trait Cacheable: SipHashable {
    fn serialize(
        self,
        sc: &mut SerializeContext,
        chunk_container: &mut ChunkContainer,
        root_ref: Ref,
    );
}
