use crate::canvas::Canvas;
use crate::ext_g_state::CompositeMode;
use crate::transform::FiniteTransform;
use crate::{Fill, FillRule, PathWrapper, Stroke};
use tiny_skia_path::{NormalizedF32, Path, Transform};

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum Instruction {
    PushLayer,
    PopLayer,
    ClipPath(Box<(PathWrapper, FillRule)>),
    StrokePath(Box<(PathWrapper, FiniteTransform, Stroke)>),
    FillPath(Box<(PathWrapper, FiniteTransform, Fill)>),
    DrawCanvas(Box<(Canvas, FiniteTransform, CompositeMode, NormalizedF32, bool)>),
}

#[derive(Debug, Hash, Eq, PartialEq)]
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
