//! Shading patterns.

use std::hash::Hash;
use std::sync::Arc;

use pdf_writer::{Chunk, Finish, Name, Ref};
use tiny_skia_path::Transform;

use crate::object::shading_function::{GradientProperties, ShadingFunction};
use crate::object::{Cacheable, ChunkContainerFn, Resourceable};
use crate::resource;
use crate::serialize::SerializeContext;
use crate::util::{HashExt, TransformExt};

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

#[cfg(test)]
mod tests {
    use krilla_macros::{snapshot, visreg};
    use tiny_skia_path::{NormalizedF32, Rect};

    use crate::object::shading_function::GradientPropertiesExt;
    use crate::object::shading_pattern::ShadingPattern;
    use crate::page::Page;
    use crate::paint::{LinearGradient, RadialGradient, SpreadMethod, SweepGradient};
    use crate::path::Fill;
    use crate::serialize::SerializeContext;
    use crate::surface::Surface;
    use crate::tests::{
        rect_to_path, stops_with_1_solid, stops_with_2_solid_1, stops_with_3_solid_1,
    };

    #[snapshot]
    fn linear_gradient_pad(sc: &mut SerializeContext) {
        let gradient = LinearGradient {
            x1: 50.0,
            y1: 0.0,
            x2: 150.0,
            y2: 0.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: stops_with_2_solid_1(),
            anti_alias: false,
        };

        let (props, transform) =
            gradient.gradient_properties(Rect::from_ltrb(50.0, 50.0, 150.0, 150.0).unwrap());
        let shading_pattern = ShadingPattern::new(props, transform);
        sc.register_cacheable(shading_pattern);
    }

    #[snapshot]
    fn linear_gradient_repeat(sc: &mut SerializeContext) {
        let gradient = LinearGradient {
            x1: 50.0,
            y1: 0.0,
            x2: 150.0,
            y2: 0.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Repeat,
            stops: stops_with_2_solid_1(),
            anti_alias: false,
        };

        let (props, transform) =
            gradient.gradient_properties(Rect::from_ltrb(50.0, 50.0, 150.0, 150.0).unwrap());
        let shading_pattern = ShadingPattern::new(props, transform);
        sc.register_cacheable(shading_pattern);
    }

    #[snapshot]
    fn sweep_gradient_pad(sc: &mut SerializeContext) {
        let gradient = SweepGradient {
            cx: 100.0,
            cy: 100.0,
            start_angle: 0.0,
            end_angle: 90.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: stops_with_2_solid_1(),
            anti_alias: false,
        };

        let (props, transform) =
            gradient.gradient_properties(Rect::from_ltrb(50.0, 50.0, 150.0, 150.0).unwrap());
        let shading_pattern = ShadingPattern::new(props, transform);
        sc.register_cacheable(shading_pattern);
    }

    #[snapshot]
    fn sweep_gradient_repeat(sc: &mut SerializeContext) {
        let gradient = SweepGradient {
            cx: 100.0,
            cy: 100.0,
            start_angle: 0.0,
            end_angle: 90.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Repeat,
            stops: stops_with_2_solid_1(),
            anti_alias: false,
        };

        let (props, transform) =
            gradient.gradient_properties(Rect::from_ltrb(50.0, 50.0, 150.0, 150.0).unwrap());
        let shading_pattern = ShadingPattern::new(props, transform);
        sc.register_cacheable(shading_pattern);
    }

    #[snapshot]
    fn radial_gradient_pad(sc: &mut SerializeContext) {
        let gradient = RadialGradient {
            cx: 100.0,
            cy: 100.0,
            cr: 50.0,
            fx: 120.0,
            fy: 120.0,
            fr: 50.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: stops_with_2_solid_1(),
            anti_alias: false,
        };

        let (props, transform) =
            gradient.gradient_properties(Rect::from_ltrb(50.0, 50.0, 150.0, 150.0).unwrap());
        let shading_pattern = ShadingPattern::new(props, transform);
        sc.register_cacheable(shading_pattern);
    }
}
