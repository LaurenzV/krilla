use crate::bytecode::ByteCode;
use crate::canvas::{Canvas, CanvasPdfSerializer};
use crate::object::xobject::XObject;
use crate::resource::{Resource, ResourceDictionary, XObjectResource};
use crate::serialize::{Object, SerializerContext};
use crate::transform::TransformWrapper;
use crate::util::{NameExt, TransformExt};
use pdf_writer::types::{PaintType, TilingType};
use pdf_writer::{Chunk, Content, Finish, Ref};
use std::sync::Arc;
use usvg::NormalizedF32;

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    canvas: Arc<Canvas>,
    transform: TransformWrapper,
    base_opacity: NormalizedF32,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct TilingPattern(Arc<Repr>);

impl TilingPattern {
    pub fn new(
        canvas: Arc<Canvas>,
        transform: TransformWrapper,
        base_opacity: NormalizedF32,
    ) -> Self {
        Self(Arc::new(Repr {
            canvas,
            transform,
            base_opacity,
        }))
    }
}

impl Object for TilingPattern {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let mut chunk = Chunk::new();
        // TODO: Deduplicate.

        let mut resource_dictionary = ResourceDictionary::new();

        // stroke/fill opacity doesn't work consistently across different viewers for patterns,
        // so instead we simulate it ourselves.
        let content_stream = if self.0.base_opacity == NormalizedF32::ONE {
            let (content_stream, _) = {
                let mut serializer = CanvasPdfSerializer::new(&mut resource_dictionary);
                serializer.serialize_bytecode(&self.0.canvas.byte_code);
                serializer.finish()
            };
            content_stream
        } else {
            let mut byte_code = ByteCode::new();
            byte_code.push_opacified(self.0.base_opacity, self.0.canvas.byte_code.clone());

            let x_object = XObject::new(Arc::new(byte_code), false, false, None);
            let mut content = Content::new();
            content.x_object(
                resource_dictionary
                    .register_resource(Resource::XObject(XObjectResource::XObject(x_object)))
                    .to_pdf_name(),
            );
            content.finish()
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
