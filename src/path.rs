//! Path-related properties.

use crate::color::rgb;
use crate::paint::Paint;
use crate::util::F32Wrapper;
use tiny_skia_path::{FiniteF32, NormalizedF32};
pub use tiny_skia_path::{Path, PathBuilder};

/// A line cap.
#[derive(Eq, PartialEq, Debug, Clone, Copy, Default, Hash)]
pub enum LineCap {
    /// The butt line cap.
    #[default]
    Butt,
    /// The round line cap.
    Round,
    /// The square line cap.
    Square,
}

/// A line join.
#[derive(PartialEq, Eq, Debug, Clone, Copy, Default, Hash)]
pub enum LineJoin {
    /// The miter line join.
    #[default]
    Miter,
    /// The round line join.
    Round,
    /// The bevel line join.
    Bevel,
}

/// A stroke dash.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct StrokeDash {
    /// The dash array.
    pub array: Vec<F32Wrapper>,
    /// The offset of the dash.
    pub offset: F32Wrapper,
}

impl StrokeDash {
    /// Create a new stroke dash.
    pub fn new(array: impl IntoIterator<Item = f32>, offset: f32) -> Self {
        Self {
            array: array.into_iter().map(|n| F32Wrapper(n)).collect::<Vec<_>>(),
            offset: F32Wrapper(offset),
        }
    }
}

/// A stroke.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Stroke {
    /// The paint of the stroke.
    pub paint: Paint,
    /// The width of the stroke.
    pub width: F32Wrapper,
    /// The miter limit of the stroke.
    pub miter_limit: F32Wrapper,
    /// The line cap of the stroke.
    pub line_cap: LineCap,
    /// The line join of the stroke.
    pub line_join: LineJoin,
    /// The opacity of the stroke.
    pub opacity: NormalizedF32,
    /// The (optional) dash of the stroke.
    pub dash: Option<StrokeDash>,
}

impl Stroke {
    /// Create a new stroke.
    pub fn new(
        paint: Paint,
        width: f32,
        miter_limit: f32,
        line_cap: LineCap,
        line_join: LineJoin,
        opacity: NormalizedF32,
        dash: Option<StrokeDash>,
    ) -> Self {
        Self {
            paint,
            width: F32Wrapper(width),
            miter_limit: F32Wrapper(miter_limit),
            line_cap,
            line_join,
            opacity,
            dash,
        }
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke {
            paint: rgb::Color::black().into(),
            width: F32Wrapper(1.0),
            miter_limit: F32Wrapper(10.0),
            line_cap: LineCap::default(),
            line_join: LineJoin::default(),
            opacity: NormalizedF32::ONE,
            dash: None,
        }
    }
}

impl Stroke {
    pub(crate) fn into_tiny_skia(self) -> tiny_skia_path::Stroke {
        let mut stroke = tiny_skia_path::Stroke {
            width: self.width.0,
            miter_limit: self.miter_limit.0,
            line_cap: match self.line_cap {
                LineCap::Butt => tiny_skia_path::LineCap::Butt,
                LineCap::Round => tiny_skia_path::LineCap::Round,
                LineCap::Square => tiny_skia_path::LineCap::Square,
            },
            line_join: match self.line_join {
                LineJoin::Miter => tiny_skia_path::LineJoin::Miter,
                LineJoin::Round => tiny_skia_path::LineJoin::Round,
                LineJoin::Bevel => tiny_skia_path::LineJoin::Bevel,
            },
            dash: None,
        };

        if let Some(stroke_dash) = self.dash {
            stroke.dash = tiny_skia_path::StrokeDash::new(
                stroke_dash.array.iter().map(|n| n.0).collect::<Vec<_>>(),
                stroke_dash.offset.0,
            );
        }

        stroke
    }
}

/// A fill rule.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub enum FillRule {
    /// The `non-zero` fill rule.
    NonZero,
    /// The `even-odd` fill rule.
    EvenOdd,
}

impl Default for FillRule {
    fn default() -> Self {
        Self::NonZero
    }
}

/// A fill.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fill {
    /// The paint of the fill.
    pub paint: Paint,
    /// The opacity of the fill.
    pub opacity: NormalizedF32,
    /// The fill rule that should be used when applying the fill.
    pub rule: FillRule,
}

impl Default for Fill {
    fn default() -> Self {
        Fill {
            paint: rgb::Color::black().into(),
            opacity: NormalizedF32::ONE,
            rule: FillRule::default(),
        }
    }
}
