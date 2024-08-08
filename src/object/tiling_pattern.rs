use crate::serialize::{Object, RegisterableObject, SerializerContext};
use crate::stream::Stream;
use crate::surface::StreamBuilder;
use crate::transform::TransformWrapper;
use crate::util::TransformExt;
use pdf_writer::types::{PaintType, TilingType};
use pdf_writer::{Chunk, Finish, Ref};
use tiny_skia_path::FiniteF32;
use usvg::NormalizedF32;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct TilingPattern {
    stream: Stream,
    transform: TransformWrapper,
    base_opacity: NormalizedF32,
    width: FiniteF32,
    height: FiniteF32,
}

impl TilingPattern {
    pub fn new(
        stream: Stream,
        transform: TransformWrapper,
        base_opacity: NormalizedF32,
        width: FiniteF32,
        height: FiniteF32,
        serializer_context: &mut SerializerContext,
    ) -> Self {
        // stroke/fill opacity doesn't work consistently across different viewers for patterns,
        // so instead we simulate it ourselves.
        let pattern_stream = if base_opacity == NormalizedF32::ONE {
            stream
        } else {
            let stream = {
                let mut builder = StreamBuilder::new(serializer_context);
                let mut surface = builder.surface();
                surface.draw_opacified_stream(base_opacity, stream);
                surface.finish();
                builder.finish()
            };

            stream
        };

        Self {
            stream: pattern_stream,
            transform,
            base_opacity,
            width,
            height,
        }
    }
}

impl Object for TilingPattern {
    fn serialize_into(self, sc: &mut SerializerContext) -> (Ref, Chunk) {
        let root_ref = sc.new_ref();
        let mut chunk = Chunk::new();

        let mut tiling_pattern = chunk.tiling_pattern(root_ref, &self.stream.content);
        self.stream
            .resource_dictionary
            .to_pdf_resources(sc, &mut tiling_pattern.resources());

        let final_bbox = pdf_writer::Rect::new(0.0, 0.0, self.width.get(), self.height.get());

        tiling_pattern
            .tiling_type(TilingType::ConstantSpacing)
            .paint_type(PaintType::Colored)
            .bbox(final_bbox)
            .matrix(self.transform.0.to_pdf_transform())
            .x_step(final_bbox.x2 - final_bbox.x1)
            .y_step(final_bbox.y2 - final_bbox.y1);

        tiling_pattern.finish();

        (root_ref, chunk)
    }
}

impl RegisterableObject for TilingPattern {}
