//! Paint types for krilla Python bindings.

use pyo3::prelude::*;

use crate::color::{CmykColor, Color, LumaColor, RgbColor};
use crate::enums::{FillRule, LineCap, LineJoin, SpreadMethod};
use crate::geometry::Transform;
use crate::num::NormalizedF32;

/// A paint that can be used for filling or stroking.
///
/// Paint can be a solid color or a gradient. Use the from_* methods
/// or gradient into_paint() to create a Paint.
#[pyclass]
#[derive(Clone)]
pub struct Paint {
    pub(crate) inner: krilla::paint::Paint,
}

#[pymethods]
impl Paint {
    /// Create a paint from an RGB color.
    #[staticmethod]
    fn from_rgb(color: &RgbColor) -> Self {
        Paint {
            inner: krilla::paint::Paint::from(color.inner),
        }
    }

    /// Create a paint from a grayscale color.
    #[staticmethod]
    fn from_luma(color: &LumaColor) -> Self {
        Paint {
            inner: krilla::paint::Paint::from(color.inner),
        }
    }

    /// Create a paint from a CMYK color.
    #[staticmethod]
    fn from_cmyk(color: &CmykColor) -> Self {
        Paint {
            inner: krilla::paint::Paint::from(color.inner),
        }
    }

    /// Create a paint from a Color.
    #[staticmethod]
    fn from_color(color: &Color) -> Self {
        Paint {
            inner: krilla::paint::Paint::from(color.inner),
        }
    }

    /// Create a paint from a linear gradient.
    #[staticmethod]
    fn from_linear_gradient(gradient: &LinearGradient) -> Self {
        Paint {
            inner: krilla::paint::Paint::from(gradient.to_inner()),
        }
    }

    /// Create a paint from a radial gradient.
    #[staticmethod]
    fn from_radial_gradient(gradient: &RadialGradient) -> Self {
        Paint {
            inner: krilla::paint::Paint::from(gradient.to_inner()),
        }
    }

    /// Create a paint from a sweep gradient.
    #[staticmethod]
    fn from_sweep_gradient(gradient: &SweepGradient) -> Self {
        Paint {
            inner: krilla::paint::Paint::from(gradient.to_inner()),
        }
    }

    fn __repr__(&self) -> String {
        "Paint(...)".to_string()
    }
}

impl Paint {
    pub fn into_inner(self) -> krilla::paint::Paint {
        self.inner
    }

    pub fn from_inner(inner: krilla::paint::Paint) -> Self {
        Paint { inner }
    }
}

/// A color stop in a gradient.
#[pyclass]
#[derive(Clone)]
pub struct Stop {
    /// Position of the stop (0.0 to 1.0).
    #[pyo3(get, set)]
    pub offset: NormalizedF32,
    /// Color at this stop.
    #[pyo3(get, set)]
    pub color: Color,
    /// Opacity at this stop.
    #[pyo3(get, set)]
    pub opacity: NormalizedF32,
}

#[pymethods]
impl Stop {
    /// Create a new gradient stop.
    #[new]
    #[pyo3(signature = (offset, color, opacity=None))]
    fn new(offset: NormalizedF32, color: Color, opacity: Option<NormalizedF32>) -> Self {
        Stop {
            offset,
            color,
            opacity: opacity.unwrap_or_else(NormalizedF32::one),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Stop(offset={}, opacity={})",
            self.offset.inner.get(),
            self.opacity.inner.get()
        )
    }
}

impl Stop {
    pub fn to_inner(&self) -> krilla::paint::Stop {
        krilla::paint::Stop {
            offset: self.offset.into_inner(),
            color: self.color.into_inner(),
            opacity: self.opacity.into_inner(),
        }
    }
}

/// A linear gradient.
#[pyclass]
#[derive(Clone)]
pub struct LinearGradient {
    /// Start x coordinate.
    #[pyo3(get, set)]
    pub x1: f32,
    /// Start y coordinate.
    #[pyo3(get, set)]
    pub y1: f32,
    /// End x coordinate.
    #[pyo3(get, set)]
    pub x2: f32,
    /// End y coordinate.
    #[pyo3(get, set)]
    pub y2: f32,
    /// Transformation applied to the gradient.
    #[pyo3(get, set)]
    pub transform: Transform,
    /// How the gradient spreads beyond its bounds.
    #[pyo3(get, set)]
    pub spread_method: SpreadMethod,
    /// Color stops.
    #[pyo3(get, set)]
    pub stops: Vec<Stop>,
    /// Whether to apply anti-aliasing.
    #[pyo3(get, set)]
    pub anti_alias: bool,
}

#[pymethods]
impl LinearGradient {
    /// Create a new linear gradient.
    #[new]
    #[pyo3(signature = (x1, y1, x2, y2, stops, transform=None, spread_method=None, anti_alias=false))]
    fn new(
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        stops: Vec<Stop>,
        transform: Option<Transform>,
        spread_method: Option<SpreadMethod>,
        anti_alias: bool,
    ) -> Self {
        LinearGradient {
            x1,
            y1,
            x2,
            y2,
            transform: transform.unwrap_or_else(Transform::identity),
            spread_method: spread_method.unwrap_or(SpreadMethod::Pad),
            stops,
            anti_alias,
        }
    }

    /// Convert to a Paint.
    fn into_paint(&self) -> Paint {
        Paint::from_linear_gradient(self)
    }

    fn __repr__(&self) -> String {
        format!(
            "LinearGradient(({}, {}) -> ({}, {}), {} stops)",
            self.x1,
            self.y1,
            self.x2,
            self.y2,
            self.stops.len()
        )
    }
}

impl LinearGradient {
    pub fn to_inner(&self) -> krilla::paint::LinearGradient {
        krilla::paint::LinearGradient {
            x1: self.x1,
            y1: self.y1,
            x2: self.x2,
            y2: self.y2,
            transform: self.transform.into_inner(),
            spread_method: self.spread_method.into_inner(),
            stops: self.stops.iter().map(|s| s.to_inner()).collect(),
            anti_alias: self.anti_alias,
        }
    }
}

/// A radial gradient.
#[pyclass]
#[derive(Clone)]
pub struct RadialGradient {
    /// Focal point x coordinate.
    #[pyo3(get, set)]
    pub fx: f32,
    /// Focal point y coordinate.
    #[pyo3(get, set)]
    pub fy: f32,
    /// Focal radius.
    #[pyo3(get, set)]
    pub fr: f32,
    /// Center x coordinate.
    #[pyo3(get, set)]
    pub cx: f32,
    /// Center y coordinate.
    #[pyo3(get, set)]
    pub cy: f32,
    /// Center radius.
    #[pyo3(get, set)]
    pub cr: f32,
    /// Transformation applied to the gradient.
    #[pyo3(get, set)]
    pub transform: Transform,
    /// How the gradient spreads beyond its bounds.
    #[pyo3(get, set)]
    pub spread_method: SpreadMethod,
    /// Color stops.
    #[pyo3(get, set)]
    pub stops: Vec<Stop>,
    /// Whether to apply anti-aliasing.
    #[pyo3(get, set)]
    pub anti_alias: bool,
}

#[pymethods]
impl RadialGradient {
    /// Create a new radial gradient.
    #[new]
    #[pyo3(signature = (fx, fy, fr, cx, cy, cr, stops, transform=None, spread_method=None, anti_alias=false))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        fx: f32,
        fy: f32,
        fr: f32,
        cx: f32,
        cy: f32,
        cr: f32,
        stops: Vec<Stop>,
        transform: Option<Transform>,
        spread_method: Option<SpreadMethod>,
        anti_alias: bool,
    ) -> Self {
        RadialGradient {
            fx,
            fy,
            fr,
            cx,
            cy,
            cr,
            transform: transform.unwrap_or_else(Transform::identity),
            spread_method: spread_method.unwrap_or(SpreadMethod::Pad),
            stops,
            anti_alias,
        }
    }

    /// Convert to a Paint.
    fn into_paint(&self) -> Paint {
        Paint::from_radial_gradient(self)
    }

    fn __repr__(&self) -> String {
        format!(
            "RadialGradient(focal=({}, {}, {}), center=({}, {}, {}), {} stops)",
            self.fx,
            self.fy,
            self.fr,
            self.cx,
            self.cy,
            self.cr,
            self.stops.len()
        )
    }
}

impl RadialGradient {
    pub fn to_inner(&self) -> krilla::paint::RadialGradient {
        krilla::paint::RadialGradient {
            fx: self.fx,
            fy: self.fy,
            fr: self.fr,
            cx: self.cx,
            cy: self.cy,
            cr: self.cr,
            transform: self.transform.into_inner(),
            spread_method: self.spread_method.into_inner(),
            stops: self.stops.iter().map(|s| s.to_inner()).collect(),
            anti_alias: self.anti_alias,
        }
    }
}

/// A sweep (conic) gradient.
#[pyclass]
#[derive(Clone)]
pub struct SweepGradient {
    /// Center x coordinate.
    #[pyo3(get, set)]
    pub cx: f32,
    /// Center y coordinate.
    #[pyo3(get, set)]
    pub cy: f32,
    /// Start angle in degrees.
    #[pyo3(get, set)]
    pub start_angle: f32,
    /// End angle in degrees.
    #[pyo3(get, set)]
    pub end_angle: f32,
    /// Transformation applied to the gradient.
    #[pyo3(get, set)]
    pub transform: Transform,
    /// How the gradient spreads beyond its bounds.
    #[pyo3(get, set)]
    pub spread_method: SpreadMethod,
    /// Color stops.
    #[pyo3(get, set)]
    pub stops: Vec<Stop>,
    /// Whether to apply anti-aliasing.
    #[pyo3(get, set)]
    pub anti_alias: bool,
}

#[pymethods]
impl SweepGradient {
    /// Create a new sweep gradient.
    #[new]
    #[pyo3(signature = (cx, cy, start_angle, end_angle, stops, transform=None, spread_method=None, anti_alias=false))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        cx: f32,
        cy: f32,
        start_angle: f32,
        end_angle: f32,
        stops: Vec<Stop>,
        transform: Option<Transform>,
        spread_method: Option<SpreadMethod>,
        anti_alias: bool,
    ) -> Self {
        SweepGradient {
            cx,
            cy,
            start_angle,
            end_angle,
            transform: transform.unwrap_or_else(Transform::identity),
            spread_method: spread_method.unwrap_or(SpreadMethod::Pad),
            stops,
            anti_alias,
        }
    }

    /// Convert to a Paint.
    fn into_paint(&self) -> Paint {
        Paint::from_sweep_gradient(self)
    }

    fn __repr__(&self) -> String {
        format!(
            "SweepGradient(center=({}, {}), angles=({}, {}), {} stops)",
            self.cx,
            self.cy,
            self.start_angle,
            self.end_angle,
            self.stops.len()
        )
    }
}

impl SweepGradient {
    pub fn to_inner(&self) -> krilla::paint::SweepGradient {
        krilla::paint::SweepGradient {
            cx: self.cx,
            cy: self.cy,
            start_angle: self.start_angle,
            end_angle: self.end_angle,
            transform: self.transform.into_inner(),
            spread_method: self.spread_method.into_inner(),
            stops: self.stops.iter().map(|s| s.to_inner()).collect(),
            anti_alias: self.anti_alias,
        }
    }
}

/// Dash pattern for strokes.
#[pyclass]
#[derive(Clone)]
pub struct StrokeDash {
    /// Dash pattern array.
    #[pyo3(get, set)]
    pub array: Vec<f32>,
    /// Phase offset.
    #[pyo3(get, set)]
    pub offset: f32,
}

#[pymethods]
impl StrokeDash {
    /// Create a new stroke dash pattern.
    #[new]
    #[pyo3(signature = (array, offset=0.0))]
    fn new(array: Vec<f32>, offset: f32) -> Self {
        StrokeDash { array, offset }
    }

    fn __repr__(&self) -> String {
        format!("StrokeDash(array={:?}, offset={})", self.array, self.offset)
    }
}

impl StrokeDash {
    pub fn to_inner(&self) -> krilla::paint::StrokeDash {
        krilla::paint::StrokeDash {
            array: self.array.clone(),
            offset: self.offset,
        }
    }
}

/// Fill properties for drawing paths.
#[pyclass]
#[derive(Clone)]
pub struct Fill {
    /// The paint to use for filling.
    #[pyo3(get, set)]
    pub paint: Paint,
    /// Fill opacity.
    #[pyo3(get, set)]
    pub opacity: NormalizedF32,
    /// Fill rule.
    #[pyo3(get, set)]
    pub rule: FillRule,
}

#[pymethods]
impl Fill {
    /// Create a new fill.
    #[new]
    #[pyo3(signature = (paint, opacity=None, rule=None))]
    fn new(paint: Paint, opacity: Option<NormalizedF32>, rule: Option<FillRule>) -> Self {
        Fill {
            paint,
            opacity: opacity.unwrap_or(NormalizedF32::ONE),
            rule: rule.unwrap_or(FillRule::NonZero),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Fill(paint=..., opacity={}, rule={:?})",
            self.opacity.get(),
            self.rule
        )
    }
}

impl Fill {
    pub fn to_inner(&self) -> krilla::paint::Fill {
        krilla::paint::Fill {
            paint: self.paint.inner.clone(),
            opacity: self.opacity.into_inner(),
            rule: self.rule.into_inner(),
        }
    }

    pub fn from_inner(inner: &krilla::paint::Fill) -> Self {
        Fill {
            paint: Paint::from_inner(inner.paint.clone()),
            opacity: NormalizedF32::from_inner(inner.opacity),
            rule: FillRule::from_inner(inner.rule),
        }
    }
}

/// Stroke properties for drawing paths.
#[pyclass]
#[derive(Clone)]
pub struct Stroke {
    /// The paint to use for stroking.
    #[pyo3(get, set)]
    pub paint: Paint,
    /// Stroke width.
    #[pyo3(get, set)]
    pub width: f32,
    /// Miter limit.
    #[pyo3(get, set)]
    pub miter_limit: f32,
    /// Line cap style.
    #[pyo3(get, set)]
    pub line_cap: LineCap,
    /// Line join style.
    #[pyo3(get, set)]
    pub line_join: LineJoin,
    /// Stroke opacity.
    #[pyo3(get, set)]
    pub opacity: NormalizedF32,
    /// Optional dash pattern.
    #[pyo3(get, set)]
    pub dash: Option<StrokeDash>,
}

#[pymethods]
impl Stroke {
    /// Create a new stroke.
    #[new]
    #[pyo3(signature = (paint, width=None, miter_limit=None, line_cap=None, line_join=None, opacity=None, dash=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        paint: Paint,
        width: Option<f32>,
        miter_limit: Option<f32>,
        line_cap: Option<LineCap>,
        line_join: Option<LineJoin>,
        opacity: Option<NormalizedF32>,
        dash: Option<StrokeDash>,
    ) -> Self {
        Stroke {
            paint,
            width: width.unwrap_or(1.0),
            miter_limit: miter_limit.unwrap_or(10.0),
            line_cap: line_cap.unwrap_or(LineCap::Butt),
            line_join: line_join.unwrap_or(LineJoin::Miter),
            opacity: opacity.unwrap_or(NormalizedF32::ONE),
            dash,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Stroke(width={}, opacity={}, line_cap={:?}, line_join={:?})",
            self.width,
            self.opacity.get(),
            self.line_cap,
            self.line_join
        )
    }
}

impl Stroke {
    pub fn to_inner(&self) -> krilla::paint::Stroke {
        krilla::paint::Stroke {
            paint: self.paint.inner.clone(),
            width: self.width,
            miter_limit: self.miter_limit,
            line_cap: self.line_cap.into_inner(),
            line_join: self.line_join.into_inner(),
            opacity: self.opacity.into_inner(),
            dash: self.dash.as_ref().map(|d| d.to_inner()),
        }
    }

    pub fn from_inner(inner: &krilla::paint::Stroke) -> Self {
        Stroke {
            paint: Paint::from_inner(inner.paint.clone()),
            width: inner.width,
            miter_limit: inner.miter_limit,
            line_cap: LineCap::from_inner(inner.line_cap),
            line_join: LineJoin::from_inner(inner.line_join),
            opacity: NormalizedF32::from_inner(inner.opacity),
            dash: inner.dash.as_ref().map(|d| StrokeDash {
                array: d.array.clone(),
                offset: d.offset,
            }),
        }
    }
}
