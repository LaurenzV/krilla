use crate::bytecode::ByteCode;
use crate::canvas::CanvasPdfSerializer;
use crate::resource::ResourceDictionary;
use crate::serialize::{Object, RegisterableObject, SerializerContext};
use crate::util::RectExt;
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::sync::Arc;
use tiny_skia_path::Rect;

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    byte_code: Arc<ByteCode>,
    isolated: bool,
    transparency_group_color_space: bool,
    custom_bbox: Option<Rect>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct XObject(Arc<Repr>);

impl XObject {
    pub fn new(
        byte_code: Arc<ByteCode>,
        isolated: bool,
        transparency_group_color_space: bool,
        custom_bbox: Option<Rect>,
    ) -> Self {
        XObject(Arc::new(Repr {
            byte_code,
            isolated,
            transparency_group_color_space,
            custom_bbox,
        }))
    }
}

impl Object for XObject {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let srgb_ref = sc.srgb();

        let mut chunk = Chunk::new();

        // TODO: Deduplicate
        let mut resource_dictionary = ResourceDictionary::new();
        let (content_stream, bbox) = {
            let mut serializer = CanvasPdfSerializer::new(&mut resource_dictionary, sc);
            serializer.serialize_bytecode(&self.0.byte_code);
            serializer.finish()
        };

        let mut x_object = chunk.form_xobject(root_ref, &content_stream);
        resource_dictionary.to_pdf_resources(sc, &mut x_object.resources());
        x_object.bbox(self.0.custom_bbox.unwrap_or(bbox).to_pdf_rect());

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

        sc.chunk_mut().extend(&chunk);
    }
}

impl RegisterableObject for XObject {}
