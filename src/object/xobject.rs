use crate::serialize::{Object, RegisterableObject, SerializerContext};
use crate::stream::Stream;
use crate::util::RectExt;
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::sync::Arc;
use tiny_skia_path::Rect;

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    stream: Arc<Stream>,
    isolated: bool,
    transparency_group_color_space: bool,
    custom_bbox: Option<Rect>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct XObject(Arc<Repr>);

impl XObject {
    pub fn new(
        stream: Arc<Stream>,
        isolated: bool,
        transparency_group_color_space: bool,
        custom_bbox: Option<Rect>,
    ) -> Self {
        XObject(Arc::new(Repr {
            stream,
            isolated,
            transparency_group_color_space,
            custom_bbox,
        }))
    }

    pub fn bbox(&self) -> Rect {
        self.0.custom_bbox.unwrap_or(self.0.stream.bbox())
    }
}

impl Object for XObject {
    fn serialize_into(self, sc: &mut SerializerContext) -> (Ref, Chunk) {
        let srgb_ref = sc.srgb();

        let root_ref = sc.new_ref();
        let mut chunk = Chunk::new();

        let mut x_object = chunk.form_xobject(root_ref, &self.0.stream.content());
        self.0
            .stream
            .resource_dictionary()
            .to_pdf_resources(sc, &mut x_object.resources());
        x_object.bbox(
            self.0
                .custom_bbox
                .unwrap_or(self.0.stream.bbox())
                .to_pdf_rect(),
        );

        if self.0.isolated || self.0.transparency_group_color_space {
            let mut group = x_object.group();
            let transparency = group.transparency();

            if self.0.isolated {
                transparency.isolated(self.0.isolated);
            }

            if self.0.transparency_group_color_space {
                transparency.pair(Name(b"CS"), srgb_ref);
            }

            transparency.finish();
            group.finish();
        }

        x_object.finish();

        (root_ref, chunk)
    }
}

impl RegisterableObject for XObject {}
