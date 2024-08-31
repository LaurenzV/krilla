use crate::chunk_container::ChunkContainer;
use crate::error::{KrillaError, KrillaResult};
use crate::serialize::{Object, SerializerContext};
use pdf_writer::{Chunk, Ref};
use std::hash::{Hash, Hasher};
use tiny_skia_path::{Point, Transform};

#[derive(Hash)]
pub enum Destination {
    Xyz(XyzDestination),
}

impl Object for Destination {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.destinations
    }

    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        match self {
            Destination::Xyz(xyz) => xyz.serialize_into(sc, root_ref),
        }
    }
}

#[derive(Clone)]
pub struct XyzDestination {
    page_index: usize,
    point: Point,
}

impl Hash for XyzDestination {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.page_index.hash(state);
        self.point.x.to_bits().hash(state);
        self.point.y.to_bits().hash(state);
    }
}

impl Into<Destination> for XyzDestination {
    fn into(self) -> Destination {
        Destination::Xyz(self)
    }
}

impl XyzDestination {
    pub fn new(page_index: usize, point: Point) -> Self {
        Self { page_index, point }
    }
}

impl Object for XyzDestination {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.destinations
    }

    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        let page_info = sc
            .page_infos()
            .get(self.page_index)
            .ok_or(KrillaError::UserError(
                "attempted to link to non-existing page".to_string(),
            ))?;
        let page_ref = page_info.ref_;
        let page_size = page_info.media_box.height();

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

        Ok(chunk)
    }
}
