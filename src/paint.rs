//! Paints that can be used for filling and stroking text or paths.

use crate::color::{cmyk, rgb, Color};
use crate::stream::Stream;
use tiny_skia_path::{NormalizedF32, Transform};

#[derive(Debug, Clone)]
pub(crate) enum InnerStops {
    RgbStops(Vec<Stop<rgb::Color>>),
    CmykStops(Vec<Stop<cmyk::Color>>),
}

/// The color stops of a gradient.
#[derive(Debug, Clone)]
pub struct Stops(pub(crate) InnerStops);

impl IntoIterator for InnerStops {
    type Item = crate::object::shading_function::Stop;
    type IntoIter = std::vec::IntoIter<crate::object::shading_function::Stop>;

    fn into_iter(self) -> Self::IntoIter {
        // TODO: Avoid collect somehow?
        match self {
            InnerStops::RgbStops(r) => r
                .into_iter()
                .map(|c| c.into())
                .collect::<Vec<_>>()
                .into_iter(),
            InnerStops::CmykStops(c) => c
                .into_iter()
                .map(|c| c.into())
                .collect::<Vec<_>>()
                .into_iter(),
        }
    }
}

impl From<Vec<Stop<rgb::Color>>> for Stops {
    fn from(value: Vec<Stop<rgb::Color>>) -> Self {
        Stops(InnerStops::RgbStops(value))
    }
}

impl From<Vec<Stop<cmyk::Color>>> for Stops {
    fn from(value: Vec<Stop<cmyk::Color>>) -> Self {
        Stops(InnerStops::CmykStops(value))
    }
}

/// A linear gradient.
#[derive(Debug, Clone)]
pub struct LinearGradient {
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
    pub spread_method: SpreadMethod,
    /// The color stops of the linear gradient.
    pub stops: Stops,
}

/// A radial gradient.
#[derive(Debug, Clone)]
pub struct RadialGradient {
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
    pub stops: Stops,
}

/// A sweep gradient.
///
/// Angles start from the right and go counter-clockwise with increasing values.
#[derive(Debug, Clone)]
pub struct SweepGradient {
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
    pub spread_method: SpreadMethod,
    /// The color stops of the sweep gradient.
    pub stops: Stops,
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
pub(crate) enum InnerPaint {
    RgbColor(rgb::Color),
    CmykColor(cmyk::Color),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
    SweepGradient(SweepGradient),
    Pattern(Pattern),
}

/// A paint.
///
/// You cannot construct this type directly, but instead can convert
/// into it by calling `into` on the various types of paint, such as linear
/// gradients and patterns.
#[derive(Debug, Clone)]
pub struct Paint(pub(crate) InnerPaint);

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

impl From<LinearGradient> for Paint {
    fn from(value: LinearGradient) -> Self {
        Paint(InnerPaint::LinearGradient(value))
    }
}

impl From<RadialGradient> for Paint {
    fn from(value: RadialGradient) -> Self {
        Paint(InnerPaint::RadialGradient(value))
    }
}

impl From<SweepGradient> for Paint {
    fn from(value: SweepGradient) -> Self {
        Paint(InnerPaint::SweepGradient(value))
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
#[allow(private_bounds)]
pub struct Stop<C>
where
    C: Into<Color>,
{
    /// The normalized offset of the stop.
    pub offset: NormalizedF32,
    /// The color of the stop.
    pub color: C,
    /// The opacity of the stop.
    pub opacity: NormalizedF32,
}

impl<C> From<Stop<C>> for crate::object::shading_function::Stop
where
    C: Into<Color>,
{
    fn from(val: Stop<C>) -> Self {
        crate::object::shading_function::Stop {
            offset: val.offset,
            opacity: val.opacity,
            color: val.color.into(),
        }
    }
}
