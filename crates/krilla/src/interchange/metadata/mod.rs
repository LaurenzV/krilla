//! Setting document metadata.
//!
//! PDF allows for the inclusion of metadata in a PDF document. To do so in krilla,
//! create a [`Metadata`] object, set the data, and then include it
//! in the document via [`Document::set_metadata`].
//!
//! [`Document::set_metadata`]: crate::document::Document::set_metadata
pub mod xmp;

use pdf_writer::{Finish, Pdf, Ref, TextStr};
use std::cell::LazyCell;
use xmp_writer::{
    CustomNamespace, LangId, Namespace as XmpNamespace, RdfCollectionType, Timezone, XmpWriter,
};

use crate::configure::{Configuration, PdfVersion, ValidationError, Validators};
use crate::serialize::SerializeContext;

use self::xmp::{Category, Namespace, Property, StructField, Value, XmpError};

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
    pub(crate) language: Option<String>,
    pub(crate) creation_date: Option<DateTime>,
    pub(crate) text_direction: Option<TextDirection>,
    pub(crate) page_layout: Option<PageLayout>,
    pub(crate) custom_xmp_properties: Vec<Property>,
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

    /// The custom XMP properties, each written under its user-defined
    /// namespace.
    pub fn custom_xmp_properties(mut self, properties: Vec<Property>) -> Self {
        self.custom_xmp_properties = properties;
        self
    }

    pub(crate) fn has_document_info(&self) -> bool {
        self.title.is_some()
            || self.producer.is_some()
            || self.keywords.is_some()
            || self.authors.is_some()
            || self.creator.is_some()
            || self.creation_date.is_some()
            || self.description.is_some()
    }

    pub(crate) fn serialize_xmp_metadata<'n>(
        &'n self,
        xmp: &mut XmpWriter<'n>,
        sc: &mut SerializeContext,
        instance_id: &str,
    ) {
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

        self.serialize_custom_xmp_properties(xmp, sc);
    }

    fn serialize_custom_xmp_properties<'n>(
        &'n self,
        xmp: &mut XmpWriter<'n>,
        sc: &mut SerializeContext,
    ) {
        if self.custom_xmp_properties.is_empty() {
            return;
        }

        let validators = sc.serialize_settings().validators();
        let check_descriptions = validators.requires_xmp_metadata_extension_schema();
        let xmp_2005 = validators.uses_xmp_2005_predefined_schemas();

        for property in &self.custom_xmp_properties {
            if check_descriptions
                && !is_predefined_pdfa_schema(&property.namespace.uri, xmp_2005)
                && !property
                    .namespace
                    .property_descriptions
                    .iter()
                    .any(|desc| desc.name == property.name)
            {
                sc.register_validation_error(ValidationError::MissingXmpPropertyDescription {
                    namespace_uri: property.namespace.uri.clone(),
                    property_name: property.name.clone(),
                });
            }

            let element = xmp.element(&property.name, build_xmp_namespace(&property.namespace));
            write_value(element, &property.value);
        }
    }

    /// Calls `f` on every namespace referenced by the custom XMP properties,
    /// including those of struct fields, in insertion order.
    fn visit_custom_xmp_namespaces<'a>(&'a self, f: &mut impl FnMut(&'a Namespace)) {
        fn visit_value<'a>(value: &'a Value, f: &mut impl FnMut(&'a Namespace)) {
            match value {
                Value::OrderedArray(items)
                | Value::UnorderedArray(items)
                | Value::AlternativeArray(items) => {
                    for item in items {
                        visit_value(item, f);
                    }
                }
                Value::Struct(fields) => {
                    for field in fields {
                        f(&field.namespace);
                        visit_value(&field.value, f);
                    }
                }
                _ => {}
            }
        }

        for property in &self.custom_xmp_properties {
            f(&property.namespace);
            visit_value(&property.value, f);
        }
    }

    /// Returns the user-defined XMP namespaces that need a PDF/A extension
    /// schema, deduplicated by URI in insertion order. Only predefined PDF/A
    /// schemas (version-dependent, see [`is_predefined_pdfa_schema`]) are
    /// excluded.
    pub(crate) fn deduped_custom_xmp_namespaces(&self, validators: Validators) -> Vec<&Namespace> {
        let xmp_2005 = validators.uses_xmp_2005_predefined_schemas();
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        self.visit_custom_xmp_namespaces(&mut |ns| {
            if !is_predefined_pdfa_schema(&ns.uri, xmp_2005) && seen.insert(ns.uri.as_str()) {
                result.push(ns);
            }
        });

        result
    }

    /// Checks the custom XMP properties for problems that would corrupt the
    /// output regardless of the validator: namespace conflicts that produce
    /// invalid XML (one prefix bound to two URIs) or silently drop data (one
    /// URI declared with different definitions), and non-finite reals.
    pub(crate) fn check_custom_xmp_properties(&self) -> Result<(), XmpError> {
        let mut namespaces = Vec::new();
        self.visit_custom_xmp_namespaces(&mut |ns| namespaces.push(ns));

        let mut by_uri = std::collections::HashMap::new();
        let mut by_prefix = std::collections::HashMap::new();

        for ns in namespaces {
            // Predefined schemas are always serialized under their canonical
            // prefix, so the user-supplied fields can't conflict.
            if builtin_xmp_namespace(&ns.uri).is_some() {
                continue;
            }

            if is_reserved_xmp_prefix(&ns.prefix) {
                return Err(XmpError::ReservedPrefix(ns.prefix.clone()));
            }

            if *by_uri.entry(ns.uri.as_str()).or_insert(ns) != ns {
                return Err(XmpError::ConflictingNamespace(ns.uri.clone()));
            }

            if *by_prefix
                .entry(ns.prefix.as_str())
                .or_insert(ns.uri.as_str())
                != ns.uri
            {
                return Err(XmpError::ConflictingPrefix(ns.prefix.clone()));
            }
        }

        for property in &self.custom_xmp_properties {
            if has_non_finite_real(&property.value) {
                return Err(XmpError::NonFiniteReal(property.name.clone()));
            }
        }

        Ok(())
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
        }
    }
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

/// The namespaces xmp-writer can serialize natively. A user-supplied namespace
/// with one of these URIs must be mapped to the corresponding built-in variant,
/// since xmp-writer would otherwise declare the same namespace twice.
static BUILTIN_XMP_NAMESPACES: &[XmpNamespace<'static>] = &[
    XmpNamespace::Rdf,
    XmpNamespace::DublinCore,
    XmpNamespace::Xmp,
    XmpNamespace::XmpRights,
    XmpNamespace::XmpResourceRef,
    XmpNamespace::XmpResourceEvent,
    XmpNamespace::XmpVersion,
    XmpNamespace::XmpJob,
    XmpNamespace::XmpJobManagement,
    XmpNamespace::XmpColorant,
    XmpNamespace::XmpFont,
    XmpNamespace::XmpDimensions,
    XmpNamespace::XmpMedia,
    XmpNamespace::XmpPaged,
    XmpNamespace::XmpDynamicMedia,
    XmpNamespace::XmpImage,
    XmpNamespace::XmpIdq,
    XmpNamespace::AdobePdf,
    XmpNamespace::PdfAId,
    XmpNamespace::PdfUAId,
    XmpNamespace::PdfXId,
    XmpNamespace::PdfAExtension,
    XmpNamespace::PdfASchema,
    XmpNamespace::PdfAProperty,
    XmpNamespace::PdfAType,
    XmpNamespace::PdfAField,
];

/// Returns the predefined xmp-writer namespace with the given URI, if any.
fn builtin_xmp_namespace(uri: &str) -> Option<XmpNamespace<'static>> {
    BUILTIN_XMP_NAMESPACES
        .iter()
        .find(|ns| ns.url() == uri)
        .cloned()
}

fn is_reserved_xmp_prefix(prefix: &str) -> bool {
    BUILTIN_XMP_NAMESPACES
        .iter()
        .any(|ns| ns.prefix() == prefix)
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

fn has_non_finite_real(value: &Value) -> bool {
    match value {
        Value::Real(r) => !r.is_finite(),
        Value::OrderedArray(items)
        | Value::UnorderedArray(items)
        | Value::AlternativeArray(items) => items.iter().any(has_non_finite_real),
        Value::Struct(fields) => fields.iter().any(|field| has_non_finite_real(&field.value)),
        _ => false,
    }
}

pub(crate) fn build_xmp_namespace(ns: &Namespace) -> XmpNamespace<'_> {
    builtin_xmp_namespace(&ns.uri).unwrap_or_else(|| {
        XmpNamespace::Custom(Box::new(CustomNamespace::new(
            ns.schema_name.as_deref().unwrap_or(ns.prefix.as_str()),
            ns.prefix.as_str(),
            ns.uri.as_str(),
        )))
    })
}

fn write_value<'n>(element: xmp_writer::Element<'_, 'n>, value: &'n Value) {
    match value {
        Value::Text(s) => element.value(s.as_str()),
        Value::Bool(b) => element.value(*b),
        Value::Integer(i) => element.value(*i),
        Value::Real(r) => element.value(*r),
        Value::Date(d) => element.value(xmp_date(*d)),
        Value::OrderedArray(items) => write_array(element.array(RdfCollectionType::Seq), items),
        Value::UnorderedArray(items) => write_array(element.array(RdfCollectionType::Bag), items),
        Value::AlternativeArray(items) => write_array(element.array(RdfCollectionType::Alt), items),
        Value::LanguageAlternative(items) => {
            element.language_alternative(
                items
                    .iter()
                    .map(|(lang, text)| (lang.as_deref().map(LangId), text.as_str())),
            );
        }
        Value::Struct(fields) => write_struct(element.obj(), fields),
    }
}

fn write_array<'n>(mut array: xmp_writer::Array<'_, 'n>, items: &'n [Value]) {
    for item in items {
        write_value(array.element(), item);
    }
}

fn write_struct<'n>(mut s: xmp_writer::Struct<'_, 'n>, fields: &'n [StructField]) {
    for field in fields {
        let element = s.element(field.name.as_str(), build_xmp_namespace(&field.namespace));
        write_value(element, &field.value);
    }
}

pub(crate) fn write_user_extension_schema(
    schemas: &mut xmp_writer::pdfa::PdfAExtSchemasWriter,
    ns: &Namespace,
) {
    let xmp_namespace = build_xmp_namespace(ns);
    // A namespace xmp-writer knows natively is serialized under its canonical
    // prefix, not the user-supplied one.
    let prefix = xmp_namespace.prefix();

    let mut schema = schemas.add_schema();
    let schema_name = ns
        .schema_name
        .clone()
        .unwrap_or_else(|| format!("{prefix} schema"));
    schema
        .element("schema", XmpNamespace::PdfASchema)
        .value(schema_name.as_str());
    schema
        .element("namespaceURI", XmpNamespace::PdfASchema)
        .value(ns.uri.as_str());
    schema
        .element("prefix", XmpNamespace::PdfASchema)
        .value(prefix);

    if !ns.property_descriptions.is_empty() {
        let mut properties = schema.properties();
        for desc in &ns.property_descriptions {
            properties
                .add_property()
                .name(desc.name.as_str())
                .value_type(desc.value_type.as_str())
                .category(desc.category == Category::Internal)
                .description(desc.description.as_str());
        }
    }
}
