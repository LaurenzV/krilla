use crate::color::Color;
use crate::Opacity;
use strict_num::{NormalizedF64, PositiveF64};
use tiny_skia_path::Transform;

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

pub type StopOffset = NormalizedF64;

pub struct Stop {
    pub offset: StopOffset,
    pub color: Color,
    pub opacity: Opacity,
}

pub struct LinearGradient {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub transform: Transform,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

pub struct RadialGradient {
    pub cx: f64,
    pub cy: f64,
    pub r: PositiveF64,
    pub fx: f64,
    pub fy: f64,
    pub transform: Transform,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

pub enum Paint {
    Color(Color),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
}
