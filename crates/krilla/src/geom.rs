//! Geometrical helper structs.

pub use tiny_skia_path::Transform;

/// An immutable, finite `f32` in a 0..=1 range.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[repr(transparent)]
pub struct NormalizedF32(tiny_skia_path::NormalizedF32);

impl NormalizedF32 {
    /// A `NormalizedF32` value initialized with zero.
    pub const ZERO: Self = NormalizedF32(tiny_skia_path::NormalizedF32::ZERO);
    /// A `NormalizedF32` value initialized with one.
    pub const ONE: Self = NormalizedF32(tiny_skia_path::NormalizedF32::ONE);

    /// Create a new normalized f32.
    ///
    /// Panics if the number is not normalized.
    pub fn new(num: f32) -> Option<Self> {
        Some(Self(tiny_skia_path::NormalizedF32::new(num)?))
    }

    /// Returns the value as a primitive type.
    #[inline]
    pub const fn get(self) -> f32 {
        self.0.get()
    }
}

/// A point.
#[allow(missing_docs)]
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    /// Create a new point with the given x and y coordinates.
    pub fn from_xy(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub(crate) fn to_tsp(&self) -> tiny_skia_path::Point {
        tiny_skia_path::Point::from_xy(self.x, self.y)
    }
}

/// A size.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Size(tiny_skia_path::Size);

impl Size {
    /// Creates a new `Size` from width and height.
    ///
    /// Returns `None` if either the width or the height is not > 0.
    pub fn from_wh(width: f32, height: f32) -> Option<Self> {
        Some(Self(tiny_skia_path::Size::from_wh(width, height)?))
    }

    /// Returns the width of the size.
    pub fn width(&self) -> f32 {
        self.0.width()
    }

    /// Returns the height of the size.
    pub fn height(&self) -> f32 {
        self.0.height()
    }
}

/// A rectangle defined by left, top, right and bottom edges.
#[allow(missing_docs)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Rect(tiny_skia_path::Rect);

impl Rect {
    /// Creates new `Rect`.
    pub fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Option<Self> {
        Some(Self(tiny_skia_path::Rect::from_ltrb(
            left, top, right, bottom,
        )?))
    }

    /// Creates new `Rect`.
    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Option<Self> {
        Some(Self(tiny_skia_path::Rect::from_xywh(x, y, w, h)?))
    }

    /// Returns the left edge.
    pub fn left(&self) -> f32 {
        self.0.left()
    }

    /// Returns the top edge.
    pub fn top(&self) -> f32 {
        self.0.top()
    }

    /// Returns the right edge.
    pub fn right(&self) -> f32 {
        self.0.right()
    }

    /// Returns the bottom edge.
    pub fn bottom(&self) -> f32 {
        self.0.bottom()
    }

    /// Returns rect's width.
    pub fn width(&self) -> f32 {
        self.0.width()
    }

    /// Returns rect's height.
    pub fn height(&self) -> f32 {
        self.0.height()
    }

    pub(crate) fn to_tsp(&self) -> tiny_skia_path::Rect {
        self.0
    }
}
