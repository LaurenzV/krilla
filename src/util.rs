//! Internal utilities.

use crate::object::color::ColorSpace;
use crate::path::{LineCap, LineJoin, Stroke};
use pdf_writer::types::{LineCapStyle, LineJoinStyle};
use pdf_writer::Name;
use siphasher::sip128::{Hasher128, SipHasher13};
use skrifa::instance::Location;
use std::any::Any;
use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use tiny_skia_path::{FiniteF32, Path, PathBuilder, Rect, Size, Transform};

pub trait NameExt {
    fn to_pdf_name(&self) -> Name;
}

impl NameExt for String {
    fn to_pdf_name(&self) -> Name {
        Name(self.as_bytes())
    }
}

impl NameExt for &str {
    fn to_pdf_name(&self) -> Name {
        Name(self.as_bytes())
    }
}

pub trait TransformExt {
    fn to_pdf_transform(&self) -> [f32; 6];
}

impl TransformExt for tiny_skia_path::Transform {
    fn to_pdf_transform(&self) -> [f32; 6] {
        [self.sx, self.ky, self.kx, self.sy, self.tx, self.ty]
    }
}

pub trait LineCapExt {
    fn to_pdf_line_cap(&self) -> LineCapStyle;
}

impl LineCapExt for LineCap {
    fn to_pdf_line_cap(&self) -> LineCapStyle {
        match self {
            LineCap::Butt => LineCapStyle::ButtCap,
            LineCap::Round => LineCapStyle::RoundCap,
            LineCap::Square => LineCapStyle::ProjectingSquareCap,
        }
    }
}

pub trait LineJoinExt {
    fn to_pdf_line_join(&self) -> LineJoinStyle;
}

impl LineJoinExt for LineJoin {
    fn to_pdf_line_join(&self) -> LineJoinStyle {
        match self {
            LineJoin::Miter => LineJoinStyle::MiterJoin,
            LineJoin::Round => LineJoinStyle::RoundJoin,
            LineJoin::Bevel => LineJoinStyle::BevelJoin,
        }
    }
}

pub trait RectExt {
    fn expand(&mut self, other: &Rect);
    fn to_pdf_rect(&self) -> pdf_writer::Rect;
    fn to_clip_path(&self) -> Path;
}

impl RectExt for Rect {
    fn expand(&mut self, other: &Rect) {
        let left = self.left().min(other.left());
        let top = self.top().min(other.top());
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        *self = Rect::from_ltrb(left, top, right, bottom).unwrap();
    }

    fn to_pdf_rect(&self) -> pdf_writer::Rect {
        pdf_writer::Rect::new(
            self.x(),
            self.y(),
            self.x() + self.width(),
            self.y() + self.height(),
        )
    }

    fn to_clip_path(&self) -> Path {
        let mut path_builder = PathBuilder::new();
        path_builder.move_to(self.left(), self.top());
        path_builder.line_to(self.right(), self.top());
        path_builder.line_to(self.right(), self.bottom());
        path_builder.line_to(self.left(), self.bottom());
        path_builder.close();
        path_builder.finish().unwrap()
    }
}

pub fn calculate_stroke_bbox(stroke: &Stroke<impl ColorSpace>, path: &Path) -> Option<Rect> {
    let stroke = stroke.clone().into_tiny_skia();

    if let Some(stroked_path) = path.stroke(&stroke, 1.0) {
        return stroked_path.compute_tight_bounds();
    }

    None
}

pub struct Prehashed<T: ?Sized> {
    hash: u128,
    value: T,
}

impl<T: Hash + 'static> Prehashed<T> {
    #[inline]
    pub fn new(value: T) -> Self {
        let hash = value.sip_hash();
        Self { hash, value }
    }
}

impl<T: Hash + ?Sized + 'static> Eq for Prehashed<T> {}

impl<T: Hash + ?Sized + 'static> PartialEq for Prehashed<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl<T: ?Sized> Deref for Prehashed<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Hash + Clone + 'static> Clone for Prehashed<T> {
    fn clone(&self) -> Self {
        Self {
            hash: self.hash,
            value: self.value.clone(),
        }
    }
}

impl<T: Debug> Debug for Prehashed<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl<T: Hash + ?Sized + 'static> Hash for Prehashed<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u128(self.hash);
    }
}

/// Extra methods for [`[T]`](slice).
pub trait SliceExt<T> {
    /// Split a slice into consecutive runs with the same key and yield for
    /// each such run the key and the slice of elements with that key.
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F>
    where
        F: FnMut(&T) -> K,
        K: PartialEq;
}

impl<T> SliceExt<T> for [T] {
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F> {
        GroupByKey { slice: self, f }
    }
}

/// This struct is created by [`SliceExt::group_by_key`].
pub struct GroupByKey<'a, T, F> {
    slice: &'a [T],
    f: F,
}

impl<'a, T, K, F> Iterator for GroupByKey<'a, T, F>
where
    F: FnMut(&T) -> K,
    K: PartialEq,
{
    type Item = (K, &'a [T]);

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.slice.iter();
        let key = (self.f)(iter.next()?);
        let count = 1 + iter.take_while(|t| (self.f)(t) == key).count();
        let (head, tail) = self.slice.split_at(count);
        self.slice = tail;
        Some((key, head))
    }
}

// TODO: Remove with new resvg release
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct RectWrapper(pub Rect);

impl Deref for RectWrapper {
    type Target = Rect;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Eq for RectWrapper {}

impl Hash for RectWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        FiniteF32::new(self.0.left()).unwrap().hash(state);
        FiniteF32::new(self.0.top()).unwrap().hash(state);
        FiniteF32::new(self.0.right()).unwrap().hash(state);
        FiniteF32::new(self.0.bottom()).unwrap().hash(state);
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct SizeWrapper(pub Size);

impl Deref for SizeWrapper {
    type Target = Size;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Eq for SizeWrapper {}

impl Hash for SizeWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        FiniteF32::new(self.0.width()).unwrap().hash(state);
        FiniteF32::new(self.0.height()).unwrap().hash(state);
    }
}

#[derive(Debug)]
pub struct LocationWrapper(pub Location);

impl Hash for LocationWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.coords().hash(state);
    }
}

impl PartialEq for LocationWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.coords().eq(other.0.coords())
    }
}

impl Eq for LocationWrapper {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformWrapper(pub(crate) Transform);

// We don't care about NaNs.
impl Eq for TransformWrapper {}

impl Hash for TransformWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.tx.to_bits().hash(state);
        self.0.ty.to_bits().hash(state);
        self.0.sx.to_bits().hash(state);
        self.0.sy.to_bits().hash(state);
        self.0.kx.to_bits().hash(state);
        self.0.ky.to_bits().hash(state);
    }
}

pub trait SipHashable {
    fn sip_hash(&self) -> u128;
}

impl<T> SipHashable for T
where
    T: Hash + ?Sized + 'static,
{
    fn sip_hash(&self) -> u128 {
        let mut state = SipHasher13::new();
        self.type_id().hash(&mut state);
        self.hash(&mut state);
        state.finish128().as_u128()
    }
}
