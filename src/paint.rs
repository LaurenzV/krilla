use crate::canvas::Canvas;
use crate::color::Color;
use crate::transform::TransformWrapper;
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, NormalizedF32};

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
