//! Internal utilities.

use crate::path::{LineCap, LineJoin, Stroke};
use base64::Engine;
use pdf_writer::types::{LineCapStyle, LineJoinStyle};
use pdf_writer::Name;
use siphasher::sip128::{Hasher128, SipHasher13};
use std::any::Any;
use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
#[cfg(feature = "svg")]
use tiny_skia_path::PathBuilder;
use tiny_skia_path::{FiniteF32, Path, Rect, Size, Transform};

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
    #[cfg(feature = "svg")]
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

    #[cfg(feature = "svg")]
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

pub fn calculate_stroke_bbox(stroke: &Stroke, path: &Path) -> Option<Rect> {
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

pub trait HashExt {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H);
}

impl HashExt for Transform {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tx.to_bits().hash(state);
        self.ty.to_bits().hash(state);
        self.sx.to_bits().hash(state);
        self.sy.to_bits().hash(state);
        self.kx.to_bits().hash(state);
        self.ky.to_bits().hash(state);
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

/// Create a base64-encoded hash of the value.
pub(crate) fn hash_base64<T: Hash + ?Sized>(value: &T) -> String {
    base64::engine::general_purpose::STANDARD.encode(hash128(value).to_be_bytes())
}

/// Calculate a 128-bit siphash of a value.
pub(crate) fn hash128<T: Hash + ?Sized>(value: &T) -> u128 {
    let mut state = SipHasher13::new();
    value.hash(&mut state);
    state.finish128().as_u128()
}

/// Just a stub, until we re-add the `Deferred` functionality
/// with rayon.
pub(crate) struct Deferred<T>(T);

impl<T: Send + Sync + 'static> Deferred<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> T + Send + Sync + 'static,
    {
        Self(f())
    }

    pub fn wait(&self) -> &T {
        &self.0
    }
}

// /// A value that is lazily executed on another thread.
// ///
// /// Execution will be started in the background and can be waited on.
// pub(crate) struct Deferred<T>(Arc<OnceCell<T>>);
//
// impl<T: Send + Sync + 'static> Deferred<T> {
//     /// Creates a new deferred value.
//     ///
//     /// The closure will be called on a secondary thread such that the value
//     /// can be initialized in parallel.
//     pub fn new<F>(f: F) -> Self
//     where
//         F: FnOnce() -> T + Send + Sync + 'static,
//     {
//         let inner = Arc::new(OnceCell::new());
//         let cloned = Arc::clone(&inner);
//         rayon::spawn(move || {
//             // Initialize the value if it hasn't been initialized yet.
//             // We do this to avoid panicking in case it was set externally.
//             cloned.get_or_init(f);
//         });
//         Self(inner)
//     }
//
//     /// Waits on the value to be initialized.
//     ///
//     /// If the value has already been initialized, this will return
//     /// immediately. Otherwise, this will block until the value is
//     /// initialized in another thread.
//     pub fn wait(&self) -> &T {
//         // Fast path if the value is already available. We don't want to yield
//         // to rayon in that case.
//         if let Some(value) = self.0.get() {
//             return value;
//         }
//
//         // Ensure that we yield to give the deferred value a chance to compute
//         // single-threaded platforms (for WASM compatibility).
//         while let Some(rayon::Yield::Executed) = rayon::yield_now() {}
//
//         self.0.wait()
//     }
// }
