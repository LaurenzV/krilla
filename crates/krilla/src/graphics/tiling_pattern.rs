//! Tiling patterns.

use std::hash::{Hash, Hasher};
use std::ops::DerefMut;

use pdf_writer::types::{PaintType, TilingType};
use pdf_writer::{Chunk, Finish, Ref};

use crate::chunk_container::ChunkContainerFn;
use crate::serialize::SerializeContext;
use crate::stream::StreamBuilder;
use crate::stream::{FilterStreamBuilder, Stream};
use crate::util::HashExt;
use crate::util::TransformExt;
use crate::{resource, Cacheable, NormalizedF32, Transform};
use crate::resource::Resourceable;

#[derive(Debug, PartialEq)]
pub(crate) struct TilingPattern {
    stream: Stream,
    transform: Transform,
    base_opacity: NormalizedF32,
    width: f32,
    height: f32,
}

impl Eq for TilingPattern {}

impl Hash for TilingPattern {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.stream.hash(state);
        self.transform.hash(state);
        self.base_opacity.hash(state);
        self.width.to_bits().hash(state);
        self.height.to_bits().hash(state);
    }
}

impl TilingPattern {
    pub(crate) fn new(
        stream: Stream,
        transform: Transform,
        base_opacity: NormalizedF32,
        width: f32,
        height: f32,
        serializer_context: &mut SerializeContext,
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

impl Cacheable for TilingPattern {
    fn chunk_container(&self) -> ChunkContainerFn {
        |cc| &mut cc.patterns
    }

    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        for validation_error in self.stream.validation_errors {
            sc.register_validation_error(validation_error);
        }

        let pattern_stream = FilterStreamBuilder::new_from_content_stream(
            &self.stream.content,
            &sc.serialize_settings(),
        )
        .finish(&sc.serialize_settings());
        let mut tiling_pattern = chunk.tiling_pattern(root_ref, pattern_stream.encoded_data());
        pattern_stream.write_filters(tiling_pattern.deref_mut().deref_mut());

        self.stream
            .resource_dictionary
            .to_pdf_resources(&mut tiling_pattern, sc.serialize_settings().pdf_version());

        let final_bbox = pdf_writer::Rect::new(0.0, 0.0, self.width, self.height);

        tiling_pattern
            .tiling_type(TilingType::ConstantSpacing)
            .paint_type(PaintType::Colored)
            .bbox(final_bbox)
            .matrix(self.transform.to_pdf_transform())
            .x_step(final_bbox.x2 - final_bbox.x1)
            .y_step(final_bbox.y2 - final_bbox.y1);

        tiling_pattern.finish();

        chunk
    }
}

impl Resourceable for TilingPattern {
    type Resource = resource::Pattern;
}
