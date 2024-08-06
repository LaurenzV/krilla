use crate::color::Color;
use crate::paint::Paint;
use std::hash::{Hash, Hasher};
use tiny_skia_path::{FiniteF32, NonZeroPositiveF32, NormalizedF32};
pub use tiny_skia_path::{Path, PathBuilder};

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

impl Default for LineCap {
    fn default() -> Self {
        LineCap::Butt
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum LineJoin {
    Miter,
    Round,
    Bevel,
}

impl Default for LineJoin {
    fn default() -> Self {
        LineJoin::Miter
    }
}

#[derive(Debug, Clone)]
pub struct StrokeDash {
    pub array: Vec<FiniteF32>,
    pub offset: FiniteF32,
}

#[derive(Debug, Clone)]
pub struct Stroke {
    pub paint: Paint,
    pub width: f32,
    pub miter_limit: f32,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub opacity: NormalizedF32,
    pub dash: Option<StrokeDash>,
}

impl Stroke {
    pub fn to_tiny_skia(&self) -> tiny_skia_path::Stroke {
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

        if let Some(ref stroke_dash) = self.dash {
            stroke.dash = tiny_skia_path::StrokeDash::new(
                stroke_dash
                    .array
                    .iter()
                    .map(|n| n.get())
                    .collect::<Vec<_>>(),
                stroke_dash.offset.get(),
            );
        }

        stroke
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke {
            paint: Paint::Color(Color::black()),
            width: 1.0,
            miter_limit: 10.0,
            line_cap: LineCap::default(),
            line_join: LineJoin::default(),
            opacity: NormalizedF32::ONE,
            dash: None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

impl Default for FillRule {
    fn default() -> Self {
        Self::NonZero
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Fill {
    pub paint: Paint,
    pub opacity: NormalizedF32,
    pub rule: FillRule,
}

impl Default for Fill {
    fn default() -> Self {
        Fill {
            paint: Paint::Color(Color::black()),
            opacity: NormalizedF32::ONE,
            rule: FillRule::default(),
        }
    }
}