//! Various PDF objects.

use pdf_writer::{Chunk, Ref};

use crate::chunk_container::ChunkContainer;
use crate::resource::Resource;
use crate::serialize::SerializeContext;
use crate::util::SipHashable;

pub mod color;
pub(crate) mod ext_g_state;
#[cfg(feature = "raster-images")]
pub mod image;
pub mod mask;
pub mod page;
pub(crate) mod shading_function;
pub(crate) mod shading_pattern;
pub(crate) mod tiling_pattern;
pub(crate) mod xobject;

pub(crate) type ChunkContainerFn = fn(&mut ChunkContainer) -> &mut Vec<Chunk>;

pub(crate) trait Cacheable: SipHashable {
    fn chunk_container(&self) -> ChunkContainerFn;
    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk;
}

pub(crate) trait Resourceable: Cacheable {
    type Resource: Resource;
}
