use crate::canvas::{Canvas, CanvasPdfSerializer};
use crate::resource::{PdfColorSpace, ResourceDictionary};
use crate::serialize::{CacheableObject, ObjectSerialize, SerializerContext};
use crate::util::RectExt;
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::sync::Arc;

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct Mask {
    mask_type: MaskType,
    canvas: Arc<Canvas>,
}

impl Mask {
    pub fn new(canvas: Canvas, mask_type: MaskType) -> Self {
        Self {
            mask_type,
            canvas: Arc::new(canvas),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum MaskType {
    Luminance,
    Alpha,
}

impl MaskType {
    pub fn to_name(self) -> Name<'static> {
        match self {
            MaskType::Alpha => Name(b"Alpha"),
            MaskType::Luminance => Name(b"Luminosity"),
        }
    }
}

impl ObjectSerialize for Mask {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let x_ref = sc.new_ref();
        let srgb_ref = sc.add_cached(CacheableObject::PdfColorSpace(PdfColorSpace::SRGB));

        let mut chunk = Chunk::new();
        let mut resource_dictionary = ResourceDictionary::new();
        let (content_stream, bbox) = {
            let mut serializer = CanvasPdfSerializer::new(&mut resource_dictionary);
            serializer.serialize_instructions(self.canvas.byte_code.instructions());
            serializer.finish()
        };

        let mut x_object = chunk.form_xobject(x_ref, &content_stream);
        resource_dictionary.to_pdf_resources(sc, &mut x_object.resources());
        x_object.bbox(bbox.to_pdf_rect());

        x_object
            .group()
            .transparency()
            .isolated(false)
            .knockout(false)
            .pair(Name(b"CS"), srgb_ref);

        x_object.finish();

        let mut dict = sc.chunk_mut().indirect(root_ref).dict();
        dict.pair(Name(b"Type"), Name(b"Mask"));
        dict.pair(Name(b"S"), self.mask_type.to_name());
        dict.pair(Name(b"G"), x_ref);

        dict.finish();

        sc.chunk_mut().extend(&chunk);
    }
}
