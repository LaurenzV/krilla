use crate::canvas::Canvas;
use crate::color::Color;
use crate::serialize::Object;
use crate::transform::TransformWrapper;
use crate::util::RectExt;
use pdf_writer::types::FunctionShadingType;
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, NormalizedF32, Point, Rect, Scalar, Transform};

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum SpreadMethod {
    Pad,
    Reflect,
    Repeat,
}

impl Default for SpreadMethod {
    fn default() -> Self {
        Self::Pad
    }
}

pub type StopOffset = NormalizedF32;

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub struct Stop {
    pub offset: StopOffset,
    pub color: Color,
    pub opacity: NormalizedF32,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct LinearGradient {
    pub x1: FiniteF32,
    pub y1: FiniteF32,
    pub x2: FiniteF32,
    pub y2: FiniteF32,
    pub transform: TransformWrapper,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct RadialGradient {
    pub cx: FiniteF32,
    pub cy: FiniteF32,
    pub r: FiniteF32,
    pub fx: FiniteF32,
    pub fy: FiniteF32,
    pub transform: TransformWrapper,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct SweepGradient {
    pub cx: FiniteF32,
    pub cy: FiniteF32,
    pub angle: FiniteF32,
    pub transform: TransformWrapper,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Pattern {
    pub(crate) canvas: Arc<Canvas>,
    pub(crate) transform: TransformWrapper,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum Paint {
    Color(Color),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
    SweepGradient(SweepGradient),
    Pattern(Arc<Pattern>),
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct Shading(GradientProperties);

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct GradientProperties {
    pub min: FiniteF32,
    pub max: FiniteF32,
    pub shading_type: FunctionShadingType,
    pub stops: Vec<Stop>,
    // The bbox of the object the gradient is applied to
    pub bbox: Rect,
    pub spread_method: SpreadMethod,
}

pub trait GradientPropertiesExt {
    // TODO: BBox only needed if extend is not pad
    fn gradient_properties(&self, bbox: Rect) -> (GradientProperties, TransformWrapper);
}

fn get_expanded_bbox(mut bbox: Rect, shading_transform: Transform) -> Rect {
    // We need to make sure the shading covers the whole bbox of the object after
    // the transform as been applied. In order to know that, we need to calculate the
    // resulting bbox from the inverted transform.
    bbox.expand(&bbox.transform(shading_transform.invert().unwrap()).unwrap());
    bbox
}

fn get_point_ts(start: Point, end: Point) -> (Transform, f32, f32) {
    let dist = start.distance(end);

    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let angle = dy.atan2(dx).to_degrees();

    (
        Transform::from_rotate_at(angle, start.x, start.y),
        start.x,
        start.x + dist,
    )
}

impl GradientPropertiesExt for LinearGradient {
    fn gradient_properties(&self, bbox: Rect) -> (GradientProperties, TransformWrapper) {
        // TODO: Make prettier

        let (ts, min, max) = get_point_ts(
            Point::from_xy(self.x1.get(), self.y1.get()),
            Point::from_xy(self.x2.get(), self.y2.get()),
        );
        (
            GradientProperties {
                min: FiniteF32::new(min).unwrap(),
                max: FiniteF32::new(max).unwrap(),
                shading_type: FunctionShadingType::Axial,
                stops: Vec::from(self.stops.clone()),
                bbox: get_expanded_bbox(bbox, self.transform.0.post_concat(ts)),
                spread_method: self.spread_method,
            },
            TransformWrapper(self.transform.0.post_concat(ts)),
        )
    }
}

impl GradientPropertiesExt for SweepGradient {
    fn gradient_properties(&self, bbox: Rect) -> (GradientProperties, TransformWrapper) {
        todo!()
    }
}

impl GradientPropertiesExt for RadialGradient {
    fn gradient_properties(&self, bbox: Rect) -> (GradientProperties, TransformWrapper) {
        // TODO: Normalize coords
        todo!();
        // let (bbox, transform) = get_normalized(bbox, self.transform.0);
        // (
        //     GradientProperties {
        //         coords: vec![
        //             self.fx,
        //             self.fy,
        //             FiniteF32::new(0.0).unwrap(),
        //             self.cx,
        //             self.cy,
        //             FiniteF32::new(self.r.get()).unwrap(),
        //         ],
        //         shading_type: FunctionShadingType::Radial,
        //         stops: Vec::from(self.stops.clone()),
        //         spread_method: self.spread_method,
        //         bbox,
        //     },
        //     TransformWrapper(transform),
        // )
    }
}
