use crate::canvas::{Canvas, CanvasPdfSerializer};
use crate::resource::ResourceDictionary;
use crate::serialize::{ObjectSerialize, SerializerContext};
use crate::util::RectExt;
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::sync::Arc;

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    canvas: Arc<Canvas>,
    isolated: bool,
    transparency_group_color_space: bool,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct XObject(Arc<Repr>);

impl XObject {
    pub fn new(canvas: Arc<Canvas>, isolated: bool, transparency_group_color_space: bool) -> Self {
        XObject(Arc::new(Repr {
            canvas,
            isolated,
            transparency_group_color_space,
        }))
    }
}

impl ObjectSerialize for XObject {
    const CACHED: bool = false;

    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let srgb_ref = sc.srgb();

        let mut chunk = Chunk::new();

        // TODO: Deduplicate
        let mut resource_dictionary = ResourceDictionary::new();
        let (content_stream, bbox) = {
            let mut serializer = CanvasPdfSerializer::new(&mut resource_dictionary);
            serializer.serialize_instructions(self.0.canvas.byte_code.instructions());
            serializer.finish()
        };

        let mut x_object = chunk.form_xobject(root_ref, &content_stream);
        resource_dictionary.to_pdf_resources(sc, &mut x_object.resources());
        x_object.bbox(bbox.to_pdf_rect());

        if self.0.isolated || self.0.transparency_group_color_space {
            let mut transparency = x_object.group().transparency();

            if self.0.isolated {
                transparency.isolated(self.0.isolated);
            }

            if self.0.transparency_group_color_space {
                transparency.pair(Name(b"CS"), srgb_ref);
            }

            transparency.finish();
        }

        x_object.finish();

        sc.chunk_mut().extend(&chunk);
    }
}
