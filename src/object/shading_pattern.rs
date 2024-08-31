use crate::chunk_container::ChunkContainer;
use crate::error::KrillaResult;
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

    fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let shading_ref = sc.add_object(self.0.shading_function.clone())?;
        let mut shading_pattern = chunk.shading_pattern(root_ref);
        shading_pattern.pair(Name(b"Shading"), shading_ref);
        shading_pattern.matrix(self.0.shading_transform.0.to_pdf_transform());

        shading_pattern.finish();

        Ok(chunk)
    }
}

mod tests {
    use tiny_skia_path::Rect;
    use krilla_macros::snapshot;
    use crate::{LinearGradient, RadialGradient, SpreadMethod, SweepGradient};
    use crate::object::shading_function::GradientPropertiesExt;
    use crate::object::shading_pattern::ShadingPattern;
    use crate::serialize::SerializerContext;
    use crate::tests::stops_with_2_solid_1;

    #[snapshot]
    fn linear_gradient_pad(sc: &mut SerializerContext) {
        let gradient = LinearGradient {
            x1: 50.0,
            y1: 0.0,
            x2: 150.0,
            y2: 0.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: stops_with_2_solid_1(),
        };

        let (props, transform) = gradient.gradient_properties(Rect::from_ltrb(50.0, 50.0, 150.0, 150.0).unwrap());
        let shading_pattern = ShadingPattern::new(props, transform);
        sc.add_object(shading_pattern).unwrap();
    }

    #[snapshot]
    fn linear_gradient_repeat(sc: &mut SerializerContext) {
        let gradient = LinearGradient {
            x1: 50.0,
            y1: 0.0,
            x2: 150.0,
            y2: 0.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Repeat,
            stops: stops_with_2_solid_1(),
        };

        let (props, transform) = gradient.gradient_properties(Rect::from_ltrb(50.0, 50.0, 150.0, 150.0).unwrap());
        let shading_pattern = ShadingPattern::new(props, transform);
        sc.add_object(shading_pattern).unwrap();
    }

    #[snapshot]
    fn sweep_gradient_pad(sc: &mut SerializerContext) {
        let gradient = SweepGradient {
            cx: 100.0,
            cy: 100.0,
            start_angle: 0.0,
            end_angle: 90.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: stops_with_2_solid_1(),
        };

        let (props, transform) = gradient.gradient_properties(Rect::from_ltrb(50.0, 50.0, 150.0, 150.0).unwrap());
        let shading_pattern = ShadingPattern::new(props, transform);
        sc.add_object(shading_pattern).unwrap();
    }

    #[snapshot]
    fn sweep_gradient_repeat(sc: &mut SerializerContext) {
        let gradient = SweepGradient {
            cx: 100.0,
            cy: 100.0,
            start_angle: 0.0,
            end_angle: 90.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Repeat,
            stops: stops_with_2_solid_1(),
        };

        let (props, transform) = gradient.gradient_properties(Rect::from_ltrb(50.0, 50.0, 150.0, 150.0).unwrap());
        let shading_pattern = ShadingPattern::new(props, transform);
        sc.add_object(shading_pattern).unwrap();
    }

    #[snapshot]
    fn radial_gradient_pad(sc: &mut SerializerContext) {
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
        };

        let (props, transform) = gradient.gradient_properties(Rect::from_ltrb(50.0, 50.0, 150.0, 150.0).unwrap());
        let shading_pattern = ShadingPattern::new(props, transform);
        sc.add_object(shading_pattern).unwrap();
    }
}
