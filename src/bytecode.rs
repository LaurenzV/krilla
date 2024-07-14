use crate::canvas::Canvas;
use crate::{Fill, Stroke};
use tiny_skia_path::Path;

pub enum Instruction {
    SaveState,
    RestoreState,
    StrokePath(Box<(Path, tiny_skia_path::Transform, Stroke)>),
    FillPath(Box<(Path, tiny_skia_path::Transform, Fill)>),
    DrawCanvas(Box<Canvas>),
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
