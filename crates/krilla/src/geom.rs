//! Geometrical helper structs.

use std::hash::{Hash, Hasher};

use crate::util::{HashExt, TransformExt};

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

    pub(crate) fn to_tsp(self) -> tiny_skia_path::Point {
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

impl Eq for Rect {}

impl Hash for Rect {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.left().to_bits().hash(state);
        self.0.top().to_bits().hash(state);
        self.0.right().to_bits().hash(state);
        self.0.bottom().to_bits().hash(state);
    }
}

impl Rect {
    /// Creates new `Rect`.
    pub fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Option<Self> {
        Some(Self(tiny_skia_path::Rect::from_ltrb(
            left, top, right, bottom,
        )?))
    }

    /// Apply a transform to the rect.
    pub fn transform(self, transform: Transform) -> Option<Self> {
        Some(Self(self.0.transform(transform.to_tsp())?))
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

    pub(crate) fn to_tsp(self) -> tiny_skia_path::Rect {
        self.0
    }

    pub(crate) fn from_tsp(rect: tiny_skia_path::Rect) -> Self {
        Self(rect)
    }
}

/// An affine transformation matrix.
///
/// Unlike other types, doesn't guarantee to be valid. This is Skia quirk.
/// Meaning Transform(0, 0, 0, 0, 0, 0) is ok, while it's technically not.
/// Non-finite values are also not an error.
#[allow(missing_docs)]
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Transform(tiny_skia_path::Transform);

impl Transform {
    /// Creates an identity transform.
    pub fn identity() -> Self {
        Self(tiny_skia_path::Transform::default())
    }

    /// Creates a new `Transform`.
    pub fn from_row(sx: f32, ky: f32, kx: f32, sy: f32, tx: f32, ty: f32) -> Self {
        Self(tiny_skia_path::Transform::from_row(sx, ky, kx, sy, tx, ty))
    }

    /// Creates a new translating `Transform`.
    pub fn from_translate(tx: f32, ty: f32) -> Self {
        Self(tiny_skia_path::Transform::from_translate(tx, ty))
    }

    /// Creates a new scaling `Transform`.
    pub fn from_scale(sx: f32, sy: f32) -> Self {
        Self(tiny_skia_path::Transform::from_scale(sx, sy))
    }

    /// Creates a new skewing `Transform`.
    pub fn from_skew(kx: f32, ky: f32) -> Self {
        Self(tiny_skia_path::Transform::from_skew(kx, ky))
    }

    /// Creates a new rotating `Transform`.
    ///
    /// `angle` in degrees.
    pub fn from_rotate(angle: f32) -> Self {
        Self(tiny_skia_path::Transform::from_rotate(angle))
    }

    /// Creates a new rotating `Transform` at the specified position.
    ///
    /// `angle` in degrees.
    pub fn from_rotate_at(angle: f32, tx: f32, ty: f32) -> Self {
        Self(tiny_skia_path::Transform::from_rotate_at(angle, tx, ty))
    }

    /// Return the `sx` component
    pub fn sx(&self) -> f32 {
        self.0.sx
    }

    /// Return the `sy` component
    pub fn sy(&self) -> f32 {
        self.0.sy
    }

    /// Return the `kx` component
    pub fn kx(&self) -> f32 {
        self.0.kx
    }

    /// Return the `kx` component
    pub fn ky(&self) -> f32 {
        self.0.ky
    }

    /// Return the `tx` component
    pub fn tx(&self) -> f32 {
        self.0.tx
    }

    /// Return the `ty` component
    pub fn ty(&self) -> f32 {
        self.0.ty
    }

    /// Returns the inverted transform.
    pub fn invert(&self) -> Option<Self> {
        Some(Self(self.0.invert()?))
    }

    pub(crate) fn pre_concat(&self, other: Self) -> Self {
        Self(self.0.pre_concat(other.0))
    }

    pub(crate) fn post_concat(&self, other: Self) -> Self {
        Self(self.0.post_concat(other.0))
    }

    pub(crate) fn to_tsp(self) -> tiny_skia_path::Transform {
        self.0
    }

    pub(crate) fn from_tsp(ts: tiny_skia_path::Transform) -> Self {
        Self(ts)
    }
}

// TODO: Implement dircetly
impl HashExt for Transform {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.tx.to_bits().hash(state);
        self.0.ty.to_bits().hash(state);
        self.0.sx.to_bits().hash(state);
        self.0.sy.to_bits().hash(state);
        self.0.kx.to_bits().hash(state);
        self.0.ky.to_bits().hash(state);
    }
}

impl TransformExt for Transform {
    fn to_pdf_transform(&self) -> [f32; 6] {
        [
            self.0.sx, self.0.ky, self.0.kx, self.0.sy, self.0.tx, self.0.ty,
        ]
    }
}
