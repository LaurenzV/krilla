use crate::chunk_container::ChunkContainer;
use crate::error::KrillaResult;
use crate::object::Object;
use crate::serialize::{FilterStream, SerializerContext};
use crate::stream::Stream;
use crate::util::TransformExt;
use crate::util::TransformWrapper;
use pdf_writer::types::{PaintType, TilingType};
use pdf_writer::{Chunk, Finish, Ref};
use std::ops::DerefMut;
use tiny_skia_path::FiniteF32;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) struct TilingPattern {
    stream: Stream,
    transform: TransformWrapper,
    width: FiniteF32,
    height: FiniteF32,
}

impl TilingPattern {
    pub fn new(
        stream: Stream,
        transform: TransformWrapper,
        width: FiniteF32,
        height: FiniteF32,
    ) -> Self {
        Self {
            stream,
            transform,
            width,
            height,
        }
    }
}

impl Object for TilingPattern {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.patterns
    }

    fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let pattern_stream =
            FilterStream::new_from_content_stream(self.stream.content(), &sc.serialize_settings);
        let mut tiling_pattern = chunk.tiling_pattern(root_ref, pattern_stream.encoded_data());
        pattern_stream.write_filters(tiling_pattern.deref_mut().deref_mut());

        self.stream
            .resource_dictionary()
            .to_pdf_resources(sc, &mut tiling_pattern)?;

        let final_bbox = pdf_writer::Rect::new(0.0, 0.0, self.width.get(), self.height.get());

        tiling_pattern
            .tiling_type(TilingType::ConstantSpacing)
            .paint_type(PaintType::Colored)
            .bbox(final_bbox)
            .matrix(self.transform.0.to_pdf_transform())
            .x_step(final_bbox.x2 - final_bbox.x1)
            .y_step(final_bbox.y2 - final_bbox.y1);

        tiling_pattern.finish();

        Ok(chunk)
    }
}

#[cfg(test)]
mod tests {
    use crate::color::rgb::Rgb;
    use crate::paint::{Paint, Pattern};
    use crate::path::Fill;
    use crate::serialize::SerializerContext;
    use crate::stream::StreamBuilder;
    use crate::surface::Surface;
    use crate::tests::{basic_pattern_stream, rect_to_path};
    use crate::tiling_pattern::TilingPattern;
    use crate::util::TransformWrapper;
    use krilla_macros::{snapshot, visreg};
    use tiny_skia_path::{FiniteF32, NormalizedF32, Transform};

    #[snapshot]
    fn tiling_pattern_basic(sc: &mut SerializerContext) {
        let stream_builder = StreamBuilder::new(sc);
        let pattern_stream = basic_pattern_stream(stream_builder);

        let tiling_pattern = TilingPattern::new(
            pattern_stream,
            TransformWrapper(Transform::identity()),
            FiniteF32::new(20.0).unwrap(),
            FiniteF32::new(20.0).unwrap(),
        );

        sc.add_object(tiling_pattern).unwrap();
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
                paint: Paint::<Rgb>::Pattern(pattern),
                opacity: NormalizedF32::new(0.5).unwrap(),
                rule: Default::default(),
            },
        )
    }
}
