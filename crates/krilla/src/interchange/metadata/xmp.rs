//! Custom XMP metadata.
//!
//! In addition to the standard properties exposed on [`Metadata`], krilla
//! supports writing arbitrary XMP metadata under user-defined namespaces.
//! This is useful for embedding things like e-invoice descriptors
//! (e.g. ZUGFeRD/Factur-X) or RDF license information into a PDF.
//!
//! Create a [`Namespace`], build [`Property`] values in it with
//! [`Namespace::property`], and attach them via
//! [`Metadata::custom_xmp_property`].
//!
//! ## Schemas
//!
//! A [`Schema`] describes the properties of a namespace: their value types
//! and what they mean. Attach it via [`Metadata::custom_xmp_schema`].
//! Properties are checked against the schema of their namespace, if there is
//! one.
//!
//! PDF/A-1 to PDF/A-3 require every property outside the predefined XMP
//! schemas to be described in a PDF/A extension schema, which krilla writes
//! from the attached schemas. For other export targets, schemas are
//! optional.
//!
//! ## Predefined schemas
//!
//! If a namespace URI matches a schema known to xmp-writer,
//! properties are written under that schema's canonical prefix, and the
//! namespace's prefix is ignored. Such a namespace can't be used to
//! redefine properties of the predefined schema.
//!
//! Custom namespaces must not reuse the prefix of a predefined schema or
//! bind one prefix to two different URIs, and each URI must always be used
//! with the same prefix; otherwise [`Document::finish`] returns an
//! [`XmpError`].
//!
//! [`Document::finish`]: crate::document::Document::finish
//! [`Metadata`]: super::Metadata
//! [`Metadata::custom_xmp_property`]: super::Metadata::custom_xmp_property
//! [`Metadata::custom_xmp_schema`]: super::Metadata::custom_xmp_schema

use std::sync::Arc;

use super::DateTime;

/// An XMP namespace, identified by its URI.
///
/// The prefix is the short name used in the serialized XML.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Namespace {
    prefix: Arc<str>,
    uri: Arc<str>,
}

impl Namespace {
    /// Create a new namespace with the given prefix (e.g. `"fx"`) and URI
    /// (e.g. `"urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#"`).
    pub fn new(prefix: impl Into<Arc<str>>, uri: impl Into<Arc<str>>) -> Self {
        Self {
            prefix: prefix.into(),
            uri: uri.into(),
        }
    }

    /// The prefix of the namespace.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// The URI of the namespace.
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Create a property in this namespace.
    pub fn property(&self, name: impl Into<String>, value: impl Into<Value>) -> Property {
        Property::new(self.clone(), name, value.into())
    }
}

/// A description of the properties in a [`Namespace`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    pub(crate) namespace: Namespace,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) properties: Vec<PropertyDescription>,
}

impl Schema {
    /// Create a new schema without any properties.
    pub fn new(namespace: Namespace) -> Self {
        Self {
            namespace,
            name: None,
            description: None,
            properties: Vec::new(),
        }
    }

    /// The human-readable name of the schema. Defaults to
    /// `"<prefix> schema"`.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// A human-readable description of the schema.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Describe a property of the schema.
    pub fn property(mut self, property: PropertyDescription) -> Self {
        self.properties.push(property);
        self
    }

    pub(crate) fn get(&self, name: &str) -> Option<&PropertyDescription> {
        self.properties.iter().find(|desc| desc.name == name)
    }
}

/// Description of a single XMP property, or of a field of a [`StructType`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDescription {
    pub(crate) name: String,
    pub(crate) value_type: ValueType,
    pub(crate) category: Category,
    pub(crate) required: bool,
    pub(crate) description: String,
}

impl PropertyDescription {
    /// Create a new description of an optional, external property.
    pub fn new(
        name: impl Into<String>,
        value_type: impl Into<ValueType>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            value_type: value_type.into(),
            category: Category::External,
            required: false,
            description: description.into(),
        }
    }

    /// Mark the property as being generated internally by the producer.
    pub fn internal(mut self) -> Self {
        self.category = Category::Internal;
        self
    }

    /// Mark the property as required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// Whether a property is generated internally or supplied externally.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Category {
    /// The property is computed by the producer (e.g. page count).
    Internal,
    /// The property is supplied by the user.
    External,
}

/// A user-defined structure type.
///
/// The fields of a structure are properties in the namespace of the type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructType {
    pub(crate) namespace: Namespace,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) fields: Vec<PropertyDescription>,
}

impl StructType {
    /// Create a new structure type without any fields.
    pub fn new(
        namespace: Namespace,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            namespace,
            name: name.into(),
            description: description.into(),
            fields: Vec::new(),
        }
    }

    /// Describe a field of the structure.
    pub fn field(mut self, field: PropertyDescription) -> Self {
        self.fields.push(field);
        self
    }
}

/// The type of an XMP value.
///
/// This mirrors the XMP type system: simple types, choices, the three array
/// kinds, language alternatives and structures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    /// A simple value.
    Simple(SimpleType),
    /// A value of the given type, typically one of the recommended values.
    OpenChoice(SimpleType, Vec<String>),
    /// One of the given values.
    ClosedChoice(SimpleType, Vec<String>),
    /// An ordered array (`rdf:Seq`) of the contained type.
    OrderedArray(Box<ValueType>),
    /// An unordered array (`rdf:Bag`) of the contained type.
    UnorderedArray(Box<ValueType>),
    /// An alternative array (`rdf:Alt`) of the contained type.
    AlternativeArray(Box<ValueType>),
    /// A language alternative: an `rdf:Alt` of `xml:lang`-tagged text.
    LanguageAlternative,
    /// A structure type defined by the XMP specification.
    PredefinedStruct(PredefinedStruct),
    /// A user-defined structure type.
    Struct(Arc<StructType>),
}

impl ValueType {
    /// An ordered array (`rdf:Seq`) of the given type.
    pub fn ordered_array(item: impl Into<ValueType>) -> Self {
        Self::OrderedArray(Box::new(item.into()))
    }

    /// An unordered array (`rdf:Bag`) of the given type.
    pub fn unordered_array(item: impl Into<ValueType>) -> Self {
        Self::UnorderedArray(Box::new(item.into()))
    }

    /// An alternative array (`rdf:Alt`) of the given type.
    pub fn alternative_array(item: impl Into<ValueType>) -> Self {
        Self::AlternativeArray(Box::new(item.into()))
    }

    /// A value of the given type, typically one of `values`.
    pub fn open_choice(
        ty: SimpleType,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::OpenChoice(ty, values.into_iter().map(Into::into).collect())
    }

    /// One of `values`.
    pub fn closed_choice(
        ty: SimpleType,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::ClosedChoice(ty, values.into_iter().map(Into::into).collect())
    }
}

impl From<SimpleType> for ValueType {
    fn from(value: SimpleType) -> Self {
        Self::Simple(value)
    }
}

impl From<PredefinedStruct> for ValueType {
    fn from(value: PredefinedStruct) -> Self {
        Self::PredefinedStruct(value)
    }
}

impl From<StructType> for ValueType {
    fn from(value: StructType) -> Self {
        Self::Struct(Arc::new(value))
    }
}

impl From<Arc<StructType>> for ValueType {
    fn from(value: Arc<StructType>) -> Self {
        Self::Struct(value)
    }
}

/// A simple XMP value type.
///
/// `Text`, `Boolean`, `Integer`, `Real` and `Date` are the core simple types;
/// the rest are derived from them and are all stored as text, but carry
/// additional meaning.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SimpleType {
    /// Unconstrained text.
    Text,
    /// A boolean, written as `True` or `False`.
    Boolean,
    /// An integer.
    Integer,
    /// A floating-point number.
    Real,
    /// A date and time.
    Date,
    /// The name of an agent, i.e. a piece of software.
    AgentName,
    /// A globally unique identifier.
    Guid,
    /// A language tag.
    Locale,
    /// A MIME type.
    MimeType,
    /// The name of a person or organization.
    ProperName,
    /// The name of a rendition of a document.
    RenditionClass,
    /// A URI.
    Uri,
    /// A URL.
    Url,
    /// A rational number, written as `numerator/denominator`.
    Rational,
    /// A frame rate, written as `f<frames>` or `f<frames>s<basis>`.
    FrameRate,
    /// A number of frames at a given frame rate.
    FrameCount,
    /// A path identifying a portion of a resource, e.g. `/content/audio`.
    Part,
    /// An XPath expression.
    XPath,
}

impl SimpleType {
    /// The name of the type in a PDF/A extension schema description.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Boolean => "Boolean",
            Self::Integer => "Integer",
            Self::Real => "Real",
            Self::Date => "Date",
            Self::AgentName => "AgentName",
            Self::Guid => "GUID",
            Self::Locale => "Locale",
            Self::MimeType => "MIMEType",
            Self::ProperName => "ProperName",
            Self::RenditionClass => "RenditionClass",
            Self::Uri => "URI",
            Self::Url => "URL",
            Self::Rational => "Rational",
            Self::FrameRate => "FrameRate",
            Self::FrameCount => "FrameCount",
            Self::Part => "Part",
            Self::XPath => "XPath",
        }
    }
}

/// A structure type defined by the XMP specification.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PredefinedStruct {
    /// The dimensions of a document (`stDim`).
    Dimensions,
    /// A thumbnail image (`xmpGImg`).
    Thumbnail,
    /// A high-level event in the history of a resource (`stEvt`).
    ResourceEvent,
    /// A reference to another resource (`stRef`).
    ResourceRef,
    /// A version of a document (`stVer`).
    Version,
    /// A job (`stJob`).
    Job,
    /// A colorant (`xmpG`).
    Colorant,
    /// A font (`stFnt`).
    Font,
}

impl PredefinedStruct {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Dimensions => "Dimensions",
            Self::Thumbnail => "Thumbnail",
            Self::ResourceEvent => "ResourceEvent",
            Self::ResourceRef => "ResourceRef",
            Self::Version => "Version",
            Self::Job => "Job",
            Self::Colorant => "Colorant",
            Self::Font => "Font",
        }
    }
}

/// A single XMP property in a [`Namespace`].
///
/// Also used for the fields of a [`Value::Struct`], which are named values in
/// a namespace just like top-level properties.
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    /// The namespace this property belongs to.
    pub namespace: Namespace,
    /// The property name within the namespace.
    pub name: String,
    /// The property value.
    pub value: Value,
}

impl Property {
    /// Create a new XMP property.
    pub fn new(namespace: Namespace, name: impl Into<String>, value: Value) -> Self {
        Self {
            namespace,
            name: name.into(),
            value,
        }
    }
}

/// An XMP property value.
///
/// The variants mirror the value shapes supported by XMP/RDF: simple
/// values, the three array kinds (`rdf:Seq`, `rdf:Bag`, `rdf:Alt`), language
/// alternatives, and structs. Derived types like URIs or agent names are
/// written as [`Value::Text`].
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A text value.
    Text(String),
    /// A boolean value.
    Bool(bool),
    /// An integer value.
    Integer(i64),
    /// A floating-point value. Must be finite.
    Real(f64),
    /// A rational number, given as numerator and denominator. The
    /// denominator must not be zero.
    Rational(i64, u64),
    /// A date value.
    Date(DateTime),
    /// An ordered array (`rdf:Seq`).
    OrderedArray(Vec<Value>),
    /// An unordered array (`rdf:Bag`).
    UnorderedArray(Vec<Value>),
    /// An alternative array (`rdf:Alt`).
    AlternativeArray(Vec<Value>),
    /// A language-alternative array (`rdf:Alt` of `xml:lang`-tagged text).
    ///
    /// Each entry pairs an optional RFC 3066 language tag (`None` ⇒
    /// `x-default`) with its text value.
    LanguageAlternative(Vec<(Option<String>, String)>),
    /// A struct value, given as its fields.
    Struct(Vec<Property>),
}

impl Value {
    /// Convenience constructor for a [`Value::Text`].
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Integer(value.into())
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Real(value)
    }
}

impl From<DateTime> for Value {
    fn from(value: DateTime) -> Self {
        Self::Date(value)
    }
}

/// An invalid set of custom XMP properties or schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmpError {
    /// One namespace URI was used with two different prefixes. Contains the
    /// namespace URI.
    ConflictingNamespace(String),
    /// One prefix was bound to two different namespace URIs. Contains the
    /// prefix.
    ConflictingPrefix(String),
    /// A custom namespace used a prefix reserved by a predefined XMP schema
    /// (e.g. `dc`, `xmp`, `pdf`). Contains the prefix.
    ReservedPrefix(String),
    /// More than one schema was attached for the same namespace. Contains
    /// the namespace URI.
    DuplicateSchema(String),
    /// A [`Value::Real`] was NaN or infinite, or a [`Value::Rational`] had
    /// a zero denominator. Contains the name of the containing property.
    InvalidNumber(String),
    /// A value didn't match the type its schema declared for it. Contains
    /// the name of the containing property.
    InvalidValue(String),
    /// A property or structure field that its schema marked as required was
    /// missing. Contains the name of the property or field.
    MissingProperty(String),
    /// A property that krilla writes itself was set. Such properties must be
    /// set via [`Metadata`](super::Metadata), if at all. Contains the
    /// qualified name of the property.
    ManagedProperty(String),
}

impl std::fmt::Display for XmpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmpError::ConflictingNamespace(uri) => {
                write!(
                    f,
                    "the namespace {uri} was declared with multiple different prefixes"
                )
            }
            XmpError::ConflictingPrefix(prefix) => {
                write!(
                    f,
                    "the prefix {prefix} was bound to two different namespaces"
                )
            }
            XmpError::ReservedPrefix(prefix) => {
                write!(
                    f,
                    "the prefix {prefix} is reserved by a predefined XMP schema"
                )
            }
            XmpError::DuplicateSchema(uri) => {
                write!(f, "the namespace {uri} was described by multiple schemas")
            }
            XmpError::InvalidNumber(name) => {
                write!(f, "the property {name} contained an invalid number")
            }
            XmpError::InvalidValue(name) => {
                write!(f, "the property {name} doesn't match its schema")
            }
            XmpError::MissingProperty(name) => {
                write!(f, "the required property {name} is missing")
            }
            XmpError::ManagedProperty(name) => {
                write!(f, "the property {name} is managed by krilla")
            }
        }
    }
}

impl std::error::Error for XmpError {}
