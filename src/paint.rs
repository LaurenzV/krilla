use crate::color::Color;
use crate::transform::FiniteTransform;
use pdf_writer::types::FunctionShadingType;
use tiny_skia_path::{FiniteF32, NormalizedF32, Transform};

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum SpreadMethod {
    Pad,
    // Reflect,
    // Repeat,
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

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct LinearGradient {
    pub x1: FiniteF32,
    pub y1: FiniteF32,
    pub x2: FiniteF32,
    pub y2: FiniteF32,
    pub transform: FiniteTransform,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct RadialGradient {
    pub cx: FiniteF32,
    pub cy: FiniteF32,
    pub r: FiniteF32,
    pub fx: FiniteF32,
    pub fy: FiniteF32,
    pub transform: FiniteTransform,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum Paint {
    Color(Color),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct Shading(GradientProperties);

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct GradientProperties {
    pub coords: Vec<FiniteF32>,
    pub shading_type: FunctionShadingType,
    pub stops: Vec<Stop>,
}

pub trait GradientPropertiesExt {
    fn gradient_properties(&self) -> (GradientProperties, FiniteTransform);
}

impl GradientPropertiesExt for LinearGradient {
    fn gradient_properties(&self) -> (GradientProperties, FiniteTransform) {
        (
            GradientProperties {
                coords: vec![self.x1, self.y1, self.x2, self.y2],
                shading_type: FunctionShadingType::Axial,
                stops: Vec::from(self.stops.clone()),
            },
            self.transform,
        )
    }
}

impl GradientPropertiesExt for RadialGradient {
    fn gradient_properties(&self) -> (GradientProperties, FiniteTransform) {
        (
            GradientProperties {
                coords: vec![
                    self.fx,
                    self.fy,
                    FiniteF32::new(0.0).unwrap(),
                    self.cx,
                    self.cy,
                    FiniteF32::new(self.r.get()).unwrap(),
                ],
                shading_type: FunctionShadingType::Radial,
                stops: Vec::from(self.stops.clone()),
            },
            self.transform,
        )
    }
}
