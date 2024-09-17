//! Path-related properties.

use crate::color::rgb;
use crate::paint::Paint;
use std::hash::{Hash, Hasher};
use tiny_skia_path::NormalizedF32;
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

impl From<LineCap> for tiny_skia_path::LineCap {
    fn from(value: LineCap) -> Self {
        match value {
            LineCap::Butt => tiny_skia_path::LineCap::Butt,
            LineCap::Round => tiny_skia_path::LineCap::Round,
            LineCap::Square => tiny_skia_path::LineCap::Square,
        }
    }
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

impl From<LineJoin> for tiny_skia_path::LineJoin {
    fn from(value: LineJoin) -> Self {
        match value {
            LineJoin::Miter => tiny_skia_path::LineJoin::Miter,
            LineJoin::Round => tiny_skia_path::LineJoin::Round,
            LineJoin::Bevel => tiny_skia_path::LineJoin::Bevel,
        }
    }
}

/// A stroke dash.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeDash {
    /// The dash array.
    pub array: Vec<f32>,
    /// The offset of the dash.
    pub offset: f32,
}

impl Eq for StrokeDash {}

impl Hash for StrokeDash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for el in &self.array {
            el.to_bits().hash(state);
        }

        self.offset.to_bits().hash(state);
    }
}

/// A stroke.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    /// The paint of the stroke.
    pub paint: Paint,
    /// The width of the stroke.
    pub width: f32,
    /// The miter limit of the stroke.
    pub miter_limit: f32,
    /// The line cap of the stroke.
    pub line_cap: LineCap,
    /// The line join of the stroke.
    pub line_join: LineJoin,
    /// The opacity of the stroke.
    pub opacity: NormalizedF32,
    /// The (optional) dash of the stroke.
    pub dash: Option<StrokeDash>,
}

impl Eq for Stroke {}

impl Hash for Stroke {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.paint.hash(state);
        self.width.to_bits().hash(state);
        self.miter_limit.to_bits().hash(state);
        self.line_cap.hash(state);
        self.line_join.hash(state);
        self.opacity.hash(state);
        self.dash.hash(state);
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke {
            paint: rgb::Color::black().into(),
            width: 1.0,
            miter_limit: 10.0,
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
            width: self.width,
            miter_limit: self.miter_limit,
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
            stroke.dash = tiny_skia_path::StrokeDash::new(stroke_dash.array, stroke_dash.offset);
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
