use crate::font::FontIdentifier;
use crate::object::color_space::device_cmyk::DeviceCmyk;
use crate::object::color_space::luma::{DeviceGray, SGray};
use crate::object::color_space::rgb::{DeviceRgb, Srgb};
use crate::object::color_space::{DEVICE_CMYK, DEVICE_GRAY, DEVICE_RGB};
use crate::object::ext_g_state::ExtGState;
use crate::object::image::Image;
use crate::object::shading_function::ShadingFunction;
use crate::object::shading_pattern::ShadingPattern;
use crate::object::xobject::XObject;
use crate::serialize::{ChunkContainer, ChunkMap, Object, SerializerContext, SipHashable};
use crate::util::NameExt;
use pdf_writer::types::ProcSet;
use pdf_writer::{Chunk, Dict, Finish, Ref};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

pub trait ResourceTrait: Object {
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

impl ResourceTrait for ColorSpaceEnum {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.color_spaces()
    }

    fn get_prefix() -> &'static str {
        "c"
    }
}

impl ResourceTrait for ShadingFunction {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.shadings()
    }

    fn get_prefix() -> &'static str {
        "s"
    }
}

#[derive(Hash, Eq, PartialEq)]
pub(crate) enum Resource {
    XObject(XObjectResource),
    Pattern(PatternResource),
    ExtGState(ExtGState),
    ColorSpace(ColorSpaceEnum),
    Shading(ShadingFunction),
    Font(FontIdentifier),
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
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
    fn chunk_container(&self, cc: &mut ChunkContainer) -> &mut Vec<ChunkMap> {
        match self {
            XObjectResource::XObject(x) => x.chunk_container(cc),
            XObjectResource::Image(i) => i.chunk_container(cc),
        }
    }

    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        match self {
            XObjectResource::XObject(x) => x.serialize_into(sc, root_ref),
            XObjectResource::Image(i) => i.serialize_into(sc, root_ref),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum PatternResource {
    ShadingPattern(ShadingPattern),
    TilingPattern(crate::object::tiling_pattern::TilingPattern),
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
    fn chunk_container(&self, cc: &mut ChunkContainer) -> &mut Vec<ChunkMap> {
        &mut cc.patterns
    }

    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        match self {
            PatternResource::ShadingPattern(sp) => sp.serialize_into(sc, root_ref),
            PatternResource::TilingPattern(tp) => tp.serialize_into(sc, root_ref),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ResourceDictionaryBuilder {
    pub color_spaces: ResourceMapper<ColorSpaceEnum>,
    pub ext_g_states: ResourceMapper<ExtGState>,
    pub patterns: ResourceMapper<PatternResource>,
    pub x_objects: ResourceMapper<XObjectResource>,
    pub shadings: ResourceMapper<ShadingFunction>,
    pub fonts: ResourceMapper<FontIdentifier>,
}

impl ResourceDictionaryBuilder {
    pub fn new() -> Self {
        Self {
            color_spaces: ResourceMapper::new(),
            ext_g_states: ResourceMapper::new(),
            patterns: ResourceMapper::new(),
            x_objects: ResourceMapper::new(),
            shadings: ResourceMapper::new(),
            fonts: ResourceMapper::new(),
        }
    }

    fn register_color_space(&mut self, color_space: ColorSpaceEnum) -> String {
        match color_space {
            ColorSpaceEnum::DeviceRgb(_) => DEVICE_RGB.to_string(),
            ColorSpaceEnum::DeviceGray(_) => DEVICE_GRAY.to_string(),
            ColorSpaceEnum::DeviceCmyk(_) => DEVICE_CMYK.to_string(),
            ColorSpaceEnum::Srgb(_) => self.color_spaces.remap_with_name(color_space),
            ColorSpaceEnum::SGray(_) => self.color_spaces.remap_with_name(color_space),
        }
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

    fn register_shading(&mut self, shading: ShadingFunction) -> String {
        self.shadings.remap_with_name(shading)
    }

    fn register_font(&mut self, font: FontIdentifier) -> String {
        self.fonts.remap_with_name(font)
    }

    pub fn register_resource(&mut self, resource: Resource) -> String {
        match resource {
            Resource::XObject(x) => self.register_x_object(x),
            Resource::Pattern(p) => self.register_pattern(p),
            Resource::ExtGState(e) => self.register_ext_g_state(e),
            Resource::ColorSpace(c) => self.register_color_space(c),
            Resource::Shading(s) => self.register_shading(s),
            Resource::Font(f) => self.register_font(f),
        }
    }

    pub fn finish(self) -> ResourceDictionary {
        ResourceDictionary {
            color_spaces: self.color_spaces.to_resource_list(),
            ext_g_states: self.ext_g_states.to_resource_list(),
            patterns: self.patterns.to_resource_list(),
            x_objects: self.x_objects.to_resource_list(),
            shadings: self.shadings.to_resource_list(),
            fonts: self.fonts.to_resource_list(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) struct ResourceDictionary {
    pub color_spaces: ResourceList<ColorSpaceEnum>,
    pub ext_g_states: ResourceList<ExtGState>,
    pub patterns: ResourceList<PatternResource>,
    pub x_objects: ResourceList<XObjectResource>,
    pub shadings: ResourceList<ShadingFunction>,
    pub fonts: ResourceList<FontIdentifier>,
}

impl ResourceDictionary {
    pub fn to_pdf_resources<T>(&self, sc: &mut SerializerContext, parent: &mut T)
    where
        T: ResourcesExt,
    {
        let resources = &mut parent.resources();
        resources.proc_sets([
            ProcSet::Pdf,
            ProcSet::Text,
            ProcSet::ImageColor,
            ProcSet::ImageGrayscale,
        ]);
        write_resource_type(sc, resources, &self.color_spaces, false);
        write_resource_type(sc, resources, &self.ext_g_states, false);
        write_resource_type(sc, resources, &self.patterns, false);
        write_resource_type(sc, resources, &self.x_objects, false);
        write_resource_type(sc, resources, &self.shadings, false);
        write_resource_type(sc, resources, &self.fonts, true);
    }
}

fn write_resource_type<T>(
    sc: &mut SerializerContext,
    resources: &mut Resources,
    resource_list: &ResourceList<T>,
    is_font: bool,
) where
    T: Hash + Eq + ResourceTrait + Debug + Clone,
{
    if resource_list.len() > 0 {
        let mut dict = T::get_dict(resources);

        for (name, entry) in resource_list.get_entries() {
            if !is_font {
                dict.pair(name.to_pdf_name(), sc.add(entry));
            } else {
                dict.pair(name.to_pdf_name(), sc.add_font(entry));
            }
        }

        dict.finish();
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ResourceList<V>
where
    V: Hash + Eq + PartialEq + Debug,
{
    entries: Vec<V>,
}

impl<T> ResourceList<T>
where
    T: Hash + Eq + ResourceTrait + Debug + Clone,
{
    pub fn len(&self) -> u32 {
        self.entries.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn name_from_number(num: ResourceNumber) -> String {
        format!("{}{}", T::get_prefix(), num)
    }

    pub fn get_entries(&self) -> impl Iterator<Item = (String, T)> + '_ {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, r)| (Self::name_from_number(i as ResourceNumber), r.clone()))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ResourceMapper<V>
where
    V: Hash + Eq + PartialEq + Debug + ResourceTrait,
{
    forward: Vec<V>,
    backward: HashMap<u128, ResourceNumber>,
}

impl<V> ResourceMapper<V>
where
    V: Hash + Eq + ResourceTrait + Debug + 'static,
{
    pub fn new() -> Self {
        Self {
            forward: Vec::new(),
            backward: HashMap::new(),
        }
    }

    pub fn get(&self, resource: &V) -> Option<ResourceNumber> {
        self.backward.get(&resource.sip_hash()).copied()
    }

    pub fn remap(&mut self, resource: V) -> ResourceNumber {
        let forward = &mut self.forward;
        let backward = &mut self.backward;

        *backward.entry(resource.sip_hash()).or_insert_with(|| {
            let old = forward.len();
            forward.push(resource);
            old as ResourceNumber
        })
    }

    pub fn remap_with_name(&mut self, resource: V) -> String {
        Self::name_from_number(self.remap(resource))
    }

    // TODO: Deduplicate
    fn name_from_number(num: ResourceNumber) -> String {
        format!("{}{}", V::get_prefix(), num)
    }

    pub fn to_resource_list(self) -> ResourceList<V> {
        ResourceList {
            entries: self.forward,
        }
    }
}

pub type ResourceNumber = u32;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum ColorSpaceEnum {
    Srgb(Srgb),
    SGray(SGray),
    DeviceGray(DeviceGray),
    DeviceRgb(DeviceRgb),
    DeviceCmyk(DeviceCmyk),
}

impl Object for ColorSpaceEnum {
    fn chunk_container(&self, cc: &mut ChunkContainer) -> &mut Vec<ChunkMap> {
        &mut cc.color_spaces
    }

    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        match self {
            ColorSpaceEnum::Srgb(srgb) => srgb.serialize_into(sc, root_ref),
            ColorSpaceEnum::SGray(sgray) => sgray.serialize_into(sc, root_ref),
            ColorSpaceEnum::DeviceGray(_) => unreachable!(),
            ColorSpaceEnum::DeviceRgb(_) => unreachable!(),
            ColorSpaceEnum::DeviceCmyk(_) => unreachable!(),
        }
    }
}

impl ResourceTrait for FontIdentifier {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.fonts()
    }

    fn get_prefix() -> &'static str {
        "f"
    }
}

use pdf_writer::writers::{FormXObject, Page, Pages, Resources, TilingPattern, Type3Font};

/// A trait for getting the resource dictionary of an object.
pub trait ResourcesExt {
    /// Return the resources dictionary of the object.
    fn resources(&mut self) -> Resources<'_>;
}

impl ResourcesExt for FormXObject<'_> {
    fn resources(&mut self) -> Resources<'_> {
        self.resources()
    }
}

impl ResourcesExt for TilingPattern<'_> {
    fn resources(&mut self) -> Resources<'_> {
        self.resources()
    }
}

impl ResourcesExt for Type3Font<'_> {
    fn resources(&mut self) -> Resources<'_> {
        self.resources()
    }
}

impl ResourcesExt for Pages<'_> {
    fn resources(&mut self) -> Resources<'_> {
        self.resources()
    }
}

impl ResourcesExt for Page<'_> {
    fn resources(&mut self) -> Resources<'_> {
        self.resources()
    }
}
