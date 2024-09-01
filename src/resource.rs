use crate::chunk_container::ChunkContainer;
use crate::error::KrillaResult;
use crate::font::FontIdentifier;
use crate::object::color::luma::SGray;
use crate::object::color::rgb::Srgb;
use crate::object::ext_g_state::ExtGState;
#[cfg(feature = "raster-images")]
use crate::object::image::Image;
use crate::object::shading_function::ShadingFunction;
use crate::object::shading_pattern::ShadingPattern;
use crate::object::xobject::XObject;
use crate::object::Object;
use crate::serialize::SerializerContext;
use crate::util::{NameExt, SipHashable};
use pdf_writer::types::ProcSet;
use pdf_writer::writers::{FormXObject, Page, Pages, Resources, TilingPattern, Type3Font};
use pdf_writer::{Chunk, Dict, Finish, Ref};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

pub(crate) trait ResourceTrait: Hash {
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

impl ResourceTrait for ColorSpaceResource {
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
    ColorSpace(ColorSpaceResource),
    Shading(ShadingFunction),
    Font(FontIdentifier),
}

impl From<XObjectResource> for Resource {
    fn from(val: XObjectResource) -> Self {
        Resource::XObject(val)
    }
}

impl From<PatternResource> for Resource {
    fn from(val: PatternResource) -> Self {
        Resource::Pattern(val)
    }
}

impl From<ExtGState> for Resource {
    fn from(val: ExtGState) -> Self {
        Resource::ExtGState(val)
    }
}

impl From<ColorSpaceResource> for Resource {
    fn from(val: ColorSpaceResource) -> Self {
        Resource::ColorSpace(val)
    }
}

impl From<ShadingFunction> for Resource {
    fn from(val: ShadingFunction) -> Self {
        Resource::Shading(val)
    }
}

impl From<FontIdentifier> for Resource {
    fn from(val: FontIdentifier) -> Self {
        Resource::Font(val)
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) enum XObjectResource {
    XObject(XObject),
    #[cfg(feature = "raster-images")]
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
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        match self {
            XObjectResource::XObject(x) => x.chunk_container(cc),
            #[cfg(feature = "raster-images")]
            XObjectResource::Image(i) => i.chunk_container(cc),
        }
    }

    fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        match self {
            XObjectResource::XObject(x) => x.serialize(sc, root_ref),
            #[cfg(feature = "raster-images")]
            XObjectResource::Image(i) => i.serialize(sc, root_ref),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) enum PatternResource {
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
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.patterns
    }

    fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        match self {
            PatternResource::ShadingPattern(sp) => sp.serialize(sc, root_ref),
            PatternResource::TilingPattern(tp) => tp.serialize(sc, root_ref),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ResourceDictionaryBuilder {
    pub color_spaces: ResourceMapper<ColorSpaceResource>,
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

    fn register_color_space(&mut self, color_space: ColorSpaceResource) -> String {
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
            color_spaces: self.color_spaces.into_resource_list(),
            ext_g_states: self.ext_g_states.into_resource_list(),
            patterns: self.patterns.into_resource_list(),
            x_objects: self.x_objects.into_resource_list(),
            shadings: self.shadings.into_resource_list(),
            fonts: self.fonts.into_resource_list(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) struct ResourceDictionary {
    pub color_spaces: ResourceList<ColorSpaceResource>,
    pub ext_g_states: ResourceList<ExtGState>,
    pub patterns: ResourceList<PatternResource>,
    pub x_objects: ResourceList<XObjectResource>,
    pub shadings: ResourceList<ShadingFunction>,
    pub fonts: ResourceList<FontIdentifier>,
}

impl ResourceDictionary {
    pub fn to_pdf_resources<T>(
        &self,
        sc: &mut SerializerContext,
        parent: &mut T,
    ) -> KrillaResult<()>
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
        write_resource_type(sc, resources, &self.color_spaces)?;
        write_resource_type(sc, resources, &self.ext_g_states)?;
        write_resource_type(sc, resources, &self.patterns)?;
        write_resource_type(sc, resources, &self.x_objects)?;
        write_resource_type(sc, resources, &self.shadings)?;
        write_resource_type(sc, resources, &self.fonts)?;

        Ok(())
    }
}

fn write_resource_type<T>(
    sc: &mut SerializerContext,
    resources: &mut Resources,
    resource_list: &ResourceList<T>,
) -> KrillaResult<()>
where
    T: Hash + Eq + ResourceTrait + Into<Resource> + Debug + Clone,
{
    if resource_list.len() > 0 {
        let mut dict = T::get_dict(resources);

        for (name, entry) in resource_list.get_entries() {
            dict.pair(name.to_pdf_name(), sc.add_resource(entry)?);
        }

        dict.finish();
    }

    Ok(())
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) struct ResourceList<V>
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
pub(crate) struct ResourceMapper<V>
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

    fn name_from_number(num: ResourceNumber) -> String {
        format!("{}{}", V::get_prefix(), num)
    }

    pub fn into_resource_list(self) -> ResourceList<V> {
        ResourceList {
            entries: self.forward,
        }
    }
}

pub type ResourceNumber = u32;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum ColorSpaceResource {
    Srgb(Srgb),
    SGray(SGray),
}

impl Object for ColorSpaceResource {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.color_spaces
    }

    fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        match self {
            ColorSpaceResource::Srgb(srgb) => srgb.serialize(sc, root_ref),
            ColorSpaceResource::SGray(sgray) => sgray.serialize(sc, root_ref),
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
