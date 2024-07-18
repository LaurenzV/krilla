use crate::canvas::{Canvas, CanvasPdfSerializer};
use crate::serialize::{Object, SerializeSettings, SerializerContext};
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
    pub images: ImageResourceMapper,
}

impl ResourceDictionary {
    pub fn new() -> Self {
        Self {
            color_spaces: CsResourceMapper::new(),
            ext_g_state: ExtGStateResourceMapper::new(),
            patterns: PatternResourceMapper::new(),
            x_objects: XObjectResourceMapper::new(),
            images: ImageResourceMapper::new(),
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

    pub fn register_image(&mut self, image: Image) -> String {
        self.images.remap_with_name(image)
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

        for (name, entry) in self.images.get_entries() {
            x_objects.pair(name.to_pdf_name(), sc.add_uncached(entry));
        }

        x_objects.finish();
    }
}

// TODO: trait should return what kind of resource an object is (so that image and XObject both
// get assigned to XObject)
pub trait PDFResource {
    fn get_name() -> &'static str;
}

pub type CsResourceMapper = ResourceMapper<PdfColorSpace>;
pub type ExtGStateResourceMapper = ResourceMapper<ExtGState>;
pub type PatternResourceMapper = ResourceMapper<PdfPattern>;
pub type XObjectResourceMapper = ResourceMapper<XObject>;
pub type ImageResourceMapper = ResourceMapper<Image>;

impl PDFResource for Image {
    fn get_name() -> &'static str {
        "I"
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct XObject {
    pub canvas: Arc<Canvas>,
    pub isolated: bool,
    pub needs_transparency: bool,
}

impl PDFResource for XObject {
    fn get_name() -> &'static str {
        "X"
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
    use crate::serialize::Object;

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
