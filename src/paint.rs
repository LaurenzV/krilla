use crate::color::Color;
use crate::transform::FiniteTransform;
use strict_num::{NormalizedF32, NormalizedF64, PositiveF32};
use tiny_skia_path::FiniteF32;

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

pub type StopOffset = NormalizedF64;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
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
