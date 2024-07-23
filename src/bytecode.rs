use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_function::ShadingFunction;
use crate::transform::TransformWrapper;
use crate::{Color, Fill, FillRule, Paint, PathWrapper, Stroke};
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
    // TODO: Remove?
    Shaded(Box<ShadingFunction>),
    Clipped(Box<(Vec<PathWrapper>, FillRule, ByteCode)>),
    Masked(Box<(Mask, ByteCode)>),
    Opacified(Box<(NormalizedF32, ByteCode)>),
}

// TODO: Make cheap to clone?
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ByteCode(pub(crate) Vec<Instruction>);

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

pub fn into_composited(instructions: &[Instruction], black: bool) -> Vec<Instruction> {
    let mut new_instructions = vec![];

    let paint = if black {
        Paint::Color(Color::black())
    } else {
        Paint::Color(Color::white())
    };

    for instruction in instructions {
        match instruction {
            Instruction::Transformed(t) => {
                new_instructions.push(Instruction::Transformed(t.clone()))
            }
            Instruction::Isolated(i) => new_instructions.extend(into_composited(&i.0, black)),
            Instruction::Blended(b) => new_instructions.extend(into_composited(&b.1 .0, black)),
            Instruction::StrokePath(s) => {
                let stroke = Stroke {
                    paint: paint.clone(),
                    width: s.1.width,
                    miter_limit: s.1.miter_limit,
                    line_cap: s.1.line_cap,
                    line_join: s.1.line_join,
                    opacity: s.1.opacity,
                    dash: s.1.dash.clone(),
                };

                new_instructions.push(Instruction::StrokePath(Box::new((s.0.clone(), stroke))));
            }
            Instruction::FillPath(f) => {
                let fill = Fill {
                    paint: paint.clone(),
                    opacity: f.1.opacity,
                    rule: f.1.rule,
                };

                new_instructions.push(Instruction::FillPath(Box::new((f.0.clone(), fill))))
            }
            // TODO: Add
            Instruction::DrawImage(_) => {}
            Instruction::Shaded(_) => {}
            Instruction::Clipped(_) => {}
            Instruction::Masked(_) => {}
            Instruction::Opacified(_) => {}
        }
    }

    new_instructions
}
