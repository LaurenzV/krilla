//! Paints that can be used for filling and stroking text or paths.

use crate::color::{cmyk, rgb, Color};
use crate::stream::Stream;
use crate::util::{F32Wrapper, TransformWrapper};
use tiny_skia_path::{FiniteF32, NormalizedF32, Transform};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) enum InnerStops {
    RgbStops(Vec<Stop<rgb::Color>>),
    CmykStops(Vec<Stop<cmyk::Color>>),
}

/// The color stops of a gradient.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LinearGradient {
    /// The x coordinate of the first point.
    pub(crate) x1: F32Wrapper,
    /// The y coordinate of the first point.
    pub(crate) y1: F32Wrapper,
    /// The x coordinate of the second point.
    pub(crate) x2: F32Wrapper,
    /// The y coordinate of the second point.
    pub(crate) y2: F32Wrapper,
    /// A transform that should be applied to the linear gradient.
    pub(crate) transform: TransformWrapper,
    /// The spread method of the linear gradient.
    pub(crate) spread_method: SpreadMethod,
    /// The color stops of the linear gradient.
    pub(crate) stops: Stops,
}

impl LinearGradient {
    /// Create a new linear gradient.
    pub fn new(
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        transform: Transform,
        spread_method: SpreadMethod,
        stops: Stops,
    ) -> Self {
        Self {
            x1: F32Wrapper(x1),
            y1: F32Wrapper(y1),
            x2: F32Wrapper(x2),
            y2: F32Wrapper(y2),
            transform: TransformWrapper(transform),
            spread_method,
            stops,
        }
    }
}

/// A radial gradient.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct RadialGradient {
    /// The x coordinate of the start circle.
    pub(crate) fx: F32Wrapper,
    /// The y coordinate of the start circle.
    pub(crate) fy: F32Wrapper,
    /// The radius of the start circle.
    pub(crate) fr: F32Wrapper,
    /// The x coordinate of the end circle.
    pub(crate) cx: F32Wrapper,
    /// The y coordinate of the end circle.
    pub(crate) cy: F32Wrapper,
    /// The radius of the end circle.
    pub(crate) cr: F32Wrapper,
    /// A transform that should be applied to the radial gradient.
    pub(crate) transform: TransformWrapper,
    /// The spread method of the radial gradient.
    ///
    /// _Note_: The spread methods `Repeat`/`Reflect` are currently not supported
    /// for radial gradients, and will fall back to `Pad`.
    pub(crate) spread_method: SpreadMethod,
    /// The color stops of the radial gradient.
    pub(crate) stops: Stops,
}

impl RadialGradient {
    /// Create a new radial gradient.
    pub fn new(
        fx: f32,
        fy: f32,
        fr: f32,
        cx: f32,
        cy: f32,
        cr: f32,
        transform: Transform,
        spread_method: SpreadMethod,
        stops: Stops,
    ) -> Self {
        Self {
            fx: F32Wrapper(fx),
            fy: F32Wrapper(fy),
            fr: F32Wrapper(fr),
            cx: F32Wrapper(cx),
            cy: F32Wrapper(cy),
            cr: F32Wrapper(cr),
            transform: TransformWrapper(transform),
            spread_method,
            stops,
        }
    }
}

/// A sweep gradient.
///
/// Angles start from the right and go counter-clockwise with increasing values.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SweepGradient {
    /// The x coordinate of the center.
    pub(crate) cx: F32Wrapper,
    /// The y coordinate of the center.
    pub(crate) cy: F32Wrapper,
    /// The start angle.
    pub(crate) start_angle: F32Wrapper,
    /// The end angle.
    pub(crate) end_angle: F32Wrapper,
    /// A transform that should be applied to the sweep gradient.
    pub(crate) transform: TransformWrapper,
    /// The spread method of the sweep gradient.
    pub(crate) spread_method: SpreadMethod,
    /// The color stops of the sweep gradient.
    pub(crate) stops: Stops,
}

impl SweepGradient {
    /// Create a new sweep gradient.
    pub fn new(
        cx: f32,
        cy: f32,
        start_angle: f32,
        end_angle: f32,
        transform: Transform,
        spread_method: SpreadMethod,
        stops: Stops,
    ) -> Self {
        Self {
            cx: F32Wrapper(cx),
            cy: F32Wrapper(cy),
            start_angle: F32Wrapper(start_angle),
            end_angle: F32Wrapper(end_angle),
            transform: TransformWrapper(transform),
            spread_method,
            stops,
        }
    }
}

/// A pattern.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Pattern {
    /// The stream of the pattern.
    pub(crate) stream: Stream,
    /// A transform that should be applied to the pattern.
    pub(crate) transform: TransformWrapper,
    /// The width of the pattern.
    pub(crate) width: F32Wrapper,
    /// The height of the pattern.
    pub(crate) height: F32Wrapper,
}

impl Pattern {
    /// Create a new pattern.
    pub fn new(stream: Stream, transform: Transform, width: f32, height: f32) -> Self {
        Self {
            stream,
            transform: TransformWrapper(transform),
            width: F32Wrapper(width),
            height: F32Wrapper(height),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum InnerPaint {
    Color(Color),
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
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Paint(pub(crate) InnerPaint);

impl From<rgb::Color> for Paint {
    fn from(value: rgb::Color) -> Self {
        Paint(InnerPaint::Color(value.into()))
    }
}

impl TryFrom<Paint> for rgb::Color {
    type Error = ();

    fn try_from(value: Paint) -> Result<Self, Self::Error> {
        match value.0 {
            InnerPaint::Color(c) => match c {
                Color::Rgb(rgb) => Ok(rgb),
                Color::DeviceCmyk(_) => Err(()),
            },
            _ => Err(()),
        }
    }
}

impl From<cmyk::Color> for Paint {
    fn from(value: cmyk::Color) -> Self {
        Paint(InnerPaint::Color(value.into()))
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
