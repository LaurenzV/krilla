//! Various PDF objects.

use pdf_writer::{Chunk, Ref};

use crate::chunk_container::ChunkContainer;
use crate::resource::Resource;
use crate::serialize::SerializeContext;
use crate::util::SipHashable;

pub mod page;

pub(crate) type ChunkContainerFn = fn(&mut ChunkContainer) -> &mut Vec<Chunk>;

pub(crate) trait Cacheable: SipHashable {
    fn chunk_container(&self) -> ChunkContainerFn;
    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk;
}

pub(crate) trait Resourceable: Cacheable {
    type Resource: Resource;
}
