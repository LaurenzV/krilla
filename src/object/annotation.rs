use crate::object::destination::Destination;
use crate::serialize::{Object, RegisterableObject};
use tiny_skia_path::Rect;

pub trait Annotation: Object + RegisterableObject {}

pub struct LinkAnnotation {
    pub rect: Rect,
    pub dest: Box<dyn Destination>,
}
