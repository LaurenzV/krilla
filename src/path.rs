use crate::color::Color;
use crate::paint::Paint;
use std::hash::{Hash, Hasher};
use tiny_skia_path::{FiniteF32, NonZeroPositiveF32, NormalizedF32};
pub use tiny_skia_path::{Path, PathBuilder};

#[derive(Eq, PartialEq, Debug, Hash, Clone, Copy)]
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

#[derive(PartialEq, Eq, Debug, Hash, Clone, Copy)]
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

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct StrokeDash {
    pub array: Vec<FiniteF32>,
    pub offset: FiniteF32,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Stroke {
    pub paint: Paint,
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
            paint: Paint::Color(Color::black()),
            width: NonZeroPositiveF32::new(1.0).unwrap(),
            miter_limit: NonZeroPositiveF32::new(10.0).unwrap(),
            line_cap: LineCap::default(),
            line_join: LineJoin::default(),
            opacity: NormalizedF32::ONE,
            dash: None,
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

impl Default for FillRule {
    fn default() -> Self {
        Self::NonZero
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct PathWrapper(pub Path);

// We don't care about NaNs.
impl Eq for PathWrapper {}

impl Hash for PathWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.verbs().hash(state);

        for point in self.0.points() {
            debug_assert!(!point.x.is_nan());
            debug_assert!(!point.y.is_nan());

            point.x.to_bits().hash(state);
            point.y.to_bits().hash(state);
        }

        self.0.bounds().hash(state);
    }
}

impl Into<PathWrapper> for Path {
    fn into(self) -> PathWrapper {
        PathWrapper(self)
    }
}
