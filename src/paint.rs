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
    pub cr: FiniteF32,
    pub fx: FiniteF32,
    pub fy: FiniteF32,
    pub fr: FiniteF32,
    pub transform: TransformWrapper,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct SweepGradient {
    pub cx: FiniteF32,
    pub cy: FiniteF32,
    pub start_angle: FiniteF32,
    pub end_angle: FiniteF32,
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
pub enum GradientType {
    Sweep,
    Linear,
    Radial,
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct Shading(GradientProperties);

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct GradientProperties {
    pub min: FiniteF32,
    pub max: FiniteF32,
    // Only use for radial
    pub cr: FiniteF32,
    // Only use for radial
    pub fr: FiniteF32,
    pub shading_type: FunctionShadingType,
    pub stops: Vec<Stop>,
    // The bbox of the object the gradient is applied to
    pub bbox: Rect,
    pub spread_method: SpreadMethod,
    pub gradient_type: GradientType,
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
                cr: FiniteF32::new(0.0).unwrap(),
                fr: FiniteF32::new(0.0).unwrap(),
                shading_type: FunctionShadingType::Axial,
                stops: Vec::from(self.stops.clone()),
                bbox: get_expanded_bbox(bbox, self.transform.0.post_concat(ts)),
                spread_method: self.spread_method,
                gradient_type: GradientType::Linear,
            },
            TransformWrapper(self.transform.0.post_concat(ts)),
        )
    }
}

impl GradientPropertiesExt for SweepGradient {
    fn gradient_properties(&self, bbox: Rect) -> (GradientProperties, TransformWrapper) {
        let mut min = self.start_angle;
        let max = self.end_angle;

        let transform = self
            .transform
            .0
            .post_concat(Transform::from_translate(self.cx.get(), self.cy.get()));

        (
            GradientProperties {
                min,
                max,
                cr: FiniteF32::new(0.0).unwrap(),
                fr: FiniteF32::new(0.0).unwrap(),
                shading_type: FunctionShadingType::Function,
                stops: Vec::from(self.stops.clone()),
                bbox: get_expanded_bbox(bbox, transform),
                spread_method: self.spread_method,
                gradient_type: GradientType::Sweep,
            },
            TransformWrapper(transform),
        )
    }
}

impl GradientPropertiesExt for RadialGradient {
    fn gradient_properties(&self, bbox: Rect) -> (GradientProperties, TransformWrapper) {
        let (ts, min, max) = get_point_ts(
            Point::from_xy(self.cx.get(), self.cy.get()),
            Point::from_xy(self.fx.get(), self.fy.get()),
        );

        (
            GradientProperties {
                min: FiniteF32::new(min).unwrap(),
                max: FiniteF32::new(max).unwrap(),
                cr: self.cr,
                fr: self.fr,
                shading_type: FunctionShadingType::Radial,
                stops: Vec::from(self.stops.clone()),
                bbox: get_expanded_bbox(bbox, self.transform.0.post_concat(ts)),
                spread_method: self.spread_method,
                gradient_type: GradientType::Radial,
            },
            TransformWrapper(self.transform.0.post_concat(ts)),
        )
    }
}
