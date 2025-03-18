use std::hash::Hash;
use std::sync::Arc;

use pdf_writer::{Chunk, Finish, Name, Ref};

use crate::chunk_container::ChunkContainerFn;
use crate::geom::Transform;
use crate::graphics::shading_function::{GradientProperties, ShadingFunction};
use crate::resource::Resourceable;
use crate::serialize::{Cacheable, SerializeContext};
use crate::resource;

#[derive(Debug, PartialEq)]
struct Repr {
    shading_function: ShadingFunction,
    shading_transform: Transform,
}

impl Eq for Repr {}

impl Hash for Repr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.shading_function.hash(state);
        self.shading_transform.hash(state);
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) struct ShadingPattern(Arc<Repr>);

impl ShadingPattern {
    pub fn new(gradient_properties: GradientProperties, shading_transform: Transform) -> Self {
        Self(Arc::new(Repr {
            // CTM doesn't need to be included to calculate the domain of the shading function
            shading_function: ShadingFunction::new(gradient_properties, false),
            shading_transform,
        }))
    }
}

impl Cacheable for ShadingPattern {
    fn chunk_container(&self) -> ChunkContainerFn {
        |cc| &mut cc.patterns
    }

    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        let shading_ref = sc.register_cacheable(self.0.shading_function.clone());
        let mut shading_pattern = chunk.shading_pattern(root_ref);
        shading_pattern.pair(Name(b"Shading"), shading_ref);
        shading_pattern.matrix(self.0.shading_transform.to_pdf_transform());

        shading_pattern.finish();

        chunk
    }
}

impl Resourceable for ShadingPattern {
    type Resource = resource::Pattern;
}
