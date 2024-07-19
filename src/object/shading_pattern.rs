use crate::object::shading_function::ShadingFunction;
use crate::paint::GradientProperties;
use crate::serialize::{Object, SerializerContext};
use crate::transform::TransformWrapper;
use crate::util::TransformExt;
use pdf_writer::{Name, Ref};
use std::sync::Arc;

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    shading_function: ShadingFunction,
    ctm: TransformWrapper,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ShadingPattern(Arc<Repr>);

impl ShadingPattern {
    pub fn new(
        gradient_properties: GradientProperties,
        ctm: TransformWrapper,
        pattern_transform: TransformWrapper,
    ) -> Self {
        Self(Arc::new(Repr {
            // CTM doesn't need to be included to calculate the domain of the shading function
            shading_function: ShadingFunction::new(gradient_properties, pattern_transform),
            ctm,
        }))
    }
}

impl Object for ShadingPattern {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let shading_ref = sc.add(self.0.shading_function.clone());
        let mut shading_pattern = sc.chunk_mut().shading_pattern(root_ref);
        shading_pattern.pair(Name(b"Shading"), shading_ref);
        shading_pattern.matrix(
            self.0
                .ctm
                .0
                .pre_concat(self.0.shading_function.shading_transform().0)
                .to_pdf_transform(),
        );
    }

    fn is_cached(&self) -> bool {
        true
    }
}
