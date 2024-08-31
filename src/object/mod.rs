use pdf_writer::{Chunk, Ref};
use crate::chunk_container::ChunkContainer;
use crate::error::KrillaResult;
use crate::serialize::{SerializerContext, SipHashable};

pub mod action;
pub mod annotation;
pub(crate) mod cid_font;
pub mod color;
pub mod destination;
pub(crate) mod ext_g_state;
pub mod image;
pub mod mask;
pub mod outline;
pub mod page;
pub(crate) mod shading_function;
pub(crate) mod shading_pattern;
pub(crate) mod tiling_pattern;
pub(crate) mod type3_font;
pub(crate) mod xobject;

pub(crate) trait Object: SipHashable {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk>;
    fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk>;
}
