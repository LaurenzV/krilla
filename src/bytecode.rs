use crate::blend_mode::BlendMode;
use crate::font::Font;
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_function::ShadingFunction;
use crate::transform::TransformWrapper;
use crate::util::RectExt;
use crate::{blend_mode, Color, Fill, FillRule, Paint, PathWrapper, Stroke};
use skrifa::GlyphId;
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, NormalizedF32, Rect, Size, Transform};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum Instruction {
    Transformed(Box<(TransformWrapper, ByteCode)>),
    Isolated(Arc<ByteCode>),
    // TODO: Replace with PDF blend mode
    Blended(Box<(BlendMode, ByteCode)>),
    StrokePath(Box<(PathWrapper, Stroke)>),
    DrawGlyph(Box<(GlyphId, Font, FiniteF32)>),
    DrawImage(Box<(Image, Size)>),
    FillPath(Box<(PathWrapper, Fill)>),
    DrawShade(Box<ShadingFunction>),
    // TODO: Remove vec?
    Clipped(Box<(Vec<(PathWrapper, FillRule)>, ByteCode)>),
    Masked(Box<(Mask, ByteCode)>),
    Opacified(Box<(NormalizedF32, ByteCode)>),
}

// TODO: Make cheap to clone?
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ByteCode {
    instructions: Vec<Instruction>,
    bbox: Rect,
}

impl ByteCode {
    pub fn new() -> Self {
        Self {
            instructions: Vec::with_capacity(10),
            bbox: Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
        }
    }

    pub fn clear(&mut self) {
        *self = ByteCode::new()
    }

    pub fn push_transformed(&mut self, transform: TransformWrapper, bytecode: ByteCode) {
        self.bbox
            .expand(&bytecode.bbox.transform(transform.0).unwrap());
        self.push(Instruction::Transformed(Box::new((transform, bytecode))));
    }

    pub fn push_isolated(&mut self, bytecode: ByteCode) {
        self.bbox.expand(&bytecode.bbox);
        self.push(Instruction::Isolated(Arc::new(bytecode)));
    }

    pub fn push_blended(&mut self, blend_mode: BlendMode, bytecode: ByteCode) {
        self.bbox.expand(&bytecode.bbox);
        self.push(Instruction::Blended(Box::new((blend_mode, bytecode))));
    }

    pub fn push_stroke_path(&mut self, path: PathWrapper, stroke: Stroke) {
        self.bbox
            .expand(&calculate_stroke_bbox(&stroke, &path.0).unwrap());
        self.push(Instruction::StrokePath(Box::new((path, stroke))));
    }

    pub fn push_fill_path(&mut self, path: PathWrapper, fill: Fill) {
        self.bbox.expand(&path.0.bounds());
        self.push(Instruction::FillPath(Box::new((path, fill))));
    }

    pub fn push_draw_glyph(&mut self, glyph_id: GlyphId, font: Font, size: FiniteF32) {
        let bbox = font
            .bbox()
            .transform(Transform::from_scale(
                size.get() / font.units_per_em() as f32,
                size.get() / font.units_per_em() as f32,
            ))
            .unwrap();
        self.bbox.expand(&bbox);
        self.push(Instruction::DrawGlyph(Box::new((
            glyph_id,
            font.clone(),
            size,
        ))));
    }

    pub fn push_shade(&mut self, shade: ShadingFunction) {
        self.push(Instruction::DrawShade(Box::new(shade)));
    }

    pub fn push_image(&mut self, image: Image, size: Size) {
        self.bbox.expand(&size.to_rect(0.0, 0.0).unwrap());
        self.push(Instruction::DrawImage(Box::new((image, size))));
    }

    pub fn push_clipped(&mut self, clips: Vec<(PathWrapper, FillRule)>, byte_code: ByteCode) {
        self.bbox.expand(&byte_code.bbox);
        self.push(Instruction::Clipped(Box::new((clips, byte_code))));
    }

    pub fn push_masked(&mut self, mask: Mask, byte_code: ByteCode) {
        self.bbox.expand(&byte_code.bbox);

        if let Some(bbox) = mask.custom_bbox() {
            self.bbox.expand(&bbox);
        }

        self.push(Instruction::Masked(Box::new((mask, byte_code))));
    }

    pub fn push_opacified(&mut self, opacity: NormalizedF32, byte_code: ByteCode) {
        self.bbox.expand(&byte_code.bbox);
        self.push(Instruction::Opacified(Box::new((opacity, byte_code))));
    }

    fn push(&mut self, op: Instruction) {
        self.instructions.push(op);
    }

    pub fn extend(&mut self, other: &ByteCode) {
        self.instructions
            .extend(other.instructions().iter().cloned());
        self.bbox.expand(&other.bbox);
    }

    pub fn instructions(&self) -> &[Instruction] {
        self.instructions.as_slice()
    }

    pub fn bbox(&self) -> Rect {
        self.bbox
    }
}

pub fn into_composited(byte_code: &ByteCode, black: bool) -> ByteCode {
    let mut new_byte_code = ByteCode::new();

    let paint = if black {
        Paint::Color(Color::black())
    } else {
        Paint::Color(Color::white())
    };

    for instruction in &byte_code.instructions {
        match instruction {
            Instruction::Transformed(t) => {
                new_byte_code.push_transformed(t.0, t.1.clone());
            }
            Instruction::Isolated(i) => new_byte_code.extend(&into_composited(&i, black)),
            Instruction::Blended(b) => new_byte_code.extend(&into_composited(&b.1, black)),
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

                new_byte_code.push_stroke_path(s.0.clone(), stroke);
            }
            Instruction::FillPath(f) => {
                let fill = Fill {
                    paint: paint.clone(),
                    opacity: f.1.opacity,
                    rule: f.1.rule,
                };

                new_byte_code.push_fill_path(f.0.clone(), fill);
            }
            Instruction::Clipped(c) => {
                new_byte_code.push_clipped(c.0.clone(), into_composited(&c.1, black));
            }
            // TODO: Add
            Instruction::DrawImage(_) => {}
            Instruction::DrawShade(_) => {}
            Instruction::Masked(_) => {}
            Instruction::Opacified(_) => {}
            Instruction::DrawGlyph(_) => {}
        }
    }

    new_byte_code
}

pub fn calculate_stroke_bbox(stroke: &Stroke, path: &tiny_skia_path::Path) -> Option<Rect> {
    let stroke = stroke.to_tiny_skia();

    if let Some(stroked_path) = path.stroke(&stroke, 1.0) {
        return stroked_path.compute_tight_bounds();
    }

    None
}
