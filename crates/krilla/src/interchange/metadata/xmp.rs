//! Custom XMP metadata.
//!
//! In addition to the standard properties exposed on [`Metadata`], krilla
//! supports writing arbitrary XMP metadata under user-defined namespaces.
//! This is useful for embedding things like e-invoice descriptors
//! (e.g. ZUGFeRD/Factur-X) or RDF license information into a PDF.
//!
//! Build a [`Property`] with a [`Namespace`] and a [`Value`], then attach
//! the properties via [`Metadata::custom_xmp_properties`].
//!
//! ## Predefined schemas
//!
//! If a namespace URI matches a schema predefined by the XMP specification,
//! properties are written under that schema's canonical prefix, and the
//! namespace's prefix, schema name and property descriptions are ignored.
//! It is your responsibility that the property actually exists in the
//! predefined schema.
//!
//! Custom namespaces must not reuse the prefix of a predefined schema or
//! bind one prefix to two different URIs, and all declarations of the same
//! URI must be identical; otherwise [`Document::finish`] returns an
//! [`XmpError`].
//!
//! ## PDF/A
//!
//! Any property in a custom namespace must be described in a PDF/A
//! extension schema. Populate [`Namespace::property_descriptions`] for
//! every property you write.
//!
//! [`Document::finish`]: crate::document::Document::finish
//! [`Metadata`]: super::Metadata
//! [`Metadata::custom_xmp_properties`]: super::Metadata::custom_xmp_properties

use super::DateTime;

/// An XMP namespace.
///
/// A namespace is identified by its URI. The prefix is the short name used
/// in the serialized XML. For PDF/A output, the
/// namespace must describe each property through
/// [`Self::property_descriptions`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namespace {
    /// The XML prefix (e.g. `"fx"`).
    pub prefix: String,
    /// The namespace URI (e.g. `"urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#"`).
    pub uri: String,
    /// Optional human-readable schema name for the PDF/A extension schema
    /// description. Defaults to `"<prefix> schema"`.
    pub schema_name: Option<String>,
    /// PDF/A extension-schema property descriptions.
    ///
    /// Required for any property name written under this namespace
    /// when exporting to PDF/A.
    pub property_descriptions: Vec<PropertyDescription>,
}

impl Namespace {
    /// Create a new namespace with the given prefix and URI.
    pub fn new(prefix: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            uri: uri.into(),
            schema_name: None,
            property_descriptions: Vec::new(),
        }
    }

    /// Set the human-readable schema name used for the PDF/A extension
    /// schema description.
    pub fn schema_name(mut self, name: impl Into<String>) -> Self {
        self.schema_name = Some(name.into());
        self
    }

    /// Add a property description used for the PDF/A extension schema.
    pub fn add_description(
        mut self,
        name: impl Into<String>,
        value_type: impl Into<String>,
        category: Category,
        description: impl Into<String>,
    ) -> Self {
        self.property_descriptions.push(PropertyDescription {
            name: name.into(),
            value_type: value_type.into(),
            category,
            description: description.into(),
        });
        self
    }
}

/// Description of a single XMP property under a [`Namespace`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDescription {
    /// The property name (e.g. `"DocumentType"`).
    pub name: String,
    /// The value type. Either a built-in XMP type (`"Text"`, `"Integer"`,
    /// `"Date"`, ...) or a custom value type.
    pub value_type: String,
    /// Whether the property is generated internally by the producer or
    /// supplied externally by the user.
    pub category: Category,
    /// Human-readable description of the property.
    pub description: String,
}

/// Whether a property is generated internally or supplied externally.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Category {
    /// The property is computed by the producer (e.g. page count).
    Internal,
    /// The property is supplied by the user.
    External,
}

/// A single XMP property attached to a [`Namespace`].
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

/// A field of a [`Value::Struct`] value.
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    /// The namespace of this field.
    pub namespace: Namespace,
    /// The field name within its namespace.
    pub name: String,
    /// The field value.
    pub value: Value,
}

impl StructField {
    /// Create a new struct field.
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
/// The variants mirror the value shapes supported by XMP/RDF: primitive
/// types, the three array kinds (`rdf:Seq`, `rdf:Bag`, `rdf:Alt`), language
/// alternatives, and structs.
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
    /// A struct value (`rdf:parseType="Resource"`).
    Struct(Vec<StructField>),
}

impl Value {
    /// Convenience constructor for a [`Value::Text`].
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    /// Convenience constructor for a [`Value::Date`].
    pub fn date(value: DateTime) -> Self {
        Self::Date(value)
    }
}

/// An invalid set of custom XMP properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmpError {
    /// The same namespace URI was declared inconsistently: all declarations
    /// of one URI must be identical, but two of them disagreed on the prefix,
    /// schema name, or property descriptions. Contains the namespace URI.
    ConflictingNamespace(String),
    /// One prefix was bound to two different namespace URLs. Contains the
    /// prefix.
    ConflictingPrefix(String),
    /// A custom namespace used a prefix reserved by a predefined XMP schema
    /// (e.g. `dc`, `xmp`, `pdf`). Contains the prefix.
    ReservedPrefix(String),
    /// A [`Value::Real`] was NaN or infinite, which can't be represented in
    /// XMP. Contains the name of the containing property.
    NonFiniteReal(String),
}

impl std::fmt::Display for XmpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmpError::ConflictingNamespace(uri) => {
                write!(f, "the namespace {uri} was declared with multiple different prefixes")
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
            XmpError::NonFiniteReal(name) => {
                write!(f, "the property {name} contained a non-finite real number")
            }
        }
    }
}

impl std::error::Error for XmpError {}
