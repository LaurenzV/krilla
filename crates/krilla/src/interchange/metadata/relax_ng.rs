//! RELAX NG schemas describing XMP metadata
//!
//! The simple type patterns are taken from the informative schemas of ISO 16684-2.

use std::borrow::Cow;
use std::fmt::Write;

use super::build_xmp_namespace;
use super::xmp::{
    Namespace, PredefinedStruct, PropertyDescription, Schema, SimpleType, StructType, ValueType,
};

const UI_NAMESPACE: &str = "http://ns.iso.org/iso-16684-2/xmp-schema-ui-info/1.0";

/// A RELAX NG schema for a packet with the properties described by `schemas`
/// and the `undescribed` properties, which may have any content.
pub(crate) fn relax_ng_schema(
    schemas: &[Cow<Schema>],
    undescribed: &[(&Namespace, &str)],
) -> String {
    let mut grammar = Grammar::default();
    let mut start = String::new();

    for schema in schemas {
        let uri = schema.namespace.uri();
        let extra: Vec<_> = undescribed
            .iter()
            .filter(|(ns, _)| ns.uri() == uri)
            .map(|(_, name)| *name)
            .collect();
        let name = grammar.schema(schema, &extra);
        write!(start, r#"<rng:ref name="{name}"/>"#).unwrap();
    }

    for (ns, name) in undescribed {
        if !schemas.iter().any(|s| s.namespace.uri() == ns.uri()) {
            let prefix = grammar.namespace(ns);
            grammar.any = true;
            write!(
                start,
                r#"<rng:optional><rng:element name="{prefix}:{}"><rng:ref name="Any"/></rng:element></rng:optional>"#,
                escape(name)
            )
            .unwrap();
        }
    }

    grammar.finish(&start)
}

#[derive(Default)]
struct Grammar<'a> {
    defines: String,
    namespaces: Vec<(String, &'a str)>,
    simple_types: Vec<SimpleType>,
    struct_types: Vec<&'a StructType>,
    language_alternative: bool,
    any: bool,
}

impl<'a> Grammar<'a> {
    fn namespace(&mut self, ns: &'a Namespace) -> String {
        let prefix = build_xmp_namespace(ns).prefix().to_string();
        if !self.namespaces.iter().any(|(_, uri)| *uri == ns.uri()) {
            self.namespaces.push((prefix.clone(), ns.uri()));
        }
        prefix
    }

    fn schema(&mut self, schema: &'a Schema, undescribed: &[&str]) -> String {
        let prefix = self.namespace(&schema.namespace);
        let define = format!("XMP_Properties-{prefix}");

        let mut interleave = String::new();
        for desc in &schema.properties {
            let name = self.property(&prefix, desc);
            if desc.required {
                write!(interleave, r#"<rng:ref name="{name}"/>"#).unwrap();
            } else {
                write!(
                    interleave,
                    r#"<rng:optional><rng:ref name="{name}"/></rng:optional>"#
                )
                .unwrap();
            }
        }

        for name in undescribed {
            self.any = true;
            write!(
                interleave,
                r#"<rng:optional><rng:element name="{prefix}:{}"><rng:ref name="Any"/></rng:element></rng:optional>"#,
                escape(name)
            )
            .unwrap();
        }

        let label = schema.name.as_deref().unwrap_or(&prefix);
        write!(
            self.defines,
            r#"<rng:define name="{define}" combine="choice"><ui:schema>{}</ui:schema>{}</rng:define>"#,
            ui_info(label, schema.description.as_deref()),
            interleaved(&interleave),
        )
        .unwrap();

        define
    }

    fn property(&mut self, prefix: &str, desc: &'a PropertyDescription) -> String {
        let name = escape(&desc.name);
        let define = format!("{prefix}.{name}");
        let pattern = self.value_type(&desc.value_type);
        write!(
            self.defines,
            r#"<rng:define name="{define}" combine="choice"><rng:element name="{prefix}:{name}">{pattern}<ui:property>{}</ui:property></rng:element></rng:define>"#,
            ui_info(&desc.name, Some(&desc.description)),
        )
        .unwrap();
        define
    }

    fn value_type(&mut self, value_type: &'a ValueType) -> String {
        match value_type {
            ValueType::Simple(ty) | ValueType::OpenChoice(ty, _) => {
                if !self.simple_types.contains(ty) {
                    self.simple_types.push(*ty);
                }
                format!(
                    r#"<rng:ref name="ISO16684-1.Types.QValue.{}"/>"#,
                    ty.as_str()
                )
            }
            ValueType::ClosedChoice(_, values) => {
                let mut choice = String::from("<rng:choice>");
                for value in values {
                    write!(choice, "<rng:value>{}</rng:value>", escape(value)).unwrap();
                }
                choice.push_str("</rng:choice>");
                choice
            }
            ValueType::OrderedArray(item) => self.array("rdf:Seq", item),
            ValueType::UnorderedArray(item) => self.array("rdf:Bag", item),
            ValueType::AlternativeArray(item) => self.array("rdf:Alt", item),
            ValueType::LanguageAlternative => {
                self.language_alternative = true;
                r#"<rng:ref name="ISO16684-1.Types.QValue.LanguageAlternative"/>"#.into()
            }
            ValueType::PredefinedStruct(ty) => match predefined_struct_type(*ty) {
                Some(ty) => self.struct_type(ty),
                None => {
                    self.any = true;
                    r#"<rng:element name="rdf:Description"><rng:ref name="Any"/></rng:element>"#
                        .into()
                }
            },
            ValueType::Struct(ty) => self.struct_type(ty),
        }
    }

    fn array(&mut self, kind: &str, item: &'a ValueType) -> String {
        let item = self.value_type(item);
        format!(
            r#"<rng:element name="{kind}"><rng:zeroOrMore><rng:element name="rdf:li">{item}</rng:element></rng:zeroOrMore></rng:element>"#
        )
    }

    fn struct_type(&mut self, ty: &'a StructType) -> String {
        let prefix = self.namespace(&ty.namespace);
        let name = format!("{prefix}.Types.{}", escape(&ty.name));
        if !self.struct_types.contains(&ty) {
            self.struct_types.push(ty);

            let mut fields = String::new();
            for field in &ty.fields {
                let pattern = self.value_type(&field.value_type);
                let element = format!(
                    r#"<rng:element name="{prefix}:{}">{pattern}</rng:element>"#,
                    escape(&field.name)
                );
                if field.required {
                    fields.push_str(&element);
                } else {
                    write!(fields, "<rng:optional>{element}</rng:optional>").unwrap();
                }
            }

            write!(
                self.defines,
                r#"<rng:define name="{name}.QValue" combine="choice">{}</rng:define><rng:define name="{name}.Base" combine="choice"><rng:element name="rdf:Description">{}</rng:element></rng:define>"#,
                qualified(&format!("{name}.Base")),
                interleaved(&fields),
            )
            .unwrap();
        }

        format!(r#"<rng:ref name="{name}.QValue"/>"#)
    }

    fn finish(mut self, start: &str) -> String {
        for ty in std::mem::take(&mut self.simple_types) {
            let name = format!("ISO16684-1.Types.Base.{}", ty.as_str());
            write!(
                self.defines,
                r#"<rng:define name="ISO16684-1.Types.QValue.{}" combine="choice">{}</rng:define><rng:define name="{name}" combine="choice">{}</rng:define>"#,
                ty.as_str(),
                qualified(&name),
                simple_type_pattern(ty),
            )
            .unwrap();
        }

        if self.language_alternative {
            self.defines.push_str(concat!(
                r#"<rng:define name="ISO16684-1.Types.QValue.LanguageAlternative" combine="choice">"#,
                r#"<rng:element name="rdf:Alt"><rng:zeroOrMore><rng:element name="rdf:li">"#,
                r#"<rng:attribute name="xml:lang"><rng:data type="string">"#,
                r#"<rng:param name="pattern">x-default|([a-zA-Z]+(-[a-zA-Z0-9]+)*)</rng:param>"#,
                r#"</rng:data></rng:attribute><rng:text/>"#,
                r#"</rng:element></rng:zeroOrMore></rng:element></rng:define>"#,
            ));
        }

        if self.any {
            self.defines.push_str(concat!(
                r#"<rng:define name="Any" combine="choice"><rng:zeroOrMore><rng:choice>"#,
                r#"<rng:attribute><rng:anyName/></rng:attribute><rng:text/>"#,
                r#"<rng:element><rng:anyName/><rng:ref name="Any"/></rng:element>"#,
                r#"</rng:choice></rng:zeroOrMore></rng:define>"#,
            ));
        }

        let mut out = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        out.push_str(concat!(
            r#"<rng:grammar xmlns:rng="http://relaxng.org/ns/structure/1.0""#,
            r#" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#""#,
            r#" xmlns:x="adobe:ns:meta/""#,
            r#" datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes""#,
        ));
        write!(out, r#" xmlns:ui="{UI_NAMESPACE}""#).unwrap();
        for (prefix, uri) in &self.namespaces {
            write!(out, r#" xmlns:{prefix}="{}""#, escape(uri)).unwrap();
        }
        out.push('>');

        write!(
            out,
            concat!(
                r#"<rng:start><rng:element name="x:xmpmeta">"#,
                r#"<rng:zeroOrMore><rng:attribute><rng:anyName/></rng:attribute></rng:zeroOrMore>"#,
                r#"<rng:element name="rdf:RDF"><rng:element name="rdf:Description">"#,
                r#"<rng:attribute name="rdf:about"/>{}"#,
                r#"</rng:element></rng:element></rng:element></rng:start>"#,
            ),
            interleaved(start),
        )
        .unwrap();

        out.push_str(&self.defines);
        out.push_str("</rng:grammar>");
        out
    }
}

/// The base type `name`, or the same value in an `rdf:Description` with
/// qualifiers.
fn qualified(name: &str) -> String {
    format!(
        r#"<rng:choice><rng:ref name="{name}"/><rng:element name="rdf:Description"><rng:element name="rdf:value"><rng:ref name="{name}"/></rng:element><rng:zeroOrMore><rng:element><rng:anyName/><rng:text/></rng:element></rng:zeroOrMore></rng:element></rng:choice>"#
    )
}

fn interleaved(patterns: &str) -> String {
    if patterns.is_empty() {
        "<rng:empty/>".into()
    } else {
        format!("<rng:interleave>{patterns}</rng:interleave>")
    }
}

fn ui_info(label: &str, description: Option<&str>) -> String {
    let mut info = format!(
        r#"<ui:label xml:lang="x-default" value="{}"/>"#,
        escape(label)
    );
    if let Some(description) = description {
        write!(
            info,
            r#"<ui:description xml:lang="x-default" value="{}"/>"#,
            escape(description)
        )
        .unwrap();
    }
    info
}

fn simple_type_pattern(ty: SimpleType) -> &'static str {
    match ty {
        SimpleType::Boolean => {
            r#"<rng:data type="string"><rng:param name="pattern">True|False</rng:param></rng:data>"#
        }
        SimpleType::Integer => r#"<rng:data type="integer"/>"#,
        SimpleType::Real => r#"<rng:data type="double"/>"#,
        SimpleType::Rational => {
            r#"<rng:data type="string"><rng:param name="pattern">-?\d+/[1-9]\d*</rng:param></rng:data>"#
        }
        SimpleType::Date => concat!(
            r#"<rng:data type="string"><rng:param name="pattern">"#,
            r#"(\d{4}((-)?(0[1-9]|1[0-2])((-)?(0[1-9]|[1-2][0-9]|3[0-1])(T(24:00(:00(\.[0]+)?)?|(([0-1][0-9]|2[0-3])(:)[0-5][0-9])((:)[0-5][0-9](\.[\d]+)?)?)((\+|-)(14:00|(0[0-9]|1[0-3])(:)[0-5][0-9])|Z)?)?)?)?)"#,
            r#"</rng:param></rng:data>"#,
        ),
        SimpleType::Locale => {
            r#"<rng:data type="string"><rng:param name="pattern">[a-zA-Z]+(-[a-zA-Z0-9]+)*</rng:param></rng:data>"#
        }
        SimpleType::MimeType => concat!(
            r#"<rng:data type="string"><rng:param name="pattern">"#,
            r#"(application|audio|example|image|message|model|multipart|text|video)/[a-zA-Z0-9]+([+\-.][a-zA-Z0-9]+)*"#,
            r#"</rng:param></rng:data>"#,
        ),
        SimpleType::Text
        | SimpleType::AgentName
        | SimpleType::Guid
        | SimpleType::ProperName
        | SimpleType::RenditionClass
        | SimpleType::Uri
        | SimpleType::Url
        | SimpleType::FrameRate
        | SimpleType::FrameCount
        | SimpleType::Part
        | SimpleType::XPath => "<rng:text/>",
    }
}

/// The fields of the predefined structure types, as listed in TechNote 0008,
/// chapter 3. Returns `None` for types whose fields aren't known.
fn predefined_struct_type(ty: PredefinedStruct) -> Option<&'static StructType> {
    use std::sync::LazyLock;

    fn struct_type(
        prefix: &str,
        uri: &str,
        name: &str,
        fields: &[(&str, ValueType)],
    ) -> StructType {
        fields.iter().fold(
            StructType::new(Namespace::new(prefix, uri), name, ""),
            |ty, (name, value_type)| {
                ty.field(PropertyDescription::new(*name, value_type.clone(), ""))
            },
        )
    }

    static TYPES: LazyLock<[StructType; 6]> = LazyLock::new(|| {
        use SimpleType::*;
        let open = |ty| ValueType::OpenChoice(ty, vec![]);
        [
            struct_type(
                "stDim",
                "http://ns.adobe.com/xap/1.0/sType/Dimensions#",
                "Dimensions",
                &[("w", Real.into()), ("h", Real.into()), ("unit", open(Text))],
            ),
            struct_type(
                "xmpGImg",
                "http://ns.adobe.com/xap/1.0/g/img/",
                "Thumbnail",
                &[
                    ("height", Integer.into()),
                    ("width", Integer.into()),
                    ("format", open(Text)),
                    ("image", Text.into()),
                ],
            ),
            struct_type(
                "stEvt",
                "http://ns.adobe.com/xap/1.0/sType/ResourceEvent#",
                "ResourceEvent",
                &[
                    ("action", open(Text)),
                    ("instanceID", Uri.into()),
                    ("parameters", Text.into()),
                    ("softwareAgent", AgentName.into()),
                    ("when", Date.into()),
                ],
            ),
            struct_type(
                "stRef",
                "http://ns.adobe.com/xap/1.0/sType/ResourceRef#",
                "ResourceRef",
                &[
                    ("instanceID", Uri.into()),
                    ("documentID", Uri.into()),
                    ("versionID", Text.into()),
                    ("renditionClass", RenditionClass.into()),
                    ("renditionParams", Text.into()),
                    ("manager", AgentName.into()),
                    ("managerVariant", Text.into()),
                    ("manageTo", Uri.into()),
                    ("manageUI", Uri.into()),
                ],
            ),
            struct_type(
                "stVer",
                "http://ns.adobe.com/xap/1.0/sType/Version#",
                "Version",
                &[
                    ("comments", Text.into()),
                    ("event", PredefinedStruct::ResourceEvent.into()),
                    ("modifyDate", Date.into()),
                    ("modifier", ProperName.into()),
                    ("version", Text.into()),
                ],
            ),
            struct_type(
                "stJob",
                "http://ns.adobe.com/xap/1.0/sType/Job#",
                "Job",
                &[
                    ("name", Text.into()),
                    ("id", Text.into()),
                    ("url", Url.into()),
                ],
            ),
        ]
    });

    let index = match ty {
        PredefinedStruct::Dimensions => 0,
        PredefinedStruct::Thumbnail => 1,
        PredefinedStruct::ResourceEvent => 2,
        PredefinedStruct::ResourceRef => 3,
        PredefinedStruct::Version => 4,
        PredefinedStruct::Job => 5,
        PredefinedStruct::Colorant | PredefinedStruct::Font => return None,
    };

    Some(&TYPES[index])
}

fn escape(text: &str) -> Cow<'_, str> {
    if !text.contains(['&', '<', '>', '"']) {
        return text.into();
    }

    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .into()
}
