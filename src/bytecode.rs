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
    PushLayer,
    PopLayer,
    Transform(TransformWrapper),
    ClipPath(Box<(PathWrapper, FillRule)>),
    StrokePath(Box<(PathWrapper, TransformWrapper, Stroke)>),
    DrawImage(Box<(Image, Size, TransformWrapper)>),
    FillPath(Box<(PathWrapper, TransformWrapper, Fill)>),
    DrawCanvas(
        Box<(
            Arc<Canvas>,
            TransformWrapper,
            BlendMode,
            NormalizedF32,
            bool,
            Option<Mask>,
        )>,
    ),
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ByteCode(Vec<Instruction>);

impl ByteCode {
    pub fn new() -> Self {
        Self(Vec::with_capacity(32))
    }

    pub fn push(&mut self, op: Instruction) {
        self.0.push(op);
    }

    pub fn instructions(&self) -> &[Instruction] {
        self.0.as_slice()
    }
}
