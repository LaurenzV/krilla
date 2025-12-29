//! Enum types for krilla Python bindings.

use pyo3::prelude::*;

/// Fill rule for determining the inside of a path.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FillRule {
    /// Non-zero winding rule (default).
    NonZero,
    /// Even-odd rule.
    EvenOdd,
}

impl FillRule {
    pub fn into_inner(self) -> krilla::paint::FillRule {
        match self {
            FillRule::NonZero => krilla::paint::FillRule::NonZero,
            FillRule::EvenOdd => krilla::paint::FillRule::EvenOdd,
        }
    }

    pub fn from_inner(inner: krilla::paint::FillRule) -> Self {
        match inner {
            krilla::paint::FillRule::NonZero => FillRule::NonZero,
            krilla::paint::FillRule::EvenOdd => FillRule::EvenOdd,
        }
    }
}

/// Line cap style for strokes.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineCap {
    /// Flat cap at the endpoint (default).
    Butt,
    /// Round cap centered at the endpoint.
    Round,
    /// Square cap extending beyond the endpoint.
    Square,
}

impl LineCap {
    pub fn into_inner(self) -> krilla::paint::LineCap {
        match self {
            LineCap::Butt => krilla::paint::LineCap::Butt,
            LineCap::Round => krilla::paint::LineCap::Round,
            LineCap::Square => krilla::paint::LineCap::Square,
        }
    }

    pub fn from_inner(inner: krilla::paint::LineCap) -> Self {
        match inner {
            krilla::paint::LineCap::Butt => LineCap::Butt,
            krilla::paint::LineCap::Round => LineCap::Round,
            krilla::paint::LineCap::Square => LineCap::Square,
        }
    }
}

/// Line join style for strokes.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineJoin {
    /// Sharp corner (default).
    Miter,
    /// Rounded corner.
    Round,
    /// Beveled corner.
    Bevel,
}

impl LineJoin {
    pub fn into_inner(self) -> krilla::paint::LineJoin {
        match self {
            LineJoin::Miter => krilla::paint::LineJoin::Miter,
            LineJoin::Round => krilla::paint::LineJoin::Round,
            LineJoin::Bevel => krilla::paint::LineJoin::Bevel,
        }
    }

    pub fn from_inner(inner: krilla::paint::LineJoin) -> Self {
        match inner {
            krilla::paint::LineJoin::Miter => LineJoin::Miter,
            krilla::paint::LineJoin::Round => LineJoin::Round,
            krilla::paint::LineJoin::Bevel => LineJoin::Bevel,
        }
    }
}

/// Spread method for gradients.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpreadMethod {
    /// Extend the edge color (default).
    Pad,
    /// Mirror the gradient.
    Reflect,
    /// Repeat the gradient.
    Repeat,
}

impl SpreadMethod {
    pub fn into_inner(self) -> krilla::paint::SpreadMethod {
        match self {
            SpreadMethod::Pad => krilla::paint::SpreadMethod::Pad,
            SpreadMethod::Reflect => krilla::paint::SpreadMethod::Reflect,
            SpreadMethod::Repeat => krilla::paint::SpreadMethod::Repeat,
        }
    }

    pub fn from_inner(inner: krilla::paint::SpreadMethod) -> Self {
        match inner {
            krilla::paint::SpreadMethod::Pad => SpreadMethod::Pad,
            krilla::paint::SpreadMethod::Reflect => SpreadMethod::Reflect,
            krilla::paint::SpreadMethod::Repeat => SpreadMethod::Repeat,
        }
    }
}

/// Blend mode for compositing.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Normal blending (default).
    Normal,
    /// Multiply.
    Multiply,
    /// Screen.
    Screen,
    /// Overlay.
    Overlay,
    /// Darken.
    Darken,
    /// Lighten.
    Lighten,
    /// Color dodge.
    ColorDodge,
    /// Color burn.
    ColorBurn,
    /// Hard light.
    HardLight,
    /// Soft light.
    SoftLight,
    /// Difference.
    Difference,
    /// Exclusion.
    Exclusion,
    /// Hue.
    Hue,
    /// Saturation.
    Saturation,
    /// Color.
    Color,
    /// Luminosity.
    Luminosity,
}

impl BlendMode {
    pub fn into_inner(self) -> krilla::blend::BlendMode {
        match self {
            BlendMode::Normal => krilla::blend::BlendMode::Normal,
            BlendMode::Multiply => krilla::blend::BlendMode::Multiply,
            BlendMode::Screen => krilla::blend::BlendMode::Screen,
            BlendMode::Overlay => krilla::blend::BlendMode::Overlay,
            BlendMode::Darken => krilla::blend::BlendMode::Darken,
            BlendMode::Lighten => krilla::blend::BlendMode::Lighten,
            BlendMode::ColorDodge => krilla::blend::BlendMode::ColorDodge,
            BlendMode::ColorBurn => krilla::blend::BlendMode::ColorBurn,
            BlendMode::HardLight => krilla::blend::BlendMode::HardLight,
            BlendMode::SoftLight => krilla::blend::BlendMode::SoftLight,
            BlendMode::Difference => krilla::blend::BlendMode::Difference,
            BlendMode::Exclusion => krilla::blend::BlendMode::Exclusion,
            BlendMode::Hue => krilla::blend::BlendMode::Hue,
            BlendMode::Saturation => krilla::blend::BlendMode::Saturation,
            BlendMode::Color => krilla::blend::BlendMode::Color,
            BlendMode::Luminosity => krilla::blend::BlendMode::Luminosity,
        }
    }

    #[allow(dead_code)]
    pub fn from_inner(inner: krilla::blend::BlendMode) -> Self {
        match inner {
            krilla::blend::BlendMode::Normal => BlendMode::Normal,
            krilla::blend::BlendMode::Multiply => BlendMode::Multiply,
            krilla::blend::BlendMode::Screen => BlendMode::Screen,
            krilla::blend::BlendMode::Overlay => BlendMode::Overlay,
            krilla::blend::BlendMode::Darken => BlendMode::Darken,
            krilla::blend::BlendMode::Lighten => BlendMode::Lighten,
            krilla::blend::BlendMode::ColorDodge => BlendMode::ColorDodge,
            krilla::blend::BlendMode::ColorBurn => BlendMode::ColorBurn,
            krilla::blend::BlendMode::HardLight => BlendMode::HardLight,
            krilla::blend::BlendMode::SoftLight => BlendMode::SoftLight,
            krilla::blend::BlendMode::Difference => BlendMode::Difference,
            krilla::blend::BlendMode::Exclusion => BlendMode::Exclusion,
            krilla::blend::BlendMode::Hue => BlendMode::Hue,
            krilla::blend::BlendMode::Saturation => BlendMode::Saturation,
            krilla::blend::BlendMode::Color => BlendMode::Color,
            krilla::blend::BlendMode::Luminosity => BlendMode::Luminosity,
        }
    }
}

/// Mask type.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MaskType {
    /// Luminosity mask.
    Luminosity,
    /// Alpha mask.
    Alpha,
}

impl MaskType {
    pub fn into_inner(self) -> krilla::mask::MaskType {
        match self {
            MaskType::Luminosity => krilla::mask::MaskType::Luminosity,
            MaskType::Alpha => krilla::mask::MaskType::Alpha,
        }
    }

    #[allow(dead_code)]
    pub fn from_inner(inner: krilla::mask::MaskType) -> Self {
        match inner {
            krilla::mask::MaskType::Luminosity => MaskType::Luminosity,
            krilla::mask::MaskType::Alpha => MaskType::Alpha,
        }
    }
}
