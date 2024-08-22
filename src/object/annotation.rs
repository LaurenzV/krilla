use pdf_writer::{Chunk, Finish, Name, Ref};
use crate::object::destination::Destination;
use crate::serialize::{Object, RegisterableObject, SerializerContext};
use tiny_skia_path::{Rect, Transform};
use crate::util::RectExt;

pub trait Annotation: Object + RegisterableObject {}

pub enum LinkTarget {
    Destination(Box<dyn Destination>)
}

pub struct LinkAnnotation {
    pub rect: Rect,
    pub page_index: usize,
    pub link_target: LinkTarget,
}

impl Object for LinkAnnotation {
    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        let target_ref = sc.new_ref();

        match &self.link_target {
            LinkTarget::Destination(dest) => dest.serialize_into(sc, target_ref)
        };

        let mut annotation = chunk.indirect(root_ref).start::<pdf_writer::writers::Annotation>();

        let page_size = sc.page_infos()[self.page_index].media_box.height();
        let invert_transform = Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, page_size);
        let actual_rect = self.rect.transform(invert_transform).unwrap();
        annotation.rect(actual_rect.to_pdf_rect());
        annotation.border(0.0, 0.0, 0.0, None);

        match &self.link_target {
            LinkTarget::Destination(_) => annotation.pair(Name(b"Dest"), target_ref)
        };

        annotation.finish();

        chunk
    }
}
