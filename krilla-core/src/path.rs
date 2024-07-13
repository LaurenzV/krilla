use crate::color::Color;
use crate::paint::Paint;
use strict_num::{NonZeroPositiveF32, NormalizedF32, NormalizedF64};
use tiny_skia_path::FiniteF32;
pub use tiny_skia_path::{Path, PathBuilder};

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

pub struct StrokeDash {
    pub array: Vec<f32>,
    pub offset: f32,
}

pub struct Stroke {
    pub width: NonZeroPositiveF32,
    pub miter_limit: NonZeroPositiveF32,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub opacity: NormalizedF32,
    pub dash: Option<StrokeDash>,
}

impl Default for Stroke {
    fn default() -> Self {
        Stroke {
            width: NonZeroPositiveF32::new(1.0).unwrap(),
            miter_limit: NonZeroPositiveF32::new(4.0).unwrap(),
            line_cap: LineCap::default(),
            line_join: LineJoin::default(),
            opacity: NormalizedF32::ONE,
            dash: None,
        }
    }
}

pub enum FillRule {
    NonZero,
    EvenOdd,
}

impl Default for FillRule {
    fn default() -> Self {
        Self::NonZero
    }
}

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
