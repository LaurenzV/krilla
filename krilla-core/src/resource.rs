use crate::color::{GREY_ICC_DEFLATED, SRGB_ICC_DEFLATED};
use crate::paint::{LinearGradient, RadialGradient};
use crate::serialize::{ObjectSerialize, RefAllocator, SerializeSettings};
use pdf_writer::{Chunk, Ref};
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::sync::Arc;

pub struct ResourceDictionary {
    color_spaces: CsResourceMapper,
}

impl ResourceDictionary {
    pub fn new() -> Self {
        Self {
            color_spaces: CsResourceMapper::new(),
        }
    }
    pub fn register_color_space(&mut self, color_space: PdfColorSpace) -> String {
        self.color_spaces.remap_with_name(color_space)
    }
}

pub struct GraphicsState {}

trait PDFResource {
    fn get_name() -> &'static str;
}

pub type CsResourceMapper = ResourceMapper<PdfColorSpace>;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum PdfColorSpace {
    SRGB,
    D65Gray,
    LinearGradient(Arc<LinearGradient>),
    RadialGradient(Arc<RadialGradient>),
}

impl PDFResource for PdfColorSpace {
    fn get_name() -> &'static str {
        "cs"
    }
}

impl ObjectSerialize for PdfColorSpace {
    fn serialize(&self, _: &SerializeSettings) -> (Chunk, Ref) {
        let mut chunk = Chunk::new();
        let mut _ref = Ref::new(1);
        let mut root_ref = _ref.bump();

        match self {
            PdfColorSpace::SRGB => {
                chunk
                    .icc_profile(root_ref, &SRGB_ICC_DEFLATED)
                    .n(3)
                    .range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
                    .filter(pdf_writer::Filter::FlateDecode);
            }
            PdfColorSpace::D65Gray => {
                chunk
                    .icc_profile(root_ref, &GREY_ICC_DEFLATED)
                    .n(1)
                    .range([0.0, 1.0])
                    .filter(pdf_writer::Filter::FlateDecode);
            }
            _ => unimplemented!(),
        }

        (chunk, root_ref)
    }
}

pub struct ResourceMapper<V>
where
    V: Hash + Eq,
{
    forward: BTreeMap<ResourceNumber, V>,
    backward: HashMap<V, ResourceNumber>,
    counter: ResourceNumber,
}

impl<V> ResourceMapper<V>
where
    V: Hash + Eq + Clone + PDFResource,
{
    pub fn new() -> Self {
        Self {
            forward: BTreeMap::new(),
            backward: HashMap::new(),
            counter: 1,
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
        let counter = &mut self.counter;

        *backward.entry(resource.clone()).or_insert_with(|| {
            let old = *counter;
            *counter += 1;
            forward.insert(old, resource.clone());
            old
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
            .map(|s| (Self::name_from_number(*s.0), s.1.clone()))
    }
}

/// Struct that keeps track name allocations in a XObject/Page.
#[derive(Clone, Copy, Eq, PartialEq, Default)]
pub struct ResourceNumberAllocator {
    /// The next number that will be used for the name of an XObject in a resource
    /// dictionary, e.g. "xo0".
    next_x_object_num: ResourceNumber,
    /// The next number that will be used for the name of a graphics state in a resource
    /// dictionary, e.g. "gs0".
    next_graphics_state_num: ResourceNumber,
    /// The next number that will be used for the name of a pattern in a resource
    /// dictionary, e.g. "po0".
    next_patterns_num: ResourceNumber,
    /// The next number that will be used for the name of a shading in a resource
    /// dictionary, e.g. "sh0".
    next_shadings_num: ResourceNumber,
    /// The next number that will be used for the name of a font in a resource
    /// dictionary, e.g. "fo0".
    next_fonts_num: ResourceNumber,
    /// The next number that will be used for the name of a color space in a resource
    /// dictionary, e.g. "cs0".
    next_color_space_num: ResourceNumber,
}

pub type ResourceNumber = u32;

impl ResourceNumberAllocator {
    /// Allocate a new XObject name.
    pub fn alloc_x_object_number(&mut self) -> ResourceNumber {
        let num = self.next_x_object_num;
        self.next_x_object_num.checked_add(1).unwrap();
        num
    }

    /// Allocate a new graphics state name.
    pub fn alloc_graphics_state_number(&mut self) -> ResourceNumber {
        let num = self.next_graphics_state_num;
        self.next_graphics_state_num.checked_add(1).unwrap();
        num
    }

    /// Allocate a new shading name.
    pub fn alloc_shading_number(&mut self) -> ResourceNumber {
        let num = self.next_shadings_num;
        self.next_shadings_num.checked_add(1).unwrap();
        num
    }

    /// Allocate a new shading name.
    pub fn alloc_font_number(&mut self) -> ResourceNumber {
        let num = self.next_fonts_num;
        self.next_fonts_num.checked_add(1).unwrap();
        num
    }

    /// Allocate a new color space name.
    pub fn alloc_color_space_number(&mut self) -> ResourceNumber {
        let num = self.next_color_space_num;
        self.next_color_space_num.checked_add(1).unwrap();
        num
    }
}

#[cfg(test)]
mod tests {
    use crate::resource::{CsResourceMapper, PdfColorSpace};
    use crate::serialize::ObjectSerialize;

    #[test]
    fn test_cs_resource_mapper() {
        let mut mapper = CsResourceMapper::new();
        assert_eq!(mapper.remap(PdfColorSpace::SRGB), 1);
        assert_eq!(mapper.remap(PdfColorSpace::D65Gray), 2);
        assert_eq!(mapper.remap(PdfColorSpace::SRGB), 1);
        assert_eq!(
            mapper.remap_with_name(PdfColorSpace::SRGB),
            String::from("cs1")
        );
        let items = mapper.get_entries().collect::<Vec<_>>();
        assert_eq!(items[0], (String::from("cs1"), PdfColorSpace::SRGB));
        assert_eq!(items[1], (String::from("cs2"), PdfColorSpace::D65Gray));
    }
}
