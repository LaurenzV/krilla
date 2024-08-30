use crate::chunk_container::ChunkContainer;
use crate::serialize::{FilterStream, Object, SerializerContext};
use crate::stream::Stream;
use crate::util::{RectExt, RectWrapper};
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::ops::DerefMut;
use tiny_skia_path::Rect;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct XObject {
    stream: Stream,
    isolated: bool,
    transparency_group_color_space: bool,
    custom_bbox: Option<RectWrapper>,
}

impl XObject {
    pub fn new(
        stream: Stream,
        isolated: bool,
        transparency_group_color_space: bool,
        custom_bbox: Option<Rect>,
    ) -> Self {
        XObject {
            stream,
            isolated,
            transparency_group_color_space,
            custom_bbox: custom_bbox.map(|c| RectWrapper(c)),
        }
    }

    pub fn bbox(&self) -> Rect {
        self.custom_bbox.map(|c| c.0).unwrap_or(self.stream.bbox())
    }
}

impl Object for XObject {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.x_objects
    }

    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let cs = sc.rgb();
        let mut chunk = Chunk::new();

        let x_object_stream =
            FilterStream::new_from_content_stream(self.stream.content(), &sc.serialize_settings);
        let mut x_object = chunk.form_xobject(root_ref, x_object_stream.encoded_data());
        x_object_stream.write_filters(x_object.deref_mut().deref_mut());

        self.stream
            .resource_dictionary()
            .to_pdf_resources(sc, &mut x_object);
        x_object.bbox(
            self.custom_bbox
                .map(|c| c.0)
                .unwrap_or(self.stream.bbox())
                .to_pdf_rect(),
        );

        if self.isolated || self.transparency_group_color_space {
            let mut group = x_object.group();
            let transparency = group.transparency();

            if self.isolated {
                transparency.isolated(self.isolated);
            }

            if self.transparency_group_color_space {
                transparency.pair(Name(b"CS"), cs);
            }

            transparency.finish();
            group.finish();
        }

        x_object.finish();

        chunk
    }
}
