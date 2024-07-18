use crate::canvas::{Canvas, CanvasPdfSerializer};
use crate::resource::ResourceDictionary;
use crate::serialize::{CacheableObject, ObjectSerialize, SerializerContext};
use crate::util::RectExt;
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::sync::Arc;

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    canvas: Canvas,
    isolated: bool,
    needs_transparency: bool,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct XObject(Arc<Repr>);

// We don't cache XObjects for now.

impl XObject {
    pub fn new(canvas: Canvas, isolated: bool) -> Self {
        let has_mask = canvas.has_mask();
        XObject(Arc::new(Repr {
            canvas,
            isolated,
            // TODO: Figure out how to initialize correctly
            needs_transparency: has_mask,
        }))
    }
}

impl ObjectSerialize for crate::resource::XObject {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let srgb_ref = sc.srgb_ref();

        let mut chunk = Chunk::new();

        // TODO: Deduplicate
        let mut resource_dictionary = ResourceDictionary::new();
        let (content_stream, bbox) = {
            let mut serializer = CanvasPdfSerializer::new(&mut resource_dictionary);
            serializer.serialize_instructions(self.canvas.byte_code.instructions());
            serializer.finish()
        };

        let mut x_object = chunk.form_xobject(root_ref, &content_stream);
        resource_dictionary.to_pdf_resources(sc, &mut x_object.resources());
        x_object.bbox(bbox.to_pdf_rect());

        if self.isolated || self.needs_transparency {
            let mut transparency = x_object.group().transparency();

            if self.isolated {
                transparency.isolated(self.isolated);
            }

            if self.needs_transparency {
                transparency.pair(Name(b"CS"), srgb_ref);
            }

            transparency.finish();
        }

        x_object.finish();

        sc.chunk_mut().extend(&chunk);
    }
}
