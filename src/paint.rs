use crate::canvas::{Canvas, CanvasPdfSerializer};
use crate::color::Color;
use crate::resource::ResourceDictionary;
use crate::serialize::{ObjectSerialize, SerializerContext};
use crate::transform::TransformWrapper;
use crate::util::TransformExt;
use pdf_writer::types::{FunctionShadingType, PaintType, TilingType};
use pdf_writer::{Chunk, Finish, Ref};
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, NormalizedF32, Rect, Transform};

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum SpreadMethod {
    Pad,
    Reflect,
    Repeat,
}

impl Default for SpreadMethod {
    fn default() -> Self {
        Self::Pad
    }
}

pub type StopOffset = NormalizedF32;

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub struct Stop {
    pub offset: StopOffset,
    pub color: Color,
    pub opacity: NormalizedF32,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct LinearGradient {
    pub x1: FiniteF32,
    pub y1: FiniteF32,
    pub x2: FiniteF32,
    pub y2: FiniteF32,
    pub transform: TransformWrapper,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct RadialGradient {
    pub cx: FiniteF32,
    pub cy: FiniteF32,
    pub r: FiniteF32,
    pub fx: FiniteF32,
    pub fy: FiniteF32,
    pub transform: TransformWrapper,
    pub spread_method: SpreadMethod,
    // TODO: Add note that all stops must be in the same color space
    pub stops: Vec<Stop>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct TilingPattern(pub Arc<Pattern>);

impl ObjectSerialize for TilingPattern {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let mut chunk = Chunk::new();
        // TODO: Deduplicate
        let mut resource_dictionary = ResourceDictionary::new();
        let (content_stream, bbox) = {
            let mut serializer = CanvasPdfSerializer::new(&mut resource_dictionary);
            serializer.serialize_instructions(self.0.canvas.byte_code.instructions());
            serializer.finish()
        };

        let mut tiling_pattern = chunk.tiling_pattern(root_ref, &content_stream);
        resource_dictionary.to_pdf_resources(sc, &mut tiling_pattern.resources());

        // We already account for the x/y of the pattern by appending it to the matrix above, so here we just need to take the height / width
        // in consideration
        let final_bbox = pdf_writer::Rect::new(
            0.0,
            0.0,
            self.0.canvas.size.width(),
            self.0.canvas.size.height(),
        );

        tiling_pattern
            .tiling_type(TilingType::ConstantSpacing)
            .paint_type(PaintType::Colored)
            .bbox(final_bbox)
            .matrix(self.0.transform.0.to_pdf_transform())
            .x_step(final_bbox.x2 - final_bbox.x1)
            .y_step(final_bbox.y2 - final_bbox.y1);

        tiling_pattern.finish();
        sc.chunk_mut().extend(&chunk);
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Pattern {
    pub(crate) canvas: Arc<Canvas>,
    pub(crate) transform: TransformWrapper,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum Paint {
    Color(Color),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
    Pattern(Arc<Pattern>),
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct Shading(GradientProperties);

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct GradientProperties {
    pub coords: Vec<FiniteF32>,
    pub shading_type: FunctionShadingType,
    pub stops: Vec<Stop>,
    // The bbox of the object the gradient is applied to
    pub bbox: Rect,
    pub spread_method: SpreadMethod,
}

pub trait GradientPropertiesExt {
    // TODO: BBox only needed if extend is not pad
    fn gradient_properties(&self, bbox: Rect) -> (GradientProperties, TransformWrapper);
}

impl GradientPropertiesExt for LinearGradient {
    fn gradient_properties(&self, bbox: Rect) -> (GradientProperties, TransformWrapper) {
        (
            GradientProperties {
                coords: vec![self.x1, self.y1, self.x2, self.y2],
                shading_type: FunctionShadingType::Axial,
                stops: Vec::from(self.stops.clone()),
                bbox,
                spread_method: self.spread_method,
            },
            self.transform,
        )
    }
}

impl GradientPropertiesExt for RadialGradient {
    fn gradient_properties(&self, bbox: Rect) -> (GradientProperties, TransformWrapper) {
        (
            GradientProperties {
                coords: vec![
                    self.fx,
                    self.fy,
                    FiniteF32::new(0.0).unwrap(),
                    self.cx,
                    self.cy,
                    FiniteF32::new(self.r.get()).unwrap(),
                ],
                shading_type: FunctionShadingType::Radial,
                stops: Vec::from(self.stops.clone()),
                bbox,
                spread_method: self.spread_method,
            },
            self.transform,
        )
    }
}
