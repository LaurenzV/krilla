use crate::canvas::{Canvas, CanvasPdfSerializer};
use crate::color::{GREY_ICC_DEFLATED, SRGB_ICC_DEFLATED};
use crate::ext_g_state::ExtGState;
use crate::mask::Mask;
use crate::paint::TilingPattern;
use crate::serialize::{CacheableObject, ObjectSerialize, SerializeSettings, SerializerContext};
use crate::shading::ShadingPattern;
use crate::util::{NameExt, RectExt};
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::sync::Arc;

pub struct ResourceDictionary {
    pub color_spaces: CsResourceMapper,
    pub ext_g_state: ExtGStateResourceMapper,
    pub patterns: PatternResourceMapper,
    pub x_objects: XObjectResourceMapper,
}

impl ResourceDictionary {
    pub fn new() -> Self {
        Self {
            color_spaces: CsResourceMapper::new(),
            ext_g_state: ExtGStateResourceMapper::new(),
            patterns: PatternResourceMapper::new(),
            x_objects: XObjectResourceMapper::new(),
        }
    }

    pub fn register_color_space(&mut self, color_space: PdfColorSpace) -> String {
        self.color_spaces.remap_with_name(color_space)
    }

    pub fn register_ext_g_state(&mut self, ext_state: ExtGState) -> String {
        self.ext_g_state.remap_with_name(ext_state)
    }

    pub fn register_pattern(&mut self, pdf_pattern: PdfPattern) -> String {
        self.patterns.remap_with_name(pdf_pattern)
    }

    pub fn register_x_object(&mut self, x_object: XObject) -> String {
        self.x_objects.remap_with_name(x_object)
    }

    pub fn to_pdf_resources(
        &self,
        sc: &mut SerializerContext,
        resources: &mut pdf_writer::writers::Resources,
    ) {
        let mut color_spaces = resources.color_spaces();
        for (name, entry) in self.color_spaces.get_entries() {
            color_spaces.pair(
                name.to_pdf_name(),
                sc.add_cached(CacheableObject::PdfColorSpace(entry)),
            );
        }
        color_spaces.finish();

        let mut ext_g_states = resources.ext_g_states();

        for (name, entry) in self.ext_g_state.get_entries() {
            ext_g_states.pair(
                name.to_pdf_name(),
                sc.add_cached(CacheableObject::ExtGState(entry)),
            );
        }
        ext_g_states.finish();

        let mut patterns = resources.patterns();

        for (name, entry) in self.patterns.get_entries() {
            patterns.pair(
                name.to_pdf_name(),
                sc.add_cached(CacheableObject::PdfPattern(entry)),
            );
        }
        patterns.finish();

        let mut x_objects = resources.x_objects();

        for (name, entry) in self.x_objects.get_entries() {
            x_objects.pair(name.to_pdf_name(), sc.add_uncached(entry));
        }

        x_objects.finish();
    }
}

pub trait PDFResource {
    fn get_name() -> &'static str;
}

pub type CsResourceMapper = ResourceMapper<PdfColorSpace>;
pub type ExtGStateResourceMapper = ResourceMapper<ExtGState>;
pub type PatternResourceMapper = ResourceMapper<PdfPattern>;
pub type XObjectResourceMapper = ResourceMapper<XObject>;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct XObject {
    pub canvas: Arc<Canvas>,
    pub isolated: bool,
    // Needed to circumvent (a potential) bug in Chrome.
    pub needs_transparency: bool
}

impl PDFResource for XObject {
    fn get_name() -> &'static str {
        "X"
    }
}

impl ObjectSerialize for XObject {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let srgb_ref = sc.add_cached(CacheableObject::PdfColorSpace(PdfColorSpace::SRGB));
        let mut chunk = Chunk::new();
        let mut resource_dictionary = ResourceDictionary::new();
        let (content_stream, bbox) = {
            let mut serializer = CanvasPdfSerializer::new(&mut resource_dictionary);
            serializer.serialize_instructions(self.canvas.byte_code.instructions());
            serializer.finish()
        };

        let mut x_object = chunk.form_xobject(root_ref, &content_stream);
        resource_dictionary.to_pdf_resources(sc, &mut x_object.resources());
        x_object.bbox(bbox.to_pdf_rect());

        if self.isolated || self.needs_transparency {
            x_object
                .group()
                .transparency()
                .isolated(self.isolated)
                .pair(Name(b"CS"), srgb_ref);
        }

        x_object.finish();

        sc.chunk_mut().extend(&chunk);
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum PdfPattern {
    ShadingPattern(ShadingPattern),
    TilingPattern(TilingPattern),
}

impl PDFResource for PdfPattern {
    fn get_name() -> &'static str {
        "P"
    }
}

impl ObjectSerialize for PdfPattern {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        match self {
            PdfPattern::ShadingPattern(sh) => sh.serialize_into(sc, root_ref),
            PdfPattern::TilingPattern(tp) => tp.serialize_into(sc, root_ref),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum PdfColorSpace {
    SRGB,
    D65Gray,
}

impl PDFResource for PdfColorSpace {
    fn get_name() -> &'static str {
        "C"
    }
}

impl ObjectSerialize for PdfColorSpace {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        match self {
            PdfColorSpace::SRGB => {
                let icc_ref = sc.new_ref();
                let mut array = sc.chunk_mut().indirect(root_ref).array();
                array.item(Name(b"ICCBased"));
                array.item(icc_ref);
                array.finish();

                sc.chunk_mut()
                    .icc_profile(icc_ref, &SRGB_ICC_DEFLATED)
                    .n(3)
                    .range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
                    .filter(pdf_writer::Filter::FlateDecode);
            }
            PdfColorSpace::D65Gray => {
                sc.chunk_mut()
                    .icc_profile(root_ref, &GREY_ICC_DEFLATED)
                    .n(1)
                    .range([0.0, 1.0])
                    .filter(pdf_writer::Filter::FlateDecode);
            }
        }
    }
}

pub struct ResourceMapper<V>
where
    V: Hash + Eq,
{
    forward: Vec<V>,
    backward: HashMap<V, ResourceNumber>,
    counter: ResourceNumber,
}

impl<V> ResourceMapper<V>
where
    V: Hash + Eq + Clone + PDFResource,
{
    pub fn new() -> Self {
        Self {
            forward: Vec::new(),
            backward: HashMap::new(),
            counter: 0,
        }
    }

    pub fn get(&self, resource: V) -> Option<ResourceNumber> {
        self.backward.get(&resource).copied()
    }

    pub fn get_with_name(&self, resource: V) -> Option<String> {
        self.get(resource).map(|u| Self::name_from_number(u))
    }

    pub fn remap(&mut self, resource: V) -> ResourceNumber {
        let forward = &mut self.forward;
        let backward = &mut self.backward;

        *backward.entry(resource.clone()).or_insert_with(|| {
            let old = forward.len();
            forward.push(resource.clone());
            old as ResourceNumber
        })
    }

    pub fn remap_with_name(&mut self, resource: V) -> String {
        Self::name_from_number(self.remap(resource))
    }

    fn name_from_number(num: ResourceNumber) -> String {
        format!("{}{}", V::get_name(), num)
    }

    pub fn get_entries(&self) -> impl Iterator<Item = (String, V)> + '_ {
        self.forward
            .iter()
            .enumerate()
            .map(|(i, r)| (Self::name_from_number(i as ResourceNumber), r.clone()))
    }
}

pub type ResourceNumber = u32;

#[cfg(test)]
mod tests {
    use crate::resource::{CsResourceMapper, PdfColorSpace};
    use crate::serialize::ObjectSerialize;

    #[test]
    fn test_cs_resource_mapper() {
        let mut mapper = CsResourceMapper::new();
        assert_eq!(mapper.remap(PdfColorSpace::SRGB), 0);
        assert_eq!(mapper.remap(PdfColorSpace::D65Gray), 1);
        assert_eq!(mapper.remap(PdfColorSpace::SRGB), 0);
        assert_eq!(
            mapper.remap_with_name(PdfColorSpace::SRGB),
            String::from("C0")
        );
        let items = mapper.get_entries().collect::<Vec<_>>();
        assert_eq!(items[0], (String::from("C0"), PdfColorSpace::SRGB));
        assert_eq!(items[1], (String::from("C1"), PdfColorSpace::D65Gray));
    }
}
