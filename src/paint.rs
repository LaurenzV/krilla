use crate::object::color_space::Color;
use crate::stream::Stream;
use std::sync::Arc;
use tiny_skia_path::{NormalizedF32, Transform};

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

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub struct Stop {
    pub offset: NormalizedF32,
    pub color: Color,
    pub opacity: NormalizedF32,
}

#[derive(Debug, Clone)]
pub struct LinearGradient {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub transform: Transform,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

#[derive(Debug, Clone)]
pub struct RadialGradient {
    pub cx: f32,
    pub cy: f32,
    pub cr: f32,
    pub fx: f32,
    pub fy: f32,
    pub fr: f32,
    pub transform: Transform,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

#[derive(Debug, Clone)]
pub struct SweepGradient {
    pub cx: f32,
    pub cy: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    pub transform: Transform,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub(crate) stream: Arc<Stream>,
    pub(crate) transform: Transform,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Debug, Clone)]
pub enum Paint {
    Color(Color),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
    SweepGradient(SweepGradient),
    Pattern(Arc<Pattern>),
}
