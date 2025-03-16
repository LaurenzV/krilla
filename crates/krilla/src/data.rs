use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use pdf_writer::{Chunk, Ref};
use crate::chunk_container::ChunkContainerFn;
use crate::serialize::SerializeContext;
use crate::util::SipHashable;

/// A type that holds some bytes.
#[derive(Clone)]
pub struct Data(pub(crate) Arc<dyn AsRef<[u8]> + Send + Sync>);

impl AsRef<[u8]> for Data {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref().as_ref()
    }
}

impl Hash for Data {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl From<Arc<dyn AsRef<[u8]> + Send + Sync>> for Data {
    fn from(value: Arc<dyn AsRef<[u8]> + Send + Sync>) -> Self {
        Self(value)
    }
}

impl From<Vec<u8>> for Data {
    fn from(value: Vec<u8>) -> Self {
        Self(Arc::new(value))
    }
}

impl From<Arc<Vec<u8>>> for Data {
    fn from(value: Arc<Vec<u8>>) -> Self {
        Self(value)
    }
}

impl Debug for Data {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Data {{..}}")
    }
}

pub(crate) trait Cacheable: SipHashable {
    fn chunk_container(&self) -> ChunkContainerFn;
    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk;
}