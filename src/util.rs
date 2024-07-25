use crate::{LineCap, LineJoin, Stroke};
use pdf_writer::types::{LineCapStyle, LineJoinStyle};
use pdf_writer::Name;
use siphasher::sip128::{Hasher128, SipHasher13};
use std::any::Any;
use std::fmt;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use tiny_skia_path::{Path, PathBuilder, Rect};

pub fn deflate(data: &[u8]) -> Vec<u8> {
    const COMPRESSION_LEVEL: u8 = 6;
    miniz_oxide::deflate::compress_to_vec_zlib(data, COMPRESSION_LEVEL)
}

pub trait NameExt {
    fn to_pdf_name(&self) -> Name;
}

impl NameExt for String {
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

pub fn calculate_stroke_bbox(stroke: &Stroke, path: &tiny_skia_path::Path) -> Option<Rect> {
    let stroke = stroke.to_tiny_skia();

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
        let hash = hash_item(&value);
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

fn hash_item<T: Hash + ?Sized + 'static>(item: &T) -> u128 {
    // Also hash the TypeId because the type might be converted
    // through an unsized coercion.
    let mut state = SipHasher13::new();
    item.type_id().hash(&mut state);
    item.hash(&mut state);
    state.finish128().as_u128()
}
