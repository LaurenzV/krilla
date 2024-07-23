use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_function::ShadingFunction;
use crate::transform::TransformWrapper;
use crate::{Fill, FillRule, PathWrapper, Stroke};
use pdf_writer::types::BlendMode;
use std::sync::Arc;
use tiny_skia_path::{NormalizedF32, Size};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum Instruction {
    Transformed(Box<(TransformWrapper, ByteCode)>),
    Isolated(Arc<ByteCode>),
    Blended(Box<(BlendMode, ByteCode)>),
    StrokePath(Box<(PathWrapper, Stroke)>),
    DrawImage(Box<(Image, Size)>),
    FillPath(Box<(PathWrapper, Fill)>),
    Shaded(Box<(ShadingFunction, ByteCode)>),
    Clipped(Box<(Vec<PathWrapper>, FillRule, ByteCode)>),
    Masked(Box<(Mask, ByteCode)>),
    Opacified(Box<(NormalizedF32, ByteCode)>),
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
