use crate::object::color_space::ColorSpace;
use crate::object::ext_g_state::ExtGState;
use crate::object::image::Image;
use crate::object::shading_pattern::ShadingPattern;
use crate::object::tiling_pattern::TilingPattern;
use crate::object::xobject::XObject;
use crate::serialize::{Object, SerializeSettings, SerializerContext};
use crate::util::{NameExt, RectExt};
use pdf_writer::writers::Resources;
use pdf_writer::{Chunk, Dict, Finish, Name, Ref};
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

trait ResourceTrait: Object {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a>;
    fn get_prefix() -> &'static str;
}

impl ResourceTrait for ExtGState {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.ext_g_states()
    }

    fn get_prefix() -> &'static str {
        "g"
    }
}

impl ResourceTrait for ColorSpace {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.color_spaces()
    }

    fn get_prefix() -> &'static str {
        "c"
    }
}

#[derive(Hash, Clone, Eq, PartialEq)]
pub enum Resource {
    XObject(XObjectResource),
    Pattern(PatternResource),
    ExtGState(ExtGState),
    ColorSpace(ColorSpace),
}

impl Object for Resource {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        match self {
            Resource::XObject(x) => x.serialize_into(sc, root_ref),
            Resource::Pattern(p) => p.serialize_into(sc, root_ref),
            Resource::ExtGState(e) => e.serialize_into(sc, root_ref),
            Resource::ColorSpace(x) => x.serialize_into(sc, root_ref),
        }
    }

    fn is_cached(&self) -> bool {
        match self {
            Resource::XObject(x) => x.is_cached(),
            Resource::Pattern(p) => p.is_cached(),
            Resource::ExtGState(e) => e.is_cached(),
            Resource::ColorSpace(x) => x.is_cached(),
        }
    }
}

#[derive(Hash, Clone, Eq, PartialEq)]
pub enum XObjectResource {
    XObject(XObject),
    Image(Image),
}

impl ResourceTrait for XObjectResource {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.x_objects()
    }

    fn get_prefix() -> &'static str {
        "x"
    }
}

impl Object for XObjectResource {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        match self {
            XObjectResource::XObject(x) => x.serialize_into(sc, root_ref),
            XObjectResource::Image(i) => i.serialize_into(sc, root_ref),
        }
    }

    fn is_cached(&self) -> bool {
        match self {
            XObjectResource::XObject(x) => x.is_cached(),
            XObjectResource::Image(i) => i.is_cached(),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum PatternResource {
    ShadingPattern(ShadingPattern),
    TiledPattern(TilingPattern),
}

impl ResourceTrait for PatternResource {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.patterns()
    }

    fn get_prefix() -> &'static str {
        "p"
    }
}

impl Object for PatternResource {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        match self {
            PatternResource::ShadingPattern(sp) => sp.serialize_into(sc, root_ref),
            PatternResource::TiledPattern(tp) => tp.serialize_into(sc, root_ref),
        }
    }

    fn is_cached(&self) -> bool {
        match self {
            PatternResource::ShadingPattern(sp) => sp.is_cached(),
            PatternResource::TiledPattern(tp) => tp.is_cached(),
        }
    }
}

pub struct ResourceDictionary {
    pub color_spaces: ResourceMapper<ColorSpace>,
    pub ext_g_states: ResourceMapper<ExtGState>,
    pub patterns: ResourceMapper<PatternResource>,
    pub x_objects: ResourceMapper<XObjectResource>,
}

impl ResourceDictionary {
    pub fn new() -> Self {
        Self {
            color_spaces: ResourceMapper::new(),
            ext_g_states: ResourceMapper::new(),
            patterns: ResourceMapper::new(),
            x_objects: ResourceMapper::new(),
        }
    }

    fn register_color_space(&mut self, color_space: ColorSpace) -> String {
        self.color_spaces.remap_with_name(color_space)
    }

    fn register_ext_g_state(&mut self, ext_state: ExtGState) -> String {
        self.ext_g_states.remap_with_name(ext_state)
    }

    fn register_pattern(&mut self, pdf_pattern: PatternResource) -> String {
        self.patterns.remap_with_name(pdf_pattern)
    }

    fn register_x_object(&mut self, x_object: XObjectResource) -> String {
        self.x_objects.remap_with_name(x_object)
    }

    pub fn register_resource(&mut self, resource: Resource) -> String {
        match resource {
            Resource::XObject(x) => self.register_x_object(x),
            Resource::Pattern(p) => self.register_pattern(p),
            Resource::ExtGState(e) => self.register_ext_g_state(e),
            Resource::ColorSpace(c) => self.register_color_space(c),
        }
    }

    pub fn to_pdf_resources(&self, sc: &mut SerializerContext, resources: &mut Resources) {
        write_resource_type(sc, resources, &self.color_spaces);
        write_resource_type(sc, resources, &self.ext_g_states);
        write_resource_type(sc, resources, &self.patterns);
        write_resource_type(sc, resources, &self.x_objects);
    }
}

fn write_resource_type<T>(
    sc: &mut SerializerContext,
    resources: &mut pdf_writer::writers::Resources,
    resource_mapper: &ResourceMapper<T>,
) where
    T: Hash + Eq + ResourceTrait,
{
    if resource_mapper.len() > 0 {
        let mut dict = T::get_dict(resources);

        for (name, entry) in resource_mapper.get_entries() {
            dict.pair(name.to_pdf_name(), sc.add(entry.clone()));
        }

        dict.finish();
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
    V: Hash + Eq + Clone + ResourceTrait,
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
        format!("{}{}", V::get_prefix(), num)
    }

    pub fn len(&self) -> u32 {
        self.counter
    }

    pub fn get_entries(&self) -> impl Iterator<Item = (String, V)> + '_ {
        self.forward
            .iter()
            .enumerate()
            .map(|(i, r)| (Self::name_from_number(i as ResourceNumber), r.clone()))
    }
}

pub type ResourceNumber = u32;

// #[cfg(test)]
// mod tests {
//     use crate::resource::{CsResourceMapper, PdfColorSpace};
//     use crate::serialize::Object;
//
//     #[test]
//     fn test_cs_resource_mapper() {
//         let mut mapper = CsResourceMapper::new();
//         assert_eq!(mapper.remap(PdfColorSpace::SRGB), 0);
//         assert_eq!(mapper.remap(PdfColorSpace::D65Gray), 1);
//         assert_eq!(mapper.remap(PdfColorSpace::SRGB), 0);
//         assert_eq!(
//             mapper.remap_with_name(PdfColorSpace::SRGB),
//             String::from("C0")
//         );
//         let items = mapper.get_entries().collect::<Vec<_>>();
//         assert_eq!(items[0], (String::from("C0"), PdfColorSpace::SRGB));
//         assert_eq!(items[1], (String::from("C1"), PdfColorSpace::D65Gray));
//     }
// }
