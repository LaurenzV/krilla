use std::borrow::Cow;

use xmp_writer::Namespace as XmpNamespace;

use crate::configure::ValidationError;
use crate::serialize::SerializeContext;

use super::custom::{build_xmp_namespace, merge_schemas, struct_types};
use super::xmp::{Category, Namespace, PropertyDescription, Schema, ValueType};

/// Registers a validation error for each property and structure field in
/// `schemas` whose value type PDF/A extension schemas can't express.
pub(super) fn check_pdfa_value_types(schemas: &[Cow<Schema>], sc: &mut SerializeContext) {
    fn is_expressible(value_type: &ValueType) -> bool {
        match value_type {
            ValueType::OrderedArray(item)
            | ValueType::UnorderedArray(item)
            | ValueType::AlternativeArray(item) => !matches!(
                **item,
                ValueType::OrderedArray(_)
                    | ValueType::UnorderedArray(_)
                    | ValueType::AlternativeArray(_)
                    | ValueType::LanguageAlternative
            ),
            _ => true,
        }
    }

    let mut check = |namespace: &Namespace, descriptions: &[PropertyDescription]| {
        for desc in descriptions {
            if !is_expressible(&desc.value_type) {
                sc.register_validation_error(ValidationError::UnsupportedXmpValueType {
                    namespace_uri: namespace.uri().to_string(),
                    property_name: desc.name.clone(),
                });
            }
        }
    };

    for schema in schemas {
        check(&schema.namespace, &schema.properties);
        for desc in &schema.properties {
            for ty in struct_types(&desc.value_type) {
                check(&ty.namespace, &ty.fields);
            }
        }
    }
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
    user: Vec<Cow<Schema>>,
) {
    for schema in merge_schemas(builtin.into_iter().map(Cow::Owned), user) {
        write_extension_schema(writer, &schema);
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
    use std::borrow::Cow;

    use xmp_writer::XmpWriter;

    use super::write_extension_schemas;
    use crate::metadata::xmp::{Namespace, PropertyDescription, Schema, SimpleType};

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
        write_extension_schemas(
            &mut xmp.extension_schemas(),
            vec![builtin],
            vec![Cow::Borrowed(&user)],
        );
        let xmp = xmp.finish(None);

        assert_eq!(xmp.matches("<pdfaSchema:namespaceURI>").count(), 1);
        assert_eq!(xmp.matches("<pdfaProperty:name>part<").count(), 1);
        assert_eq!(xmp.matches("<pdfaProperty:name>custom<").count(), 1);
    }
}
