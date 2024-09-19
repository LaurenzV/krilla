use crate::chunk_container::ChunkContainer;
use crate::object::Object;
use crate::resource::RegisterableResource;
use crate::serialize::{FilterStream, SerializerContext};
use crate::stream::Stream;
use crate::stream::StreamBuilder;
use crate::util::HashExt;
use crate::util::TransformExt;
use pdf_writer::types::{PaintType, TilingType};
use pdf_writer::{Chunk, Finish, Ref};
use std::hash::{Hash, Hasher};
use std::ops::DerefMut;
use tiny_skia_path::{NormalizedF32, Transform};

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
    pub fn new(
        stream: Stream,
        transform: Transform,
        base_opacity: NormalizedF32,
        width: f32,
        height: f32,
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

impl RegisterableResource<crate::resource::Pattern> for TilingPattern {}

impl Object for TilingPattern {
    fn chunk_container(&self) -> Box<dyn FnMut(&mut ChunkContainer) -> &mut Vec<Chunk>> {
        Box::new(|cc| &mut cc.patterns)
    }

    fn serialize(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        let pattern_stream =
            FilterStream::new_from_content_stream(&self.stream.content, &sc.serialize_settings);
        let mut tiling_pattern = chunk.tiling_pattern(root_ref, pattern_stream.encoded_data());
        pattern_stream.write_filters(tiling_pattern.deref_mut().deref_mut());

        self.stream
            .resource_dictionary
            .to_pdf_resources(&mut tiling_pattern);

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

#[cfg(test)]
mod tests {

    use crate::paint::Pattern;
    use crate::path::Fill;
    use crate::serialize::SerializerContext;
    use crate::stream::StreamBuilder;
    use crate::surface::Surface;
    use crate::tests::{basic_pattern_stream, rect_to_path};
    use crate::tiling_pattern::TilingPattern;

    use krilla_macros::{snapshot, visreg};
    use tiny_skia_path::{NormalizedF32, Transform};

    #[snapshot]
    fn tiling_pattern_basic(sc: &mut SerializerContext) {
        let stream_builder = StreamBuilder::new(sc);
        let pattern_stream = basic_pattern_stream(stream_builder);

        let tiling_pattern = TilingPattern::new(
            pattern_stream,
            Transform::identity(),
            NormalizedF32::ONE,
            20.0,
            20.0,
            sc,
        );

        sc.add_object(tiling_pattern);
    }

    #[visreg(all)]
    fn tiling_pattern_basic(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let stream_builder = surface.stream_builder();
        let pattern_stream = basic_pattern_stream(stream_builder);

        let pattern = Pattern {
            stream: pattern_stream,
            transform: Default::default(),
            width: 20.0,
            height: 20.0,
        };

        surface.fill_path(
            &path,
            Fill {
                paint: pattern.into(),
                opacity: NormalizedF32::new(0.5).unwrap(),
                rule: Default::default(),
            },
        )
    }
}
