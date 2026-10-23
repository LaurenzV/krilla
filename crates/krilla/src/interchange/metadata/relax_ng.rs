//! RELAX NG schemas describing XMP metadata
//!
//! The simple type patterns are taken from the informative schemas of ISO 16684-2.

use std::borrow::Cow;

use xmlwriter::{Indent, Options, XmlWriter};

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
    let mut namespaces = Vec::new();
    for schema in schemas {
        add_namespace(&mut namespaces, &schema.namespace);
        for desc in &schema.properties {
            add_value_type_namespaces(&mut namespaces, &desc.value_type);
        }
    }
    for (ns, _) in undescribed {
        add_namespace(&mut namespaces, ns);
    }

    let mut grammar = Grammar {
        w: XmlWriter::new(Options {
            use_single_quote: false,
            indent: Indent::None,
            attributes_indent: Indent::None,
        }),
        simple_types: Vec::new(),
        struct_types: Vec::new(),
        language_alternative: false,
        any: !undescribed.is_empty(),
    };

    grammar.w.write_declaration();
    grammar.w.start_element("rng:grammar");
    grammar
        .w
        .write_attribute("xmlns:rng", "http://relaxng.org/ns/structure/1.0");
    grammar
        .w
        .write_attribute("xmlns:rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#");
    grammar.w.write_attribute("xmlns:x", "adobe:ns:meta/");
    grammar.w.write_attribute("xmlns:ui", UI_NAMESPACE);
    for (prefix, uri) in &namespaces {
        grammar
            .w
            .write_attribute(&format!("xmlns:{prefix}"), &escape(uri));
    }
    grammar.w.write_attribute(
        "datatypeLibrary",
        "http://www.w3.org/2001/XMLSchema-datatypes",
    );

    let orphans: Vec<_> = undescribed
        .iter()
        .filter(|(ns, _)| !schemas.iter().any(|s| s.namespace.uri() == ns.uri()))
        .collect();

    grammar.element("rng:start", |g| {
        g.element("rng:element", |g| {
            g.w.write_attribute("name", "x:xmpmeta");
            g.element("rng:zeroOrMore", |g| g.any_attribute());
            g.element("rng:element", |g| {
                g.w.write_attribute("name", "rdf:RDF");
                g.element("rng:element", |g| {
                    g.w.write_attribute("name", "rdf:Description");
                    g.element("rng:attribute", |g| {
                        g.w.write_attribute("name", "rdf:about")
                    });
                    g.interleave(schemas.len() + orphans.len(), |g| {
                        for schema in schemas {
                            g.reference(&schema_define(&schema.namespace));
                        }
                        for (ns, name) in &orphans {
                            g.any_property(&prefix(ns), name);
                        }
                    });
                });
            });
        });
    });

    for schema in schemas {
        let extra: Vec<_> = undescribed
            .iter()
            .filter(|(ns, _)| ns.uri() == schema.namespace.uri())
            .map(|(_, name)| *name)
            .collect();
        grammar.schema(schema, &extra);
    }

    grammar.finish()
}

struct Grammar<'a> {
    w: XmlWriter,
    simple_types: Vec<SimpleType>,
    struct_types: Vec<&'a StructType>,
    language_alternative: bool,
    any: bool,
}

impl<'a> Grammar<'a> {
    fn element(&mut self, name: &str, f: impl FnOnce(&mut Self)) {
        self.w.start_element(name);
        f(self);
        self.w.end_element();
    }

    fn define(&mut self, name: &str, f: impl FnOnce(&mut Self)) {
        self.element("rng:define", |g| {
            g.w.write_attribute("name", &escape(name));
            g.w.write_attribute("combine", "choice");
            f(g);
        });
    }

    fn reference(&mut self, name: &str) {
        self.element("rng:ref", |g| g.w.write_attribute("name", &escape(name)));
    }

    /// An `rng:interleave` of the `len` patterns written by `f`, or
    /// `rng:empty` if there are none.
    fn interleave(&mut self, len: usize, f: impl FnOnce(&mut Self)) {
        if len == 0 {
            self.element("rng:empty", |_| {});
        } else {
            self.element("rng:interleave", f);
        }
    }

    fn optional(&mut self, optional: bool, f: impl FnOnce(&mut Self)) {
        if optional {
            self.element("rng:optional", f);
        } else {
            f(self);
        }
    }

    fn any_attribute(&mut self) {
        self.element("rng:attribute", |g| g.element("rng:anyName", |_| {}));
    }

    fn any_property(&mut self, prefix: &str, name: &str) {
        self.any = true;
        self.optional(true, |g| {
            g.element("rng:element", |g| {
                g.w.write_attribute("name", &escape(&format!("{prefix}:{name}")));
                g.reference("Any");
            });
        });
    }

    fn ui_info(&mut self, label: &str, description: Option<&str>) {
        self.element("ui:label", |g| {
            g.w.write_attribute("xml:lang", "x-default");
            g.w.write_attribute("value", &escape(label));
        });
        if let Some(description) = description {
            self.element("ui:description", |g| {
                g.w.write_attribute("xml:lang", "x-default");
                g.w.write_attribute("value", &escape(description));
            });
        }
    }

    fn schema(&mut self, schema: &'a Schema, undescribed: &[&str]) {
        let prefix = prefix(&schema.namespace);

        self.define(&schema_define(&schema.namespace), |g| {
            g.element("ui:schema", |g| {
                let label = schema.name.as_deref().unwrap_or(&prefix);
                g.ui_info(label, schema.description.as_deref());
            });
            g.interleave(schema.properties.len() + undescribed.len(), |g| {
                for desc in &schema.properties {
                    g.optional(!desc.required, |g| {
                        g.reference(&format!("{prefix}.{}", desc.name))
                    });
                }
                for name in undescribed {
                    g.any_property(&prefix, name);
                }
            });
        });

        for desc in &schema.properties {
            self.define(&format!("{prefix}.{}", desc.name), |g| {
                g.element("rng:element", |g| {
                    g.w.write_attribute("name", &escape(&format!("{prefix}:{}", desc.name)));
                    g.value_type(&desc.value_type);
                    g.element("ui:property", |g| {
                        g.ui_info(&desc.name, Some(&desc.description))
                    });
                });
            });
        }
    }

    fn value_type(&mut self, value_type: &'a ValueType) {
        match value_type {
            ValueType::Simple(ty) | ValueType::OpenChoice(ty, _) => {
                if !self.simple_types.contains(ty) {
                    self.simple_types.push(*ty);
                }
                self.reference(&format!("ISO16684-1.Types.QValue.{}", ty.as_str()));
            }
            ValueType::ClosedChoice(_, values) => self.element("rng:choice", |g| {
                for value in values {
                    g.element("rng:value", |g| g.w.write_text(&escape(value)));
                }
            }),
            ValueType::OrderedArray(item) => self.array("rdf:Seq", item),
            ValueType::UnorderedArray(item) => self.array("rdf:Bag", item),
            ValueType::AlternativeArray(item) => self.array("rdf:Alt", item),
            ValueType::LanguageAlternative => {
                self.language_alternative = true;
                self.reference("ISO16684-1.Types.QValue.LanguageAlternative");
            }
            ValueType::PredefinedStruct(ty) => match predefined_struct_type(*ty) {
                Some(ty) => self.struct_type(ty),
                None => {
                    self.any = true;
                    self.element("rng:element", |g| {
                        g.w.write_attribute("name", "rdf:Description");
                        g.reference("Any");
                    });
                }
            },
            ValueType::Struct(ty) => self.struct_type(ty),
        }
    }

    fn array(&mut self, kind: &str, item: &'a ValueType) {
        self.element("rng:element", |g| {
            g.w.write_attribute("name", kind);
            g.element("rng:zeroOrMore", |g| {
                g.element("rng:element", |g| {
                    g.w.write_attribute("name", "rdf:li");
                    g.value_type(item);
                });
            });
        });
    }

    /// References the structure type, whose definition is written in
    /// [`Self::finish`].
    fn struct_type(&mut self, ty: &'a StructType) {
        if !self.struct_types.contains(&ty) {
            self.struct_types.push(ty);
        }
        self.reference(&format!("{}.QValue", struct_define(ty)));
    }

    /// The base type `name`, or the same value in an `rdf:Description` with
    /// qualifiers.
    fn qualified(&mut self, name: &str) {
        self.element("rng:choice", |g| {
            g.reference(name);
            g.element("rng:element", |g| {
                g.w.write_attribute("name", "rdf:Description");
                g.element("rng:element", |g| {
                    g.w.write_attribute("name", "rdf:value");
                    g.reference(name);
                });
                g.element("rng:zeroOrMore", |g| {
                    g.element("rng:element", |g| {
                        g.element("rng:anyName", |_| {});
                        g.element("rng:text", |_| {});
                    });
                });
            });
        });
    }

    fn finish(mut self) -> String {
        // Writing a structure type can discover further ones.
        let mut i = 0;
        while let Some(&ty) = self.struct_types.get(i) {
            let name = struct_define(ty);
            let base = format!("{name}.Base");
            let prefix = prefix(&ty.namespace);

            self.define(&format!("{name}.QValue"), |g| g.qualified(&base));
            self.define(&base, |g| {
                g.element("rng:element", |g| {
                    g.w.write_attribute("name", "rdf:Description");
                    g.interleave(ty.fields.len(), |g| {
                        for field in &ty.fields {
                            g.optional(!field.required, |g| {
                                g.element("rng:element", |g| {
                                    g.w.write_attribute(
                                        "name",
                                        &escape(&format!("{prefix}:{}", field.name)),
                                    );
                                    g.value_type(&field.value_type);
                                });
                            });
                        }
                    });
                });
            });
            i += 1;
        }

        for ty in std::mem::take(&mut self.simple_types) {
            let base = format!("ISO16684-1.Types.Base.{}", ty.as_str());
            self.define(&format!("ISO16684-1.Types.QValue.{}", ty.as_str()), |g| {
                g.qualified(&base)
            });
            self.define(&base, |g| match simple_type_data(ty) {
                Some((data_type, pattern)) => g.element("rng:data", |g| {
                    g.w.write_attribute("type", data_type);
                    if let Some(pattern) = pattern {
                        g.element("rng:param", |g| {
                            g.w.write_attribute("name", "pattern");
                            g.w.write_text(pattern);
                        });
                    }
                }),
                None => g.element("rng:text", |_| {}),
            });
        }

        if self.language_alternative {
            self.define("ISO16684-1.Types.QValue.LanguageAlternative", |g| {
                g.array_of_language_tagged_text();
            });
        }

        if self.any {
            self.define("Any", |g| {
                g.element("rng:zeroOrMore", |g| {
                    g.element("rng:choice", |g| {
                        g.any_attribute();
                        g.element("rng:text", |_| {});
                        g.element("rng:element", |g| {
                            g.element("rng:anyName", |_| {});
                            g.reference("Any");
                        });
                    });
                });
            });
        }

        self.w.end_document()
    }

    fn array_of_language_tagged_text(&mut self) {
        self.element("rng:element", |g| {
            g.w.write_attribute("name", "rdf:Alt");
            g.element("rng:zeroOrMore", |g| {
                g.element("rng:element", |g| {
                    g.w.write_attribute("name", "rdf:li");
                    g.element("rng:attribute", |g| {
                        g.w.write_attribute("name", "xml:lang");
                        g.element("rng:data", |g| {
                            g.w.write_attribute("type", "string");
                            g.element("rng:param", |g| {
                                g.w.write_attribute("name", "pattern");
                                g.w.write_text("x-default|([a-zA-Z]+(-[a-zA-Z0-9]+)*)");
                            });
                        });
                    });
                    g.element("rng:text", |_| {});
                });
            });
        });
    }
}

fn prefix(ns: &Namespace) -> String {
    build_xmp_namespace(ns).prefix().to_string()
}

fn add_namespace<'a>(namespaces: &mut Vec<(String, &'a str)>, ns: &'a Namespace) {
    if !namespaces.iter().any(|(_, uri)| *uri == ns.uri()) {
        namespaces.push((prefix(ns), ns.uri()));
    }
}

fn add_value_type_namespaces<'a>(namespaces: &mut Vec<(String, &'a str)>, ty: &'a ValueType) {
    let struct_type = match ty {
        ValueType::OrderedArray(item)
        | ValueType::UnorderedArray(item)
        | ValueType::AlternativeArray(item) => return add_value_type_namespaces(namespaces, item),
        ValueType::PredefinedStruct(ty) => predefined_struct_type(*ty),
        ValueType::Struct(ty) => Some(&**ty),
        _ => None,
    };

    if let Some(ty) = struct_type {
        if !namespaces.iter().any(|(_, uri)| *uri == ty.namespace.uri()) {
            add_namespace(namespaces, &ty.namespace);
            for field in &ty.fields {
                add_value_type_namespaces(namespaces, &field.value_type);
            }
        }
    }
}

fn schema_define(ns: &Namespace) -> String {
    format!("XMP_Properties-{}", prefix(ns))
}

fn struct_define(ty: &StructType) -> String {
    format!("{}.Types.{}", prefix(&ty.namespace), ty.name)
}

/// The XML Schema datatype and pattern of a simple type, or `None` for
/// unconstrained text.
fn simple_type_data(ty: SimpleType) -> Option<(&'static str, Option<&'static str>)> {
    match ty {
        SimpleType::Boolean => Some(("string", Some("True|False"))),
        SimpleType::Integer => Some(("integer", None)),
        SimpleType::Real => Some(("double", None)),
        SimpleType::Rational => Some(("string", Some(r"-?\d+/[1-9]\d*"))),
        SimpleType::Date => Some((
            "string",
            Some(concat!(
                r"(\d{4}((-)?(0[1-9]|1[0-2])((-)?(0[1-9]|[1-2][0-9]|3[0-1])(T(24:00(:00(\.[0]+)?)?|",
                r"(([0-1][0-9]|2[0-3])(:)[0-5][0-9])((:)[0-5][0-9](\.[\d]+)?)?)((\+|-)(14:00|",
                r"(0[0-9]|1[0-3])(:)[0-5][0-9])|Z)?)?)?)?)",
            )),
        )),
        SimpleType::Locale => Some(("string", Some("[a-zA-Z]+(-[a-zA-Z0-9]+)*"))),
        SimpleType::MimeType => Some((
            "string",
            Some(concat!(
                "(application|audio|example|image|message|model|multipart|text|video)",
                r"/[a-zA-Z0-9]+([+\-.][a-zA-Z0-9]+)*",
            )),
        )),
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
        | SimpleType::XPath => None,
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

/// Escapes what xmlwriter doesn't: it only escapes quotes in attribute values
/// and `<` in text.
fn escape(text: &str) -> Cow<'_, str> {
    if !text.contains(['&', '<']) {
        return text.into();
    }

    text.replace('&', "&amp;").replace('<', "&lt;").into()
}
