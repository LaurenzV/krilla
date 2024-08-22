use crate::serialize::{Object, SerializerContext};
use pdf_writer::{Chunk, Ref};
use tiny_skia_path::{Point, Transform};

pub trait Destination {
    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk;
}

pub struct XyzDestination {
    page_index: usize,
    point: Point,
}

impl XyzDestination {
    pub fn new(page_index: usize, point: Point) -> Self {
        Self { page_index, point }
    }
}

impl Destination for XyzDestination {
    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let page_ref = sc.page_infos()[self.page_index].ref_;
        let page_size = sc.page_infos()[self.page_index].media_box.height();

        let mut mapped_point = self.point;
        // Convert to PDF coordinates
        let invert_transform = Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, page_size);
        invert_transform.map_point(&mut mapped_point);

        let mut chunk = Chunk::new();
        chunk
            .indirect(root_ref)
            .start::<pdf_writer::writers::Destination>()
            .page(page_ref)
            .xyz(mapped_point.x, mapped_point.y, None);

        chunk
    }
}
