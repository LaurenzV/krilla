//! Setting document metadata.
//!
//! PDF allows for the inclusion of metadata in a PDF document. To do so in krilla,
//! create a [`Metadata`] object, set the data, and then include it
//! in the document via [`Document::set_metadata`].
//!
//! [`Document::set_metadata`]: crate::document::Document::set_metadata
pub mod xmp;

use pdf_writer::{Finish, Name, Pdf, Ref, TextStr};
use std::borrow::Cow;
use std::cell::LazyCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use xmp_writer::{
    CustomNamespace, LangId, Namespace as XmpNamespace, RdfCollectionType, Timezone, XmpWriter,
};

use crate::configure::{Archival, Configuration, PdfVersion, ValidationError, Validators};
use crate::serialize::SerializeContext;

use self::xmp::{
    Category, Namespace, Property, PropertyDescription, Schema, SimpleType, StructType, Value,
    ValueType, XmpError,
};

/// Metadata for a PDF document.
#[derive(Default, Clone, Debug)]
pub struct Metadata {
    pub(crate) title: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) creator: Option<String>,
    pub(crate) producer: Option<String>,
    pub(crate) keywords: Option<Vec<String>>,
    pub(crate) authors: Option<Vec<String>>,
    pub(crate) document_id: Option<String>,
    pub(crate) version_id: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) creation_date: Option<DateTime>,
    pub(crate) text_direction: Option<TextDirection>,
    pub(crate) page_layout: Option<PageLayout>,
    pub(crate) custom_xmp_properties: Vec<Property>,
    pub(crate) custom_xmp_schemas: Vec<Schema>,
    pub(crate) custom_fields: BTreeMap<String, String>,
}

impl Metadata {
    /// Create new metadata.
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// The title of the document.
    pub fn title(mut self, title: String) -> Self {
        if !title.is_empty() {
            self.title = Some(title);
        }
        self
    }

    /// The description of the document.
    ///
    /// This should be a short, human-readable abstract, summary, or description
    /// of the topic of the document.
    pub fn description(mut self, description: String) -> Self {
        if !description.is_empty() {
            self.description = Some(description);
        }
        self
    }

    /// The keywords that describe the document.
    pub fn keywords(mut self, keywords: Vec<String>) -> Self {
        if !keywords.is_empty() {
            self.keywords = Some(keywords);
        }
        self
    }

    /// The main language of the document, as an RFC 3066 language tag.
    ///
    /// This property is required for some export modes, like for example PDF/A-3a.
    pub fn language(mut self, language: String) -> Self {
        self.language = Some(language);
        self
    }

    /// The creator tool of the document.
    pub fn creator(mut self, creator: String) -> Self {
        if !creator.is_empty() {
            self.creator = Some(creator);
        }
        self
    }

    /// The producer tool of the document.
    pub fn producer(mut self, producer: String) -> Self {
        if !producer.is_empty() {
            self.producer = Some(producer);
        }
        self
    }

    /// The authors of the document.
    pub fn authors(mut self, authors: Vec<String>) -> Self {
        if !authors.is_empty() {
            self.authors = Some(authors);
        }
        self
    }

    /// The creation date of the document.
    pub fn creation_date(mut self, creation_date: DateTime) -> Self {
        self.creation_date = Some(creation_date);
        self
    }

    /// A document ID.
    ///
    /// This attribute will be used as an identifier for identifying
    /// different versions of the same document.
    pub fn document_id(mut self, document_id: String) -> Self {
        self.document_id = Some(document_id);
        self
    }

    /// A version ID.
    ///
    /// This attribute identifies a specific version of the document.
    pub fn version_id(mut self, version_id: String) -> Self {
        self.version_id = Some(version_id);
        self
    }

    /// The main text direction of the document.
    pub fn text_direction(mut self, text_direction: TextDirection) -> Self {
        self.text_direction = Some(text_direction);
        self
    }

    /// How the viewer should lay out the pages.
    pub fn page_layout(mut self, page_layout: PageLayout) -> Self {
        self.page_layout = Some(page_layout);
        self
    }

    /// Add a custom XMP property.
    pub fn custom_xmp_property(mut self, property: Property) -> Self {
        self.custom_xmp_properties.push(property);
        self
    }

    /// Add a schema describing the properties of a custom XMP namespace.
    pub fn custom_xmp_schema(mut self, schema: Schema) -> Self {
        self.custom_xmp_schemas.push(schema);
        self
    }

    /// Add a custom field to the document information dictionary.
    ///
    /// If a field with the same name was already added, its value will be
    /// replaced. Returns `None` for empty names and names reserved for standard
    /// document information fields.
    ///
    /// **WARNING**: Custom fields will currently be ignored when exporting to
    /// PDF/A-2 and PDF/A-3 and PDF/A-4. This is a temporary limitation that will be lifted
    /// in the future.
    #[must_use]
    pub fn custom_field(mut self, name: String, value: String) -> Option<Self> {
        if name.is_empty() || is_standard_document_info_field(&name) {
            return None;
        }

        self.custom_fields.insert(name, value);
        Some(self)
    }

    pub(crate) fn has_document_info(&self) -> bool {
        self.title.is_some()
            || self.producer.is_some()
            || self.keywords.is_some()
            || self.authors.is_some()
            || self.creator.is_some()
            || self.creation_date.is_some()
            || self.description.is_some()
            || !self.custom_fields.is_empty()
    }

    pub(crate) fn serialize_xmp_metadata<'n>(
        &'n self,
        xmp: &mut XmpWriter<'n>,
        sc: &mut SerializeContext,
        instance_id: &str,
    ) -> Result<(), XmpError> {
        if let Some(title) = &self.title {
            xmp.title([(None, title.as_str())]);
        }

        if let Some(description) = &self.description {
            xmp.description([(None, description.as_str())]);
        }

        if let Some(keywords) = &self.keywords {
            let joined = keywords.join(", ");
            xmp.pdf_keywords(joined.as_str());
        }

        match &self.authors {
            Some(authors) if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf20 => {
                // PDF 2.0+ deprecates the document information dictionary, so
                // we can use the array here.
                xmp.creator(authors.iter().map(String::as_str));
            }
            Some(authors) => {
                // Turns out that if the authors are given in both the document
                // information dictionary and the XMP metadata, Acrobat takes a
                // little bit of both: The first author from the document
                // information dictionary and the remaining authors from the XMP
                // metadata.
                //
                // To fix this for Acrobat, we could omit the remaining authors
                // or all metadata from the document information catalog (it is
                // optional) and only write XMP. However, not all other tools
                // (including Apple Preview) read the XMP data. This means we do
                // want to include all authors in the document information
                // dictionary.
                //
                // Thus, the only alternative is to fold all authors into a
                // single `<rdf:li>` in the XMP metadata. This is, in fact,
                // exactly what the PDF/A spec Part 1 section 6.7.3 has to say
                // about the matter. It's a bit weird to not use the array (and
                // it makes Acrobat show the author list in quotes), but there's
                // not much we can do about that.
                let joined = authors.join(", ");
                xmp.creator([joined.as_str()]);
            }
            None => {}
        }

        if let Some(creator) = &self.creator {
            xmp.creator_tool(creator);
        }

        if let Some(producer) = &self.producer {
            xmp.producer(producer);
        }

        if let Some(version_id) = &self.version_id {
            xmp.version_id(version_id);
        }

        if let Some(lang) = &self.language {
            xmp.language([LangId(lang)]);
        }

        if let Some(date) = self.creation_date.map(xmp_date) {
            xmp.modify_date(date);
            xmp.create_date(date);

            if sc
                .serialize_settings()
                .validators()
                .requires_file_provenance_information()
            {
                let mut history = xmp.history();
                let mut saved = history.add_event();

                saved
                    .action(xmp_writer::ResourceEventAction::Saved)
                    .when(date);

                if !sc
                    .serialize_settings()
                    .validators()
                    .prohibits_instance_id_in_xmp_metadata()
                {
                    saved.instance_id(&format!("{instance_id}_source"));
                }

                saved.finish();

                let mut converted = history.add_event();

                converted
                    .action(xmp_writer::ResourceEventAction::Converted)
                    .when(date);

                if let Some(creator) = &self.creator {
                    converted.software_agent(creator);
                }

                if !sc
                    .serialize_settings()
                    .validators()
                    .prohibits_instance_id_in_xmp_metadata()
                {
                    converted.instance_id(&format!("{instance_id}_source"));
                }
            }
        } else {
            sc.register_validation_error(ValidationError::MissingDocumentDate);
        }

        self.serialize_custom_xmp(xmp, sc)
    }

    fn serialize_custom_xmp<'n>(
        &'n self,
        xmp: &mut XmpWriter<'n>,
        sc: &mut SerializeContext,
    ) -> Result<(), XmpError> {
        let mut namespaces = Namespaces::default();
        let mut schemas = HashMap::new();
        for schema in &self.custom_xmp_schemas {
            namespaces.register_schema(schema)?;
            if schemas.insert(schema.namespace.uri(), schema).is_some() {
                return Err(XmpError::DuplicateSchema(
                    schema.namespace.uri().to_string(),
                ));
            }
        }

        let validators = sc.serialize_settings().validators();
        let check_descriptions = validators.requires_xmp_metadata_extension_schema();
        let xmp_2005 = validators.uses_xmp_2005_predefined_schemas();

        let mut written = HashSet::new();
        for property in &self.custom_xmp_properties {
            let uri = property.namespace.uri();
            if let Some(ns) = managed_namespace(uri, &property.name) {
                return Err(XmpError::ManagedProperty(format!(
                    "{}:{}",
                    ns.prefix(),
                    property.name
                )));
            }

            let description = schemas.get(uri).and_then(|s| s.get(&property.name));
            if check_descriptions
                && description.is_none()
                && !is_predefined_pdfa_schema(uri, xmp_2005)
            {
                sc.register_validation_error(ValidationError::MissingXmpPropertyDescription {
                    namespace_uri: uri.to_string(),
                    property_name: property.name.clone(),
                });
            }

            let mut writer = ValueWriter {
                namespaces: &mut namespaces,
                property: &property.name,
            };
            let namespace = writer.namespace(&property.namespace)?;
            writer.write_property(
                xmp.element(&property.name, namespace),
                &property.value,
                description.map(|desc| &desc.value_type),
            )?;
            written.insert((uri, property.name.as_str()));
        }

        for schema in &self.custom_xmp_schemas {
            check_required(&schema.properties, |name| {
                written.contains(&(schema.namespace.uri(), name))
            })?;
        }

        Ok(())
    }

    /// The attached schemas that need a PDF/A extension schema.
    pub(crate) fn extension_schemas(&self, validators: Validators) -> Vec<&Schema> {
        let xmp_2005 = validators.uses_xmp_2005_predefined_schemas();
        self.custom_xmp_schemas
            .iter()
            .filter(|schema| !is_predefined_pdfa_schema(schema.namespace.uri(), xmp_2005))
            .collect()
    }

    pub(crate) fn serialize_document_info(
        &self,
        ref_: &mut Ref,
        pdf: &mut Pdf,
        config: Configuration,
    ) {
        if config.validators().prohibits_info_dict() {
            return;
        }

        if self.has_document_info() {
            let ref_ = ref_.bump();
            let mut document_info = LazyCell::new(|| pdf.document_info(ref_));

            // ALl of those are deprecated in PDF 2.0 and will only be written
            // to the XMP metadata.
            if config.version() < PdfVersion::Pdf20 {
                if let Some(title) = &self.title {
                    document_info.title(TextStr(title));
                }

                if let Some(description) = &self.description {
                    document_info.subject(TextStr(description));
                }

                if let Some(keywords) = &self.keywords {
                    let joined = keywords.join(", ");
                    document_info.keywords(TextStr(&joined));
                }

                if let Some(authors) = &self.authors {
                    let joined = authors.join(", ");
                    document_info.author(TextStr(&joined));
                }

                if let Some(creator) = &self.creator {
                    document_info.creator(TextStr(creator));
                }

                if let Some(producer) = &self.producer {
                    document_info.producer(TextStr(producer));
                }
            }

            if let Some(date_time) = self.creation_date {
                document_info.modified_date(pdf_date(date_time));
                document_info.creation_date(pdf_date(date_time));
            }

            // This is temporary until custom fields can be written to XMP.
            // See https://github.com/LaurenzV/krilla/pull/419#issuecomment-5142992957.
            if !matches!(
                config.validators().archival(),
                Some(
                    Archival::A2_A
                        | Archival::A2_B
                        | Archival::A2_U
                        | Archival::A3_A
                        | Archival::A3_B
                        | Archival::A3_U
                )
            ) {
                for (name, value) in &self.custom_fields {
                    document_info.pair(Name(name.as_bytes()), TextStr(value));
                }
            }
        }
    }
}

fn is_standard_document_info_field(name: &str) -> bool {
    matches!(
        name,
        "Title"
            | "Author"
            | "Subject"
            | "Keywords"
            | "Creator"
            | "Producer"
            | "CreationDate"
            | "ModDate"
            | "Trapped"
    )
}

/// A datetime. Invalid values will be clamped.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct DateTime {
    /// The year (0-9999).
    pub(crate) year: u16,
    /// The month (0-11).
    pub(crate) month: Option<u8>,
    /// The day (0-31).
    pub(crate) day: Option<u8>,
    /// The hour (0-23).
    pub(crate) hour: Option<u8>,
    /// The minute (0-59).
    pub(crate) minute: Option<u8>,
    /// The second (0-59).
    pub(crate) second: Option<u8>,
    /// The hour offset from UTC (-23 through 23).
    pub(crate) utc_offset_hour: Option<i8>,
    /// The minute offset from UTC (0-59). Will carry over the sign from
    /// `utc_offset_hour`.
    pub(crate) utc_offset_minute: u8,
}

impl DateTime {
    /// Create a new, minimal date. The year will be clamped within the range
    /// 0-9999.
    #[inline]
    pub fn new(year: u16) -> Self {
        Self {
            year: year.min(9999),
            month: None,
            day: None,
            hour: None,
            minute: None,
            second: None,
            utc_offset_hour: None,
            utc_offset_minute: 0,
        }
    }

    /// Add the month field. It will be clamped within the range 1-12.
    #[inline]
    pub fn month(mut self, month: u8) -> Self {
        self.month = Some(month.clamp(1, 12));
        self
    }

    /// Add the day field. It will be clamped within the range 1-31.
    #[inline]
    pub fn day(mut self, day: u8) -> Self {
        self.day = Some(day.clamp(1, 31));
        self
    }

    /// Add the hour field. It will be clamped within the range 0-23.
    #[inline]
    pub fn hour(mut self, hour: u8) -> Self {
        self.hour = Some(hour.min(23));
        self
    }

    /// Add the minute field. It will be clamped within the range 0-59.
    #[inline]
    pub fn minute(mut self, minute: u8) -> Self {
        self.minute = Some(minute.min(59));
        self
    }

    /// Add the second field. It will be clamped within the range 0-59.
    #[inline]
    pub fn second(mut self, second: u8) -> Self {
        self.second = Some(second.min(59));
        self
    }

    /// Add the offset from UTC in hours. If not specified, the time will be
    /// assumed to be local to the viewer's time zone. It will be clamped within
    /// the range -23-23.
    #[inline]
    pub fn utc_offset_hour(mut self, hour: i8) -> Self {
        self.utc_offset_hour = Some(hour.clamp(-23, 23));
        self
    }

    /// Add the offset from UTC in minutes. This will have the same sign as set in
    /// [`Self::utc_offset_hour`]. It will be clamped within the range 0-59.
    #[inline]
    pub fn utc_offset_minute(mut self, minute: u8) -> Self {
        self.utc_offset_minute = minute.min(59);
        self
    }
}

/// Converts a datetime to a pdf-writer date.
pub(crate) fn pdf_date(date_time: DateTime) -> pdf_writer::Date {
    // We always assume a full date with all fields because for some reason
    // Acrobat doesn't like PDF/A-1 files without everything set.
    pdf_writer::Date::new(date_time.year)
        .month(date_time.month.unwrap_or(1))
        .day(date_time.day.unwrap_or(1))
        .hour(date_time.hour.unwrap_or(0))
        .minute(date_time.minute.unwrap_or(0))
        .second(date_time.second.unwrap_or(0))
        .utc_offset_hour(date_time.utc_offset_hour.unwrap_or(0))
        .utc_offset_minute(date_time.utc_offset_minute)
}

/// Converts a datetime to an xmp-writer datetime.
fn xmp_date(datetime: DateTime) -> xmp_writer::DateTime {
    let timezone = match (datetime.utc_offset_hour, datetime.utc_offset_minute) {
        (Some(h), m) => Some(Timezone::Local {
            hour: h,
            minute: m as i8,
        }),
        _ => Some(Timezone::Utc),
    };

    // We always assume a full date with all fields because for some reason
    // Acrobat doesn't like PDF/A-1 files without everything set.
    xmp_writer::DateTime {
        year: datetime.year,
        month: Some(datetime.month.unwrap_or(1)),
        day: Some(datetime.day.unwrap_or(1)),
        hour: Some(datetime.hour.unwrap_or(0)),
        minute: Some(datetime.minute.unwrap_or(0)),
        second: Some(datetime.second.unwrap_or(0)),
        timezone,
    }
}

/// The main text direction of the document.
#[allow(missing_docs)]
#[derive(Copy, Clone, Debug)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

impl TextDirection {
    pub(crate) fn to_pdf(self) -> pdf_writer::types::Direction {
        match self {
            TextDirection::LeftToRight => pdf_writer::types::Direction::L2R,
            TextDirection::RightToLeft => pdf_writer::types::Direction::R2L,
        }
    }
}

/// How the viewer should lay out the pages.
#[derive(Copy, Clone, Debug)]
pub enum PageLayout {
    /// Only a single page at a time.
    SinglePage,
    /// A single, continuously scrolling column of pages.
    OneColumn,
    /// Two continuously scrolling columns of pages, laid out with odd-numbered
    /// pages on the left.
    TwoColumnLeft,
    /// Two continuously scrolling columns of pages, laid out with odd-numbered
    /// pages on the right (like in a left-bound book).
    TwoColumnRight,
    /// Only two pages are visible at a time, laid out with odd-numbered pages
    /// on the left. PDF 1.5+.
    TwoPageLeft,
    /// Only two pages are visible at a time, laid out with odd-numbered pages
    /// on the right (like in a left-bound book). PDF 1.5+.
    TwoPageRight,
}

impl PageLayout {
    pub(crate) fn to_pdf(self) -> pdf_writer::types::PageLayout {
        match self {
            PageLayout::SinglePage => pdf_writer::types::PageLayout::SinglePage,
            PageLayout::OneColumn => pdf_writer::types::PageLayout::OneColumn,
            PageLayout::TwoColumnLeft => pdf_writer::types::PageLayout::TwoColumnLeft,
            PageLayout::TwoColumnRight => pdf_writer::types::PageLayout::TwoColumnRight,
            PageLayout::TwoPageLeft => pdf_writer::types::PageLayout::TwoPageLeft,
            PageLayout::TwoPageRight => pdf_writer::types::PageLayout::TwoPageRight,
        }
    }
}

/// Returns the predefined xmp-writer namespace with the given URI, if any.
///
/// A user-supplied namespace with the URI of a namespace that xmp-writer
/// serializes natively must be mapped to the corresponding built-in variant,
/// since xmp-writer would otherwise declare the same namespace twice.
fn known_xmp_namespace(uri: &str) -> Option<XmpNamespace<'static>> {
    XmpNamespace::PREDEFINED
        .iter()
        .find(|ns| ns.url() == uri)
        .cloned()
}

fn is_reserved_xmp_prefix(prefix: &str) -> bool {
    XmpNamespace::PREDEFINED
        .iter()
        .any(|ns| ns.prefix() == prefix)
}

/// The properties krilla writes itself, either from the [`Metadata`] or
/// based on the configuration.
static MANAGED_PROPERTIES: &[(XmpNamespace<'static>, &[&str])] = &[
    (
        XmpNamespace::DublinCore,
        &["title", "description", "creator", "language", "format"],
    ),
    (
        XmpNamespace::Xmp,
        &["CreatorTool", "CreateDate", "ModifyDate"],
    ),
    (
        XmpNamespace::XmpMedia,
        &[
            "InstanceID",
            "DocumentID",
            "VersionID",
            "RenditionClass",
            "History",
        ],
    ),
    (XmpNamespace::XmpPaged, &["NPages"]),
    (
        XmpNamespace::AdobePdf,
        &["Keywords", "Producer", "PDFVersion"],
    ),
    (
        XmpNamespace::PdfAId,
        &["part", "conformance", "rev", "amd", "corr"],
    ),
    (XmpNamespace::PdfUAId, &["part", "rev", "amd", "corr"]),
    (XmpNamespace::PdfAExtension, &["schemas"]),
];

/// The namespace of the managed property `name` in `uri`, if any.
fn managed_namespace(uri: &str, name: &str) -> Option<&'static XmpNamespace<'static>> {
    MANAGED_PROPERTIES
        .iter()
        .find(|(ns, names)| ns.url() == uri && names.contains(&name))
        .map(|(ns, _)| ns)
}

/// Predefined XMP property schemas a custom property may use in PDF/A-1, -2
/// and -3 without a PDF/A extension schema description.
///
/// These are the standard schemas of the XMP 2004 specification that ISO
/// 19005-1 (PDF/A-1) treats as predefined. The PDF Association restates the
/// list in TechNote 0008, "Predefined XMP Properties in PDF/A-1":
/// <https://pdfa.org/wp-content/uploads/2011/08/tn0008_predefined_xmp_properties_in_pdfa-1_2008-03-20.pdf>
static PDF_A1_A2_A3_PREDEFINED_SCHEMAS: &[&str] = &[
    "http://www.aiim.org/pdfa/ns/id/",     // pdfaid
    "http://purl.org/dc/elements/1.1/",    // dc
    "http://ns.adobe.com/xap/1.0/",        // xmp
    "http://ns.adobe.com/xap/1.0/rights/", // xmpRights
    "http://ns.adobe.com/xap/1.0/mm/",     // xmpMM
    "http://ns.adobe.com/xap/1.0/bj/",     // xmpBJ
    "http://ns.adobe.com/xap/1.0/t/pg/",   // xmpTPg
    "http://ns.adobe.com/pdf/1.3/",        // pdf
    "http://ns.adobe.com/photoshop/1.0/",  // photoshop
    "http://ns.adobe.com/tiff/1.0/",       // tiff
    "http://ns.adobe.com/exif/1.0/",       // exif
];

/// Property schemas that PDF/A-2 and PDF/A-3 predefine in addition to
/// [`PDF_A1_A2_A3_PREDEFINED_SCHEMAS`], added by the XMP 2005 specification
/// over the XMP 2004 set.
///
/// The three schemas are defined by the Adobe XMP Specification, June 2005:
/// Dynamic Media (`xmpDM`), Camera Raw (`crs`) and EXIF auxiliary (`aux`).
/// They are the schemas present in that edition but absent
/// from XMP 2004.
static PDF_A2_A3_ADDITIONAL_PREDEFINED_SCHEMAS: &[&str] = &[
    "http://ns.adobe.com/xmp/1.0/DynamicMedia/",    // xmpDM
    "http://ns.adobe.com/camera-raw-settings/1.0/", // crs
    "http://ns.adobe.com/exif/1.0/aux/",            // aux
];

/// Whether `uri` is a predefined XMP schema that PDF/A exempts from an extension
/// schema description. `xmp_2005` selects the larger PDF/A-2/-3 set over the
/// PDF/A-1 set; see [`Validators::uses_xmp_2005_predefined_schemas`].
fn is_predefined_pdfa_schema(uri: &str, xmp_2005: bool) -> bool {
    PDF_A1_A2_A3_PREDEFINED_SCHEMAS.contains(&uri)
        || (xmp_2005 && PDF_A2_A3_ADDITIONAL_PREDEFINED_SCHEMAS.contains(&uri))
}

#[derive(Default)]
struct Namespaces<'a> {
    by_uri: HashMap<&'a str, &'a str>,
    by_prefix: HashMap<&'a str, &'a str>,
}

impl<'a> Namespaces<'a> {
    fn register(&mut self, ns: &'a Namespace) -> Result<(), XmpError> {
        // Known schemas are always serialized under their canonical
        // prefix, so the user-supplied prefix can't conflict.
        if known_xmp_namespace(ns.uri()).is_some() {
            return Ok(());
        }

        if is_reserved_xmp_prefix(ns.prefix()) {
            return Err(XmpError::ReservedPrefix(ns.prefix().to_string()));
        }

        if *self.by_uri.entry(ns.uri()).or_insert(ns.prefix()) != ns.prefix() {
            return Err(XmpError::ConflictingNamespace(ns.uri().to_string()));
        }

        if *self.by_prefix.entry(ns.prefix()).or_insert(ns.uri()) != ns.uri() {
            return Err(XmpError::ConflictingPrefix(ns.prefix().to_string()));
        }

        Ok(())
    }

    fn register_schema(&mut self, schema: &'a Schema) -> Result<(), XmpError> {
        self.register(&schema.namespace)?;
        for desc in &schema.properties {
            for ty in struct_types(&desc.value_type) {
                self.register(&ty.namespace)?;
            }
        }

        Ok(())
    }
}

struct ValueWriter<'a, 'b> {
    namespaces: &'b mut Namespaces<'a>,
    property: &'a str,
}

impl<'a> ValueWriter<'a, '_> {
    fn namespace(&mut self, ns: &'a Namespace) -> Result<XmpNamespace<'a>, XmpError> {
        self.namespaces.register(ns)?;
        Ok(build_xmp_namespace(ns))
    }

    fn write_property(
        &mut self,
        element: xmp_writer::Element<'_, 'a>,
        value: &'a Value,
        value_type: Option<&'a ValueType>,
    ) -> Result<(), XmpError> {
        if value_type.is_some_and(|ty| !matches_value_type(value, ty)) {
            return Err(XmpError::InvalidValue(self.property.to_string()));
        }

        let item_type = value_type.and_then(|ty| match ty {
            ValueType::OrderedArray(item)
            | ValueType::UnorderedArray(item)
            | ValueType::AlternativeArray(item) => Some(&**item),
            _ => None,
        });

        match value {
            Value::Text(s) => element.value(s.as_str()),
            Value::Bool(b) => element.value(*b),
            Value::Integer(i) => element.value(*i),
            Value::Real(r) if r.is_finite() => element.value(*r),
            Value::Rational(n, d) if *d != 0 => element.value(format!("{n}/{d}").as_str()),
            Value::Real(_) | Value::Rational(..) => {
                return Err(XmpError::InvalidNumber(self.property.to_string()))
            }
            Value::Date(d) => element.value(xmp_date(*d)),
            Value::OrderedArray(items) => {
                self.write_array(element.array(RdfCollectionType::Seq), items, item_type)?
            }
            Value::UnorderedArray(items) => {
                self.write_array(element.array(RdfCollectionType::Bag), items, item_type)?
            }
            Value::AlternativeArray(items) => {
                self.write_array(element.array(RdfCollectionType::Alt), items, item_type)?
            }
            Value::LanguageAlternative(items) => {
                element.language_alternative(
                    items
                        .iter()
                        .map(|(lang, text)| (lang.as_deref().map(LangId), text.as_str())),
                );
            }
            Value::Struct(fields) => {
                let struct_type = match value_type {
                    Some(ValueType::Struct(ty)) => Some(&**ty),
                    _ => None,
                };
                self.write_struct(element.obj(), fields, struct_type)?;
            }
        }

        Ok(())
    }

    fn write_struct(
        &mut self,
        mut s: xmp_writer::Struct<'_, 'a>,
        fields: &'a [Property],
        struct_type: Option<&'a StructType>,
    ) -> Result<(), XmpError> {
        for field in fields {
            let field_type = match struct_type {
                Some(ty) => {
                    let desc = ty
                        .fields
                        .iter()
                        .find(|desc| desc.name == field.name && field.namespace == ty.namespace)
                        .ok_or_else(|| XmpError::InvalidValue(self.property.to_string()))?;
                    Some(&desc.value_type)
                }
                None => None,
            };

            let element = s.element(field.name.as_str(), self.namespace(&field.namespace)?);
            self.write_property(element, &field.value, field_type)?;
        }

        if let Some(ty) = struct_type {
            check_required(&ty.fields, |name| {
                fields.iter().any(|field| field.name == name)
            })?;
        }

        Ok(())
    }

    fn write_array(
        &mut self,
        mut array: xmp_writer::Array<'_, 'a>,
        items: &'a [Value],
        item_type: Option<&'a ValueType>,
    ) -> Result<(), XmpError> {
        for item in items {
            self.write_property(array.element(), item, item_type)?;
        }

        Ok(())
    }
}

fn check_required<'a>(
    descriptions: &'a [PropertyDescription],
    mut is_present: impl FnMut(&'a str) -> bool,
) -> Result<(), XmpError> {
    match descriptions
        .iter()
        .find(|desc| desc.required && !is_present(&desc.name))
    {
        Some(desc) => Err(XmpError::MissingProperty(desc.name.clone())),
        None => Ok(()),
    }
}

/// Whether the outermost shape of `value` matches `value_type`.
fn matches_value_type(value: &Value, value_type: &ValueType) -> bool {
    match (value_type, value) {
        (ValueType::Simple(ty) | ValueType::OpenChoice(ty, _), _) => {
            matches_simple_type(value, *ty)
        }
        (ValueType::ClosedChoice(ty, choices), _) => {
            matches_simple_type(value, *ty)
                && simple_value_text(value).is_some_and(|text| choices.contains(&text))
        }
        (ValueType::OrderedArray(_), Value::OrderedArray(_))
        | (ValueType::UnorderedArray(_), Value::UnorderedArray(_))
        | (ValueType::AlternativeArray(_), Value::AlternativeArray(_))
        | (ValueType::LanguageAlternative, Value::LanguageAlternative(_))
        | (ValueType::PredefinedStruct(_) | ValueType::Struct(_), Value::Struct(_)) => true,
        _ => false,
    }
}

fn matches_simple_type(value: &Value, ty: SimpleType) -> bool {
    match (ty, value) {
        (SimpleType::Boolean, Value::Bool(_))
        | (SimpleType::Integer, Value::Integer(_))
        | (SimpleType::Real, Value::Real(_) | Value::Integer(_))
        | (SimpleType::Rational, Value::Rational(..))
        | (SimpleType::Date, Value::Date(_)) => true,
        (
            SimpleType::Boolean
            | SimpleType::Integer
            | SimpleType::Real
            | SimpleType::Rational
            | SimpleType::Date,
            _,
        ) => false,
        (_, value) => matches!(value, Value::Text(_)),
    }
}

/// The serialized form of a simple value, as compared against the values of a
/// closed choice.
fn simple_value_text(value: &Value) -> Option<String> {
    match value {
        Value::Text(s) => Some(s.clone()),
        Value::Bool(b) => Some(if *b { "True" } else { "False" }.to_string()),
        Value::Integer(i) => Some(i.to_string()),
        Value::Real(r) => Some(r.to_string()),
        _ => None,
    }
}

/// The user-defined structure types used by `value_type`, including those
/// nested in other structure types.
fn struct_types(value_type: &ValueType) -> Vec<&StructType> {
    fn visit<'a>(value_type: &'a ValueType, types: &mut Vec<&'a StructType>) {
        match value_type {
            ValueType::OrderedArray(item)
            | ValueType::UnorderedArray(item)
            | ValueType::AlternativeArray(item) => visit(item, types),
            ValueType::Struct(ty) if !types.contains(&&**ty) => {
                types.push(ty);
                for field in &ty.fields {
                    visit(&field.value_type, types);
                }
            }
            _ => {}
        }
    }

    let mut types = Vec::new();
    visit(value_type, &mut types);
    types
}

pub(crate) fn build_xmp_namespace(ns: &Namespace) -> XmpNamespace<'_> {
    known_xmp_namespace(ns.uri()).unwrap_or_else(|| {
        XmpNamespace::Custom(Box::new(CustomNamespace::new(
            ns.prefix(),
            ns.prefix(),
            ns.uri(),
        )))
    })
}

/// The value of `pdfaProperty:valueType` describing `value_type`.
///
/// The property is an "Open Choice of Text" (TechNote 0009, Section 4.4).
fn pdfa_value_type(value_type: &ValueType) -> Cow<'_, str> {
    match value_type {
        ValueType::Simple(ty) => ty.as_str().into(),
        ValueType::LanguageAlternative => "Lang Alt".into(),
        ValueType::OpenChoice(ty, _) => format!("Open Choice of {}", ty.as_str()).into(),
        ValueType::ClosedChoice(ty, _) => format!("Closed Choice of {}", ty.as_str()).into(),
        ValueType::OrderedArray(item) => format!("seq {}", pdfa_value_type(item)).into(),
        ValueType::UnorderedArray(item) => format!("bag {}", pdfa_value_type(item)).into(),
        ValueType::AlternativeArray(item) => format!("alt {}", pdfa_value_type(item)).into(),
        ValueType::PredefinedStruct(ty) => ty.as_str().into(),
        ValueType::Struct(ty) => ty.name.as_str().into(),
    }
}

/// Writes the PDF/A extension schemas for krilla's own `builtin` schemas and
/// the `user` schemas, merging schemas of the same namespace.
pub(crate) fn write_extension_schemas(
    writer: &mut xmp_writer::pdfa::PdfAExtSchemasWriter,
    builtin: Vec<Schema>,
    user: &[&Schema],
) {
    let mut schemas: Vec<Cow<Schema>> = builtin.into_iter().map(Cow::Owned).collect();
    for &schema in user {
        let uri = schema.namespace.uri();
        match schemas.iter_mut().find(|s| s.namespace.uri() == uri) {
            Some(existing) => {
                let new: Vec<_> = schema
                    .properties
                    .iter()
                    .filter(|desc| existing.get(&desc.name).is_none())
                    .cloned()
                    .collect();
                existing.to_mut().properties.extend(new);
            }
            None => schemas.push(Cow::Borrowed(schema)),
        }
    }

    for schema in &schemas {
        write_extension_schema(writer, schema);
    }
}

fn write_extension_schema(writer: &mut xmp_writer::pdfa::PdfAExtSchemasWriter, schema: &Schema) {
    let xmp_namespace = build_xmp_namespace(&schema.namespace);
    // A namespace xmp-writer knows natively is serialized under its canonical
    // prefix, not the user-supplied one.
    let prefix = xmp_namespace.prefix();

    let mut schema_writer = writer.add_schema();
    let schema_name = schema
        .name
        .clone()
        .unwrap_or_else(|| format!("{prefix} schema"));
    schema_writer
        .element("schema", XmpNamespace::PdfASchema)
        .value(schema_name.as_str());
    schema_writer
        .element("namespaceURI", XmpNamespace::PdfASchema)
        .value(schema.namespace.uri());
    schema_writer
        .element("prefix", XmpNamespace::PdfASchema)
        .value(prefix);

    if !schema.properties.is_empty() {
        let mut properties = schema_writer.properties();
        for desc in &schema.properties {
            properties
                .add_property()
                .category(desc.category == Category::Internal)
                .description(desc.description.as_str())
                .name(desc.name.as_str())
                .value_type(&pdfa_value_type(&desc.value_type));
        }
    }

    let mut types = Vec::new();
    for desc in &schema.properties {
        for ty in struct_types(&desc.value_type) {
            if !types.contains(&ty) {
                types.push(ty);
            }
        }
    }

    if !types.is_empty() {
        let mut value_types = schema_writer.value_types();
        for ty in types {
            let mut type_writer = value_types.add_value_type();
            type_writer
                .name(ty.name.as_str())
                .namespace_uri(ty.namespace.uri())
                .prefix(build_xmp_namespace(&ty.namespace).prefix())
                .description(ty.description.as_str());
            write_fields(&mut type_writer, &ty.fields);
        }
    }
}

fn write_fields(
    type_writer: &mut xmp_writer::pdfa::PdfAExtTypeWriter,
    fields: &[PropertyDescription],
) {
    let mut fields_writer = type_writer.fields();
    for field in fields {
        fields_writer
            .add_field()
            .name(field.name.as_str())
            .value_type(&pdfa_value_type(&field.value_type))
            .description(field.description.as_str());
    }
}

#[cfg(test)]
mod tests {
    use xmp_writer::XmpWriter;

    use super::xmp::{Namespace, PropertyDescription, Schema, SimpleType};
    use super::{write_extension_schemas, Metadata};

    #[test]
    fn extension_schemas_of_same_namespace_are_merged() {
        let ns = Namespace::new("pdfuaid", "http://www.aiim.org/pdfua/ns/id/");
        let builtin = Schema::new(ns.clone()).property(PropertyDescription::new(
            "part",
            SimpleType::Integer,
            "Part",
        ));
        let user = Schema::new(ns)
            .property(PropertyDescription::new(
                "part",
                SimpleType::Integer,
                "Part",
            ))
            .property(PropertyDescription::new(
                "custom",
                SimpleType::Text,
                "Custom",
            ));

        let mut xmp = XmpWriter::new();
        write_extension_schemas(&mut xmp.extension_schemas(), vec![builtin], &[&user]);
        let xmp = xmp.finish(None);

        assert_eq!(xmp.matches("<pdfaSchema:namespaceURI>").count(), 1);
        assert_eq!(xmp.matches("<pdfaProperty:name>part<").count(), 1);
        assert_eq!(xmp.matches("<pdfaProperty:name>custom<").count(), 1);
    }

    #[test]
    fn custom_field_returns_none_for_ignored_names() {
        assert!(Metadata::new()
            .custom_field(String::new(), "value".to_string())
            .is_none());
        assert!(Metadata::new()
            .custom_field("Title".to_string(), "value".to_string())
            .is_none());

        let metadata = Metadata::new()
            .custom_field("CustomField".to_string(), "value".to_string())
            .unwrap();
        assert_eq!(
            metadata
                .custom_fields
                .get("CustomField")
                .map(String::as_str),
            Some("value")
        );
    }
}
