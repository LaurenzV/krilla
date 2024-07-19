use crate::canvas::Canvas;
use crate::object::xobject::XObject;
use crate::serialize::{Object, SerializerContext};
use pdf_writer::{Name, Ref};
use std::sync::Arc;

#[derive(PartialEq, Eq, Debug, Hash)]
struct Repr {
    canvas: Arc<Canvas>,
    mask_type: MaskType,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct Mask(Arc<Repr>);

impl Mask {
    pub fn new(canvas: Arc<Canvas>, mask_type: MaskType) -> Self {
        Self(Arc::new(Repr { canvas, mask_type }))
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

impl Object for Mask {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let x_ref = sc.add(XObject::new(self.0.canvas.clone(), false, true));

        let mut dict = sc.chunk_mut().indirect(root_ref).dict();
        dict.pair(Name(b"Type"), Name(b"Mask"));
        dict.pair(Name(b"S"), self.0.mask_type.to_name());
        dict.pair(Name(b"G"), x_ref);
    }

    fn is_cached(&self) -> bool {
        false
    }
}
