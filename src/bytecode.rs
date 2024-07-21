use crate::canvas::Canvas;
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::transform::TransformWrapper;
use crate::{Fill, FillRule, PathWrapper, Stroke};
use pdf_writer::types::BlendMode;
use std::sync::Arc;
use tiny_skia_path::{NormalizedF32, Path, Size, Transform};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum Instruction {
    Transformed(Box<(TransformWrapper, ByteCode)>),
    Isolated(Box<ByteCode>),
    Blended(Box<(BlendMode, ByteCode)>),
    StrokePath(Box<(PathWrapper, Stroke)>),
    DrawImage(Box<(Image, Size)>),
    FillPath(Box<(PathWrapper, Fill)>),
    Clipped(Box<(PathWrapper, FillRule, ByteCode)>),
    Masked(Box<(Mask, ByteCode)>),
}

// TODO: Make cheap to clone?
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ByteCode(Vec<Instruction>);

impl ByteCode {
    pub fn new() -> Self {
        Self(Vec::with_capacity(32))
    }

    pub fn push(&mut self, op: Instruction) {
        self.0.push(op);
    }

    pub fn extend(&mut self, other: &ByteCode) {
        self.0.extend(other.instructions().iter().cloned());
    }

    pub fn instructions(&self) -> &[Instruction] {
        self.0.as_slice()
    }
}
