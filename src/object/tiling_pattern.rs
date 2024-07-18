use crate::canvas::{Canvas, CanvasPdfSerializer};
use crate::resource::ResourceDictionary;
use crate::serialize::{Object, SerializerContext};
use crate::transform::TransformWrapper;
use crate::util::TransformExt;
use pdf_writer::types::{PaintType, TilingType};
use pdf_writer::{Chunk, Finish, Ref};
use std::sync::Arc;
use tiny_skia_path::Transform;

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    canvas: Arc<Canvas>,
    transform: TransformWrapper,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct TilingPattern(Arc<Repr>);

impl TilingPattern {
    pub fn new(canvas: Arc<Canvas>, transform: TransformWrapper) -> Self {
        Self(Arc::new(Repr { canvas, transform }))
    }
}

impl Object for TilingPattern {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let mut chunk = Chunk::new();
        // TODO: Deduplicate.

        let mut resource_dictionary = ResourceDictionary::new();
        let (content_stream, bbox) = {
            let mut serializer = CanvasPdfSerializer::new(&mut resource_dictionary);
            serializer.serialize_instructions(self.0.canvas.byte_code.instructions());
            serializer.finish()
        };

        let mut tiling_pattern = chunk.tiling_pattern(root_ref, &content_stream);
        resource_dictionary.to_pdf_resources(sc, &mut tiling_pattern.resources());

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

    fn is_cached(&self) -> bool {
        true
    }
}
