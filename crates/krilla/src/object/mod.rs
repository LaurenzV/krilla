use crate::chunk_container::ChunkContainer;
use crate::resource::Resource;
use crate::serialize::SerializerContext;
use crate::util::SipHashable;
use pdf_writer::{Chunk, Ref};

pub mod action;
pub mod annotation;
pub mod color;
pub mod destination;
pub(crate) mod ext_g_state;
pub(crate) mod font;
#[cfg(feature = "raster-images")]
pub mod image;
pub mod mask;
pub mod outline;
pub mod page;
pub(crate) mod shading_function;
pub(crate) mod shading_pattern;
pub(crate) mod tiling_pattern;
pub(crate) mod xobject;

pub(crate) type ChunkContainerFn = Box<dyn FnMut(&mut ChunkContainer) -> &mut Vec<Chunk>>;

pub(crate) trait Cacheable: SipHashable {
    fn chunk_container(&self) -> ChunkContainerFn;
    fn serialize(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk;
}

pub(crate) trait Resourceable: Cacheable {
    type Resource: Resource;
}
