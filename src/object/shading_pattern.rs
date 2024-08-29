use crate::chunk_container::ChunkContainer;
use crate::object::shading_function::{GradientProperties, ShadingFunction};
use crate::serialize::{Object, SerializerContext};
use crate::transform::TransformWrapper;
use crate::util::TransformExt;
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::sync::Arc;

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    shading_function: ShadingFunction,
    shading_transform: TransformWrapper,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ShadingPattern(Arc<Repr>);

impl ShadingPattern {
    pub fn new(
        gradient_properties: GradientProperties,
        shading_transform: TransformWrapper,
    ) -> Self {
        Self(Arc::new(Repr {
            // CTM doesn't need to be included to calculate the domain of the shading function
            shading_function: ShadingFunction::new(gradient_properties, false),
            shading_transform,
        }))
    }
}

impl Object for ShadingPattern {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.patterns
    }

    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        let shading_ref = sc.add_object(self.0.shading_function.clone());
        let mut shading_pattern = chunk.shading_pattern(root_ref);
        shading_pattern.pair(Name(b"Shading"), shading_ref);
        shading_pattern.matrix(self.0.shading_transform.0.to_pdf_transform());

        shading_pattern.finish();

        chunk
    }
}
