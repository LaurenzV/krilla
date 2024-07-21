use std::hash::{Hash, Hasher};
use tiny_skia_path::Transform;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformWrapper(pub(crate) Transform);

// We don't care about NaNs.
impl Eq for TransformWrapper {}

impl Hash for TransformWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.tx.to_bits().hash(state);
        self.0.ty.to_bits().hash(state);
        self.0.sx.to_bits().hash(state);
        self.0.sy.to_bits().hash(state);
        self.0.kx.to_bits().hash(state);
        self.0.ky.to_bits().hash(state);
    }
}
