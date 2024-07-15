use crate::color::Color;
use crate::transform::FiniteTransform;
use pdf_writer::types::FunctionShadingType;
use strict_num::{NormalizedF32, NormalizedF64, PositiveF32};
use tiny_skia_path::{FiniteF32, Transform};

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
    pub r: PositiveF32,
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
struct GradientProperties {
    coords: Vec<FiniteF32>,
    shading_type: FunctionShadingType,
    stops: Vec<Stop>,
}

impl Paint {
    fn gradient_properties(&self) -> Option<((GradientProperties, FiniteTransform))> {
        match self {
            Paint::LinearGradient(l) => Some((GradientProperties {
                coords: vec![l.x1, l.y1, l.x2, l.y2],
                shading_type: FunctionShadingType::Axial,
                stops: Vec::from(l.stops.clone()),
            }, l.transform)),
            Paint::RadialGradient(r) => Some((GradientProperties {
                coords: vec![
                    r.fx,
                    r.fy,
                    FiniteF32::new(0.0).unwrap(),
                    r.cx,
                    r.cy,
                    FiniteF32::new(r.r.get()).unwrap(),
                ],
                shading_type: FunctionShadingType::Radial,
                stops: Vec::from(r.stops.clone())
            }, r.transform)),
            _ => None,
        }
    }
}