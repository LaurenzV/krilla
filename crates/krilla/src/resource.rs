//! Dealing with PDF resources.

use crate::color::{ICCBasedColorSpace, ICCProfile};
use crate::object::font::FontIdentifier;
#[cfg(feature = "raster-images")]
use crate::object::image::Image;
use crate::object::shading_pattern::ShadingPattern;
use crate::object::tiling_pattern::TilingPattern;
use crate::serialize::SerializerContext;
use crate::util::NameExt;
use once_cell::sync::Lazy;
use pdf_writer::types::ProcSet;
use pdf_writer::writers::{FormXObject, Page, Pages, Resources, Type3Font};
use pdf_writer::{Dict, Finish, Ref};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;

pub(crate) trait RegisterableResource<T>: Into<Resource>
where
    T: ResourceTrait,
{
}

pub(crate) trait ResourceTrait {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a>;
    fn get_prefix() -> &'static str;
    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<Self>;
}

#[derive(Debug, Eq, PartialEq, Hash, Default, Clone)]
pub(crate) struct ExtGState;

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

#[derive(Debug, Eq, PartialEq, Hash, Default, Clone)]
pub(crate) struct ColorSpace;

impl ResourceTrait for ColorSpace {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.color_spaces()
    }

    fn get_prefix() -> &'static str {
        "c"
    }

    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<ColorSpace> {
        &mut b.color_spaces
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Default, Clone)]
pub(crate) struct ShadingFunction;

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

#[derive(Debug, Eq, PartialEq, Hash, Default, Clone)]
pub(crate) struct XObject;

impl ResourceTrait for XObject {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.x_objects()
    }

    fn get_prefix() -> &'static str {
        "x"
    }

    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<XObject> {
        &mut b.x_objects
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Default, Clone)]
pub(crate) struct Pattern;

impl ResourceTrait for Pattern {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.patterns()
    }

    fn get_prefix() -> &'static str {
        "p"
    }

    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<Pattern> {
        &mut b.patterns
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Default, Clone)]
pub(crate) struct Font;

impl ResourceTrait for Font {
    fn get_dict<'a>(resources: &'a mut Resources) -> Dict<'a> {
        resources.fonts()
    }

    fn get_prefix() -> &'static str {
        "f"
    }

    fn get_mapper(b: &mut ResourceDictionaryBuilder) -> &mut ResourceMapper<Font> {
        &mut b.fonts
    }
}

#[derive(Hash, Eq, PartialEq)]
pub(crate) enum Resource {
    XObject(crate::object::xobject::XObject),
    #[cfg(feature = "raster-images")]
    Image(Image),
    ShadingPattern(ShadingPattern),
    TilingPattern(TilingPattern),
    ExtGState(crate::object::ext_g_state::ExtGState),
    LinearRgb,
    Srgb,
    Luma,
    Cmyk(ICCBasedColorSpace<4>),
    ShadingFunction(crate::object::shading_function::ShadingFunction),
    FontIdentifier(FontIdentifier),
}

impl From<crate::object::xobject::XObject> for Resource {
    fn from(value: crate::object::xobject::XObject) -> Self {
        Self::XObject(value)
    }
}

#[cfg(feature = "raster-images")]
impl From<Image> for Resource {
    fn from(value: Image) -> Self {
        Self::Image(value)
    }
}

impl From<ICCBasedColorSpace<3>> for Resource {
    fn from(_: ICCBasedColorSpace<3>) -> Self {
        Self::Srgb
    }
}

impl From<ICCBasedColorSpace<1>> for Resource {
    fn from(_: ICCBasedColorSpace<1>) -> Self {
        Self::Luma
    }
}

impl From<ICCBasedColorSpace<4>> for Resource {
    fn from(cs: ICCBasedColorSpace<4>) -> Self {
        Self::Cmyk(cs)
    }
}

impl From<ShadingPattern> for Resource {
    fn from(value: ShadingPattern) -> Self {
        Self::ShadingPattern(value)
    }
}

impl From<TilingPattern> for Resource {
    fn from(value: TilingPattern) -> Self {
        Self::TilingPattern(value)
    }
}

impl From<crate::object::shading_function::ShadingFunction> for Resource {
    fn from(value: crate::object::shading_function::ShadingFunction) -> Self {
        Self::ShadingFunction(value)
    }
}

impl From<crate::object::ext_g_state::ExtGState> for Resource {
    fn from(value: crate::object::ext_g_state::ExtGState) -> Self {
        Self::ExtGState(value)
    }
}

impl From<FontIdentifier> for Resource {
    fn from(value: FontIdentifier) -> Self {
        Self::FontIdentifier(value)
    }
}

#[derive(Debug)]
pub(crate) struct ResourceDictionaryBuilder {
    pub color_spaces: ResourceMapper<ColorSpace>,
    pub ext_g_states: ResourceMapper<ExtGState>,
    pub patterns: ResourceMapper<Pattern>,
    pub x_objects: ResourceMapper<XObject>,
    pub shadings: ResourceMapper<ShadingFunction>,
    pub fonts: ResourceMapper<Font>,
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

    pub(crate) fn register_resource<T, V>(
        &mut self,
        resource: T,
        sc: &mut SerializerContext,
    ) -> String
    where
        T: RegisterableResource<V>,
        V: ResourceTrait,
    {
        let ref_ = sc.add_resource(resource);

        V::get_mapper(self).remap_with_name(ref_)
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

#[derive(Debug, Eq, PartialEq, Hash, Default, Clone)]
pub(crate) struct ResourceDictionary {
    pub color_spaces: ResourceList<ColorSpace>,
    pub ext_g_states: ResourceList<ExtGState>,
    pub patterns: ResourceList<Pattern>,
    pub x_objects: ResourceList<XObject>,
    pub shadings: ResourceList<ShadingFunction>,
    pub fonts: ResourceList<Font>,
}

impl ResourceDictionary {
    pub fn to_pdf_resources<T>(&self, parent: &mut T)
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
        write_resource_type::<ColorSpace>(resources, &self.color_spaces);
        write_resource_type::<ExtGState>(resources, &self.ext_g_states);
        write_resource_type::<Pattern>(resources, &self.patterns);
        write_resource_type::<XObject>(resources, &self.x_objects);
        write_resource_type::<ShadingFunction>(resources, &self.shadings);
        write_resource_type::<Font>(resources, &self.fonts);
    }
}

fn write_resource_type<T>(resources: &mut Resources, resource_list: &ResourceList<T>)
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
}

#[derive(Debug, Eq, PartialEq, Hash, Clone, Default)]
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

/// The ICC v4 profile for the SRGB color space.
pub(crate) static SRGB_V4_ICC: Lazy<ICCProfile<3>> =
    Lazy::new(|| ICCProfile::new(Arc::new(include_bytes!("icc/sRGB-v4.icc"))).unwrap());
/// The ICC v2 profile for the SRGB color space.
pub(crate) static SRGB_V2_ICC: Lazy<ICCProfile<3>> =
    Lazy::new(|| ICCProfile::new(Arc::new(include_bytes!("icc/sRGB-v2-magic.icc"))).unwrap());
/// The ICC v4 profile for the sgray color space.
pub(crate) static GREY_V4_ICC: Lazy<ICCProfile<1>> =
    Lazy::new(|| ICCProfile::new(Arc::new(include_bytes!("icc/sGrey-v4.icc"))).unwrap());
/// The ICC v2 profile for the sgray color space.
pub(crate) static GREY_V2_ICC: Lazy<ICCProfile<1>> =
    Lazy::new(|| ICCProfile::new(Arc::new(include_bytes!("icc/sGrey-v2-magic.icc"))).unwrap());

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
