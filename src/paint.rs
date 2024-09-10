//! Paints that can be used for filling and stroking text or paths.

use crate::object::color::ColorSpace;
use crate::stream::Stream;
use tiny_skia_path::{NormalizedF32, Transform};
use crate::color::{cmyk, Color, rgb};

/// A linear gradient.
#[derive(Debug, Clone)]
pub struct LinearGradient<C>
where
    C: ColorSpace,
{
    /// The x coordinate of the first point.
    pub x1: f32,
    /// The y coordinate of the first point.
    pub y1: f32,
    /// The x coordinate of the second point.
    pub x2: f32,
    /// The y coordinate of the second point.
    pub y2: f32,
    /// A transform that should be applied to the linear gradient.
    pub transform: Transform,
    /// The spread method of the linear gradient.
    ///
    /// _Note_: The spread methods `Repeat`/`Reflect` are not supported in Firefox.
    pub spread_method: SpreadMethod,
    /// The color stops of the linear gradient.
    pub stops: Vec<Stop<C>>,
}

/// A radial gradient.
#[derive(Debug, Clone)]
pub struct RadialGradient<C>
where
    C: ColorSpace,
{
    /// The x coordinate of the start circle.
    pub fx: f32,
    /// The y coordinate of the start circle.
    pub fy: f32,
    /// The radius of the start circle.
    pub fr: f32,
    /// The x coordinate of the end circle.
    pub cx: f32,
    /// The y coordinate of the end circle.
    pub cy: f32,
    /// The radius of the end circle.
    pub cr: f32,
    /// A transform that should be applied to the radial gradient.
    pub transform: Transform,
    /// The spread method of the radial gradient.
    ///
    /// _Note_: The spread methods `Repeat`/`Reflect` are currently not supported
    /// for radial gradients, and will fall back to `Pad`.
    pub spread_method: SpreadMethod,
    /// The color stops of the radial gradient.
    pub stops: Vec<Stop<C>>,
}

/// A sweep gradient.
///
/// Angles start from the right and go counter-clockwise with increasing values.
#[derive(Debug, Clone)]
pub struct SweepGradient<C>
where
    C: ColorSpace,
{
    /// The x coordinate of the center.
    pub cx: f32,
    /// The y coordinate of the center.
    pub cy: f32,
    /// The start angle.
    pub start_angle: f32,
    /// The end angle.
    pub end_angle: f32,
    /// A transform that should be applied to the sweep gradient.
    pub transform: Transform,
    /// The spread method of the sweep gradient.
    ///
    /// _Note_: The spread methods `Repeat`/`Reflect` are not supported in Firefox.
    pub spread_method: SpreadMethod,
    /// The color stops of the sweep gradient.
    pub stops: Vec<Stop<C>>,
}

/// A pattern.
#[derive(Debug, Clone)]
pub struct Pattern {
    /// The stream of the pattern.
    pub stream: Stream,
    /// A transform that should be applied to the pattern.
    pub transform: Transform,
    /// The width of the pattern.
    pub width: f32,
    /// The height of the pattern.
    pub height: f32,
}

#[derive(Debug, Clone)]
pub(crate) enum InnerPaint
{
    RgbColor(rgb::Color),
    CmykColor(cmyk::Color),
    RgbLinearGradient(LinearGradient<rgb::Rgb>),
    CmykLinearGradient(LinearGradient<cmyk::DeviceCmyk>),
    RgbRadialGradient(RadialGradient<rgb::Rgb>),
    CmykRadialGradient(RadialGradient<cmyk::DeviceCmyk>),
    RgbSweepGradient(SweepGradient<rgb::Rgb>),
    CmykSweepGradient(SweepGradient<cmyk::DeviceCmyk>),
    Pattern(Pattern),
}

#[derive(Debug, Clone)]
pub struct Paint(InnerPaint);

impl From<rgb::Color> for Paint {
    fn from(value: rgb::Color) -> Self {
        Paint(InnerPaint::RgbColor(value))
    }
}

impl From<cmyk::Color> for Paint {
    fn from(value: cmyk::Color) -> Self {
        Paint(InnerPaint::CmykColor(value))
    }
}

impl From<LinearGradient<rgb::Rgb>> for Paint {
    fn from(value: LinearGradient<rgb::Rgb>) -> Self {
        Paint(InnerPaint::RgbLinearGradient(value))
    }
}

impl From<LinearGradient<cmyk::DeviceCmyk>> for Paint {
    fn from(value: LinearGradient<cmyk::DeviceCmyk>) -> Self {
        Paint(InnerPaint::CmykLinearGradient(value))
    }
}

impl From<RadialGradient<rgb::Rgb>> for Paint {
    fn from(value: RadialGradient<rgb::Rgb>) -> Self {
        Paint(InnerPaint::RgbRadialGradient(value))
    }
}

impl From<RadialGradient<cmyk::DeviceCmyk>> for Paint {
    fn from(value: RadialGradient<cmyk::DeviceCmyk>) -> Self {
        Paint(InnerPaint::CmykRadialGradient(value))
    }
}

impl From<SweepGradient<rgb::Rgb>> for Paint {
    fn from(value: SweepGradient<rgb::Rgb>) -> Self {
        Paint(InnerPaint::RgbSweepGradient(value))
    }
}

impl From<SweepGradient<cmyk::DeviceCmyk>> for Paint {
    fn from(value: SweepGradient<cmyk::DeviceCmyk>) -> Self {
        Paint(InnerPaint::CmykSweepGradient(value))
    }
}

impl From<Pattern> for Paint {
    fn from(value: Pattern) -> Self {
        Paint(InnerPaint::Pattern(value))
    }
}

/// A spread method.
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum SpreadMethod {
    /// The pad spread method.
    Pad,
    /// The reflect spread method.
    Reflect,
    /// The repeat spread method.
    Repeat,
}

impl Default for SpreadMethod {
    fn default() -> Self {
        Self::Pad
    }
}

/// A color stop in a gradient.
#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub struct Stop<C>
where
    C: ColorSpace,
{
    /// The normalized offset of the stop.
    pub offset: NormalizedF32,
    /// The color of the stop.
    pub color: C::Color,
    /// The opacity of the stop.
    pub opacity: NormalizedF32,
}

impl<C> From<Stop<C>> for crate::object::shading_function::Stop
where
    C: ColorSpace,
{
    fn from(val: Stop<C>) -> Self {
        crate::object::shading_function::Stop {
            offset: val.offset,
            opacity: val.opacity,
            color: val.color.into(),
        }
    }
}
