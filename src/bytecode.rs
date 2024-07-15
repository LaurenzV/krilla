use crate::canvas::Canvas;
use crate::ext_g_state::CompositeMode;
use crate::{Fill, Stroke};
use tiny_skia_path::{NormalizedF32, Path, Transform};

pub enum Instruction {
    PushLayer,
    PopLayer,
    StrokePath(Box<(Path, Transform, Stroke)>),
    FillPath(Box<(Path, Transform, Fill)>),
    DrawCanvas(Box<(Canvas, Transform, CompositeMode, NormalizedF32, bool)>),
}

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
