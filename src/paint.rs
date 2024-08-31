use crate::object::color::ColorSpace;
use crate::stream::Stream;
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
pub struct Stop<C>
where
    C: ColorSpace,
{
    pub offset: NormalizedF32,
    pub color: C::Color,
    pub opacity: NormalizedF32,
}

impl<C> Into<crate::object::shading_function::Stop> for Stop<C>
where
    C: ColorSpace,
{
    fn into(self) -> crate::object::shading_function::Stop {
        crate::object::shading_function::Stop {
            offset: self.offset,
            opacity: self.opacity,
            color: self.color.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinearGradient<C>
where
    C: ColorSpace,
{
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub transform: Transform,
    pub spread_method: SpreadMethod,
    pub stops: Vec<Stop<C>>,
}

#[derive(Debug, Clone)]
pub struct RadialGradient<C>
where
    C: ColorSpace,
{
    pub cx: f32,
    pub cy: f32,
    pub cr: f32,
    pub fx: f32,
    pub fy: f32,
    pub fr: f32,
    pub transform: Transform,
    pub spread_method: SpreadMethod,
    pub stops: Vec<Stop<C>>,
}

#[derive(Debug, Clone)]
pub struct SweepGradient<C>
where
    C: ColorSpace,
{
    pub cx: f32,
    pub cy: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    pub transform: Transform,
    pub spread_method: SpreadMethod,
    pub stops: Vec<Stop<C>>,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub(crate) stream: Stream,
    pub(crate) transform: Transform,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Debug, Clone)]
pub enum Paint<C>
where
    C: ColorSpace,
{
    Color(C::Color),
    LinearGradient(LinearGradient<C>),
    RadialGradient(RadialGradient<C>),
    SweepGradient(SweepGradient<C>),
    Pattern(Pattern),
}
