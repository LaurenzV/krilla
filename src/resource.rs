use crate::chunk_container::ChunkContainer;
use crate::color::ICCBasedColorSpace;
use crate::error::KrillaResult;
use crate::font::FontIdentifier;
use crate::object::ext_g_state::ExtGState;
#[cfg(feature = "raster-images")]
use crate::object::image::Image;
use crate::object::shading_function::ShadingFunction;
use crate::object::shading_pattern::ShadingPattern;
use crate::object::tiling_pattern::TilingPattern;
use crate::object::xobject::XObject;
use crate::object::Object;
use crate::serialize::SerializerContext;
use crate::util::NameExt;
use pdf_writer::types::ProcSet;
use pdf_writer::writers::{FormXObject, Page, Pages, Resources, Type3Font};
use pdf_writer::{Chunk, Dict, Finish, Ref};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;

pub(crate) trait ResourceTrait {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a>;
    fn get_prefix() -> &'static str;
    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<Self>;
}

impl ResourceTrait for ExtGState {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.ext_g_states()
    }

    fn get_prefix() -> &'static str {
        "g"
    }

    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<ExtGState> {
        &mut b.ext_g_states
    }
}

impl ResourceTrait for ColorSpaceResource {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.color_spaces()
    }

    fn get_prefix() -> &'static str {
        "c"
    }

    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<ColorSpaceResource> {
        &mut b.color_spaces
    }
}

impl ResourceTrait for ShadingFunction {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.shadings()
    }

    fn get_prefix() -> &'static str {
        "s"
    }

    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<ShadingFunction> {
        &mut b.shadings
    }
}

#[derive(Hash, Eq, PartialEq)]
pub(crate) enum Resource {
    XObject(XObject),
    Image(Image),
    ShadingPattern(ShadingPattern),
    TilingPattern(TilingPattern),
    ExtGState(ExtGState),
    ColorSpace(ColorSpaceResource),
    Shading(ShadingFunction),
    Font(FontIdentifier),
}

impl From<XObjectResource> for Resource {
    fn from(val: XObjectResource) -> Self {
        match val {
            XObjectResource::XObject(x) => Resource::XObject(x),
            XObjectResource::Image(i) => Resource::Image(i),
        }
    }
}

impl From<PatternResource> for Resource {
    fn from(val: PatternResource) -> Self {
        match val {
            PatternResource::ShadingPattern(s) => Resource::ShadingPattern(s),
            PatternResource::TilingPattern(t) => Resource::TilingPattern(t),
        }
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

    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<XObjectResource> {
        &mut b.x_objects
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) enum PatternResource {
    ShadingPattern(ShadingPattern),
    TilingPattern(TilingPattern),
}

impl ResourceTrait for PatternResource {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.patterns()
    }

    fn get_prefix() -> &'static str {
        "p"
    }

    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<PatternResource> {
        &mut b.patterns
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

#[derive(Debug)]
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

    pub(crate) fn register_resource<T>(&mut self, resource: T, sc: &mut SerializerContext) -> String
    where
        T: ResourceTrait + Into<Resource>,
    {
        // TODO Don't unwrap
        let ref_ = sc.add_resource(resource).unwrap();

        T::get_mapper(self).remap_with_name(ref_)
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
    pub fn to_pdf_resources<T>(&self, parent: &mut T) -> KrillaResult<()>
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
        write_resource_type::<ColorSpaceResource>(resources, &self.color_spaces)?;
        write_resource_type::<ExtGState>(resources, &self.ext_g_states)?;
        write_resource_type::<PatternResource>(resources, &self.patterns)?;
        write_resource_type::<XObjectResource>(resources, &self.x_objects)?;
        write_resource_type::<ShadingFunction>(resources, &self.shadings)?;
        write_resource_type::<FontIdentifier>(resources, &self.fonts)?;

        Ok(())
    }
}

fn write_resource_type<T>(
    resources: &mut Resources,
    resource_list: &ResourceList<T>,
) -> KrillaResult<()>
where
    T: ResourceTrait,
{
    if resource_list.len() > 0 {
        let mut dict = T::get_dict(resources);

        for (name, entry) in resource_list.get_entries() {
            dict.pair(name.to_pdf_name(), entry);
        }

        dict.finish();
    }

    Ok(())
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) struct ResourceList<V> {
    entries: Vec<Ref>,
    phantom: PhantomData<V>,
}

impl<T> ResourceList<T>
where
    T: ResourceTrait,
{
    pub fn len(&self) -> u32 {
        self.entries.len() as u32
    }

    fn name_from_number(num: ResourceNumber) -> String {
        format!("{}{}", T::get_prefix(), num)
    }

    pub fn get_entries(&self) -> impl Iterator<Item = (String, Ref)> + '_ {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, r)| (Self::name_from_number(i as ResourceNumber), *r))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ResourceMapper<T: ?Sized> {
    forward: Vec<Ref>,
    backward: HashMap<Ref, ResourceNumber>,
    phantom: PhantomData<T>,
}

impl<T> ResourceMapper<T>
where
    T: ResourceTrait,
{
    pub fn new() -> Self {
        Self {
            forward: Vec::new(),
            backward: HashMap::new(),
            phantom: PhantomData,
        }
    }

    pub fn remap(&mut self, ref_: Ref) -> ResourceNumber {
        let forward = &mut self.forward;
        let backward = &mut self.backward;

        *backward.entry(ref_).or_insert_with(|| {
            let old = forward.len();
            forward.push(ref_);
            old as ResourceNumber
        })
    }

    pub fn remap_with_name(&mut self, ref_: Ref) -> String {
        Self::name_from_number(self.remap(ref_))
    }

    fn name_from_number(num: ResourceNumber) -> String {
        format!("{}{}", T::get_prefix(), num)
    }

    pub fn into_resource_list(self) -> ResourceList<T> {
        ResourceList {
            entries: self.forward,
            phantom: Default::default(),
        }
    }
}

pub type ResourceNumber = u32;

/// The ICC profile for the SRGB color space.
static SRGB_ICC: &[u8] = include_bytes!("icc/sRGB-v4.icc");
/// The ICC profile for the sgray color space.
static GREY_ICC: &[u8] = include_bytes!("icc/sGrey-v4.icc");

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum ColorSpaceResource {
    Srgb,
    SGray,
}

impl Object for ColorSpaceResource {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.color_spaces
    }

    fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        match self {
            ColorSpaceResource::Srgb => {
                let icc_based = ICCBasedColorSpace::new(Arc::new(SRGB_ICC), 3);
                icc_based.serialize(sc, root_ref)
            }
            ColorSpaceResource::SGray => {
                let icc_based = ICCBasedColorSpace::new(Arc::new(GREY_ICC), 1);
                icc_based.serialize(sc, root_ref)
            }
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

    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<Self> {
        &mut b.fonts
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

impl ResourcesExt for pdf_writer::writers::TilingPattern<'_> {
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
