//! RELAX NG schemas describing XMP metadata
//!
//! The simple type patterns are taken from the informative schemas of ISO 16684-2.

use std::borrow::Cow;

use xmlwriter::{Indent, Options, XmlWriter};

use super::xmp::{Namespace, Schema, SimpleType, StructType, ValueType};
use super::{build_xmp_namespace, predefined_struct_type};

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
        ui: ui_prefix(&namespaces),
    };

    grammar.w.write_declaration();
    grammar.w.start_element("grammar");
    grammar
        .w
        .write_attribute("xmlns", "http://relaxng.org/ns/structure/1.0");
    grammar
        .w
        .write_attribute("xmlns:rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#");
    let ui = grammar.ui.clone();
    grammar
        .w
        .write_attribute(&format!("xmlns:{ui}"), UI_NAMESPACE);
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

    grammar.element("start", |g| {
        g.element("element", |g| {
            g.w.write_attribute("name", "rdf:RDF");
            g.element("element", |g| {
                g.w.write_attribute("name", "rdf:Description");
                g.element("attribute", |g| g.w.write_attribute("name", "rdf:about"));
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
    ui: String,
}

impl<'a> Grammar<'a> {
    fn element(&mut self, name: &str, f: impl FnOnce(&mut Self)) {
        self.w.start_element(name);
        f(self);
        self.w.end_element();
    }

    fn define(&mut self, name: &str, f: impl FnOnce(&mut Self)) {
        self.element("define", |g| {
            g.w.write_attribute("name", &escape(name));
            g.w.write_attribute("combine", "choice");
            f(g);
        });
    }

    fn reference(&mut self, name: &str) {
        self.element("ref", |g| g.w.write_attribute("name", &escape(name)));
    }

    /// An `interleave` of the `len` patterns written by `f`, or
    /// `empty` if there are none.
    fn interleave(&mut self, len: usize, f: impl FnOnce(&mut Self)) {
        if len == 0 {
            self.element("empty", |_| {});
        } else {
            self.element("interleave", f);
        }
    }

    fn optional(&mut self, optional: bool, f: impl FnOnce(&mut Self)) {
        if optional {
            self.element("optional", f);
        } else {
            f(self);
        }
    }

    fn any_attribute(&mut self) {
        self.element("attribute", |g| g.element("anyName", |_| {}));
    }

    fn any_property(&mut self, prefix: &str, name: &str) {
        self.any = true;
        self.optional(true, |g| {
            g.element("element", |g| {
                g.w.write_attribute("name", &escape(&format!("{prefix}:{name}")));
                g.reference("Any");
            });
        });
    }

    fn ui_info(&mut self, label: &str, description: Option<&str>) {
        self.element(&format!("{}:label", self.ui), |g| {
            g.w.write_attribute("xml:lang", "x-default");
            g.w.write_attribute("value", &escape(label));
        });
        if let Some(description) = description {
            self.element(&format!("{}:description", self.ui), |g| {
                g.w.write_attribute("xml:lang", "x-default");
                g.w.write_attribute("value", &escape(description));
            });
        }
    }

    fn schema(&mut self, schema: &'a Schema, undescribed: &[&str]) {
        let prefix = prefix(&schema.namespace);

        self.define(&schema_define(&schema.namespace), |g| {
            g.element(&format!("{}:schema", g.ui), |g| {
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
                g.element("element", |g| {
                    g.w.write_attribute("name", &escape(&format!("{prefix}:{}", desc.name)));
                    g.value_type(&desc.value_type);
                    g.element(&format!("{}:property", g.ui), |g| {
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
            ValueType::ClosedChoice(_, values) => self.element("choice", |g| {
                for value in values {
                    g.element("value", |g| g.w.write_text(&escape(value)));
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
                    self.element("element", |g| {
                        g.w.write_attribute("name", "rdf:Description");
                        g.reference("Any");
                    });
                }
            },
            ValueType::Struct(ty) => self.struct_type(ty),
        }
    }

    fn array(&mut self, kind: &str, item: &'a ValueType) {
        self.element("element", |g| {
            g.w.write_attribute("name", kind);
            g.element("zeroOrMore", |g| {
                g.element("element", |g| {
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
        self.element("choice", |g| {
            g.reference(name);
            g.element("element", |g| {
                g.w.write_attribute("name", "rdf:Description");
                g.element("element", |g| {
                    g.w.write_attribute("name", "rdf:value");
                    g.reference(name);
                });
                g.element("zeroOrMore", |g| {
                    g.element("element", |g| {
                        g.element("anyName", |_| {});
                        g.element("text", |_| {});
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
                g.element("element", |g| {
                    g.w.write_attribute("name", "rdf:Description");
                    g.interleave(ty.fields.len(), |g| {
                        for field in &ty.fields {
                            g.optional(!field.required, |g| {
                                g.element("element", |g| {
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
                Some((data_type, pattern)) => g.element("data", |g| {
                    g.w.write_attribute("type", data_type);
                    if let Some(pattern) = pattern {
                        g.element("param", |g| {
                            g.w.write_attribute("name", "pattern");
                            g.w.write_text(pattern);
                        });
                    }
                }),
                None => g.element("text", |_| {}),
            });
        }

        if self.language_alternative {
            self.define("ISO16684-1.Types.QValue.LanguageAlternative", |g| {
                g.array_of_language_tagged_text();
            });
        }

        if self.any {
            self.define("Any", |g| {
                g.element("zeroOrMore", |g| {
                    g.element("choice", |g| {
                        g.any_attribute();
                        g.element("text", |_| {});
                        g.element("element", |g| {
                            g.element("anyName", |_| {});
                            g.reference("Any");
                        });
                    });
                });
            });
        }

        self.w.end_document()
    }

    fn array_of_language_tagged_text(&mut self) {
        self.element("element", |g| {
            g.w.write_attribute("name", "rdf:Alt");
            g.element("zeroOrMore", |g| {
                g.element("element", |g| {
                    g.w.write_attribute("name", "rdf:li");
                    g.element("attribute", |g| {
                        g.w.write_attribute("name", "xml:lang");
                        g.element("data", |g| {
                            g.w.write_attribute("type", "string");
                            g.element("param", |g| {
                                g.w.write_attribute("name", "pattern");
                                g.w.write_text("x-default|([a-zA-Z]+(-[a-zA-Z0-9]+)*)");
                            });
                        });
                    });
                    g.element("text", |_| {});
                });
            });
        });
    }
}

fn ui_prefix(namespaces: &[(String, &str)]) -> String {
    (0..)
        .map(|i| match i {
            0 => "ui".to_string(),
            i => format!("ui{i}"),
        })
        .find(|ui| namespaces.iter().all(|(prefix, _)| prefix != ui))
        .unwrap()
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

/// Escapes what xmlwriter doesn't: it only escapes quotes in attribute values
/// and `<` in text.
fn escape(text: &str) -> Cow<'_, str> {
    if !text.contains(['&', '<']) {
        return text.into();
    }

    text.replace('&', "&amp;").replace('<', "&lt;").into()
}
