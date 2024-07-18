use crate::{LineCap, LineJoin};
use pdf_writer::types::{LineCapStyle, LineJoinStyle};
use pdf_writer::Name;
use tiny_skia_path::Rect;

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
}
