use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use xmp_writer::{
    CustomNamespace, LangId, Namespace as XmpNamespace, RdfCollectionType, XmpWriter,
};

use crate::configure::ValidationError;
use crate::serialize::SerializeContext;

use super::extension_schema::check_pdfa_value_types;
use super::predefined::{is_predefined_pdfa_property, managed_prefix, predefined_struct_type};
use super::xmp::{
    Namespace, Property, PropertyDescription, Schema, SimpleType, StructType, Value, ValueType,
    XmpError,
};
use super::{DateTime, Metadata};

impl Metadata {
    pub(super) fn serialize_custom_xmp<'n>(
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
        if check_descriptions {
            check_pdfa_value_types(&self.extension_schemas(validators), sc);
        }
        let xmp_2005 = validators.uses_xmp_2005_predefined_schemas();
        let builtin_schemas = validators.extension_schemas();

        let mut written = HashSet::new();
        for property in &self.custom_xmp_properties {
            check_name(&property.name)?;
            let uri = property.namespace.uri();
            if let Some(prefix) = managed_prefix(uri, &property.name) {
                return Err(XmpError::ManagedProperty(format!(
                    "{prefix}:{}",
                    property.name
                )));
            }

            let description = schemas.get(uri).and_then(|s| s.get(&property.name));
            if check_descriptions
                && description.is_none()
                && !is_predefined_pdfa_property(uri, &property.name, xmp_2005)
                && !builtin_schemas
                    .iter()
                    .any(|s| s.namespace.uri() == uri && s.get(&property.name).is_some())
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
            if !written.insert((uri, property.name.as_str())) {
                return Err(XmpError::DuplicateProperty(format!(
                    "{}:{}",
                    namespace.prefix(),
                    property.name
                )));
            }
            writer.write_property(
                xmp.element(&property.name, namespace),
                &property.value,
                description.map(|desc| &desc.value_type),
            )?;
        }

        for schema in &self.custom_xmp_schemas {
            check_required(&schema.properties, |name| {
                written.contains(&(schema.namespace.uri(), name))
            })?;
        }

        Ok(())
    }
}

/// Formats a date of a custom property in the closest of the forms allowed by
/// XMP Specification Part 1, section 8.2.1.2.
fn custom_xmp_date(date: DateTime) -> String {
    let mut text = format!("{:04}", date.year);
    let (Some(month), Some(day)) = (date.month, date.day) else {
        if let Some(month) = date.month {
            write!(text, "-{month:02}").unwrap();
        }
        return text;
    };

    write!(text, "-{month:02}-{day:02}").unwrap();
    let Some(hour) = date.hour else {
        return text;
    };

    write!(text, "T{hour:02}:{:02}", date.minute.unwrap_or(0)).unwrap();
    if let Some(second) = date.second {
        write!(text, ":{second:02}").unwrap();
    }

    if let Some(offset_hour) = date.utc_offset_hour {
        let sign = if offset_hour < 0 { '-' } else { '+' };
        write!(
            text,
            "{sign}{:02}:{:02}",
            offset_hour.unsigned_abs(),
            date.utc_offset_minute
        )
        .unwrap();
    }

    text
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
    matches!(prefix, "xml" | "xmlns")
        || XmpNamespace::PREDEFINED
            .iter()
            .any(|ns| ns.prefix() == prefix)
}

/// Checks that `name` is an `NCName`, i.e. an XML name without a colon.
fn check_name(name: &str) -> Result<(), XmpError> {
    fn is_start_char(c: char) -> bool {
        matches!(c,
            'A'..='Z' | '_' | 'a'..='z'
            | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}'
            | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}'
            | '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'..='\u{EFFFF}'
        )
    }

    fn is_char(c: char) -> bool {
        is_start_char(c)
            || matches!(c,
                '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}'
            )
    }

    let mut chars = name.chars();
    if chars.next().is_some_and(is_start_char) && chars.all(is_char) {
        Ok(())
    } else {
        Err(XmpError::InvalidName(name.to_string()))
    }
}

fn check_description_names(descriptions: &[PropertyDescription]) -> Result<(), XmpError> {
    descriptions
        .iter()
        .try_for_each(|desc| check_name(&desc.name))
}

/// Merges the `user` schemas into the `builtin` schemas of the same namespace.
pub(super) fn merge_schemas<'a>(
    builtin: impl IntoIterator<Item = Cow<'a, Schema>>,
    user: impl IntoIterator<Item = Cow<'a, Schema>>,
) -> Vec<Cow<'a, Schema>> {
    let mut schemas: Vec<_> = builtin.into_iter().collect();
    for schema in user {
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
            None => schemas.push(schema),
        }
    }

    schemas
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

        check_name(ns.prefix())?;
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
        check_description_names(&schema.properties)?;
        for desc in &schema.properties {
            for ty in struct_types(&desc.value_type) {
                self.register(&ty.namespace)?;
                check_name(&ty.name)?;
                check_description_names(&ty.fields)?;
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
            Value::Date(d) => element.value(custom_xmp_date(*d).as_str()),
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
                    Some(ValueType::PredefinedStruct(ty)) => predefined_struct_type(*ty),
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
        let mut written = HashSet::new();
        for field in fields {
            let field_type = match struct_type {
                Some(ty) => {
                    let desc = ty
                        .fields
                        .iter()
                        .find(|desc| {
                            desc.name == field.name && field.namespace.uri() == ty.namespace.uri()
                        })
                        .ok_or_else(|| XmpError::InvalidValue(self.property.to_string()))?;
                    Some(&desc.value_type)
                }
                None => None,
            };

            check_name(&field.name)?;
            let namespace = self.namespace(&field.namespace)?;
            if !written.insert((field.namespace.uri(), field.name.as_str())) {
                return Err(XmpError::DuplicateProperty(format!(
                    "{}:{}",
                    namespace.prefix(),
                    field.name
                )));
            }
            let element = s.element(field.name.as_str(), namespace);
            self.write_property(element, &field.value, field_type)?;
        }

        if let Some(ty) = struct_type {
            check_required(&ty.fields, |name| {
                fields
                    .iter()
                    .any(|field| field.name == name && field.namespace.uri() == ty.namespace.uri())
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
pub(super) fn struct_types(value_type: &ValueType) -> Vec<&StructType> {
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

pub(super) fn build_xmp_namespace(ns: &Namespace) -> XmpNamespace<'_> {
    known_xmp_namespace(ns.uri()).unwrap_or_else(|| {
        XmpNamespace::Custom(Box::new(CustomNamespace::new(
            ns.prefix(),
            ns.prefix(),
            ns.escaped_uri(),
        )))
    })
}

#[cfg(test)]
mod tests {
    use super::custom_xmp_date;
    use crate::metadata::DateTime;

    #[test]
    fn custom_dates_keep_their_precision() {
        let date = DateTime::new(2024);
        assert_eq!(custom_xmp_date(date), "2024");
        assert_eq!(custom_xmp_date(date.day(3)), "2024");
        assert_eq!(custom_xmp_date(date.month(5)), "2024-05");
        assert_eq!(custom_xmp_date(date.month(5).hour(10)), "2024-05");

        let day = date.month(5).day(3);
        assert_eq!(custom_xmp_date(day), "2024-05-03");
        assert_eq!(custom_xmp_date(day.utc_offset_hour(2)), "2024-05-03");
        assert_eq!(custom_xmp_date(day.hour(10)), "2024-05-03T10:00");
        assert_eq!(
            custom_xmp_date(day.hour(10).minute(30).second(15)),
            "2024-05-03T10:30:15"
        );
        assert_eq!(
            custom_xmp_date(day.hour(10).minute(30).utc_offset_hour(2)),
            "2024-05-03T10:30+02:00"
        );
        assert_eq!(
            custom_xmp_date(day.hour(10).utc_offset_hour(-3).utc_offset_minute(30)),
            "2024-05-03T10:00-03:30"
        );
        assert_eq!(
            custom_xmp_date(day.hour(10).utc_offset_hour(0)),
            "2024-05-03T10:00+00:00"
        );
    }
}
