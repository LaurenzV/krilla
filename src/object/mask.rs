use crate::object::shading_function::{GradientProperties, ShadingFunction};
use crate::object::xobject::XObject;
use crate::serialize::{Object, RegisterableObject, SerializeSettings, SerializerContext};
use crate::stream::{Stream, StreamBuilder};
use crate::transform::TransformWrapper;
use pdf_writer::{Name, Ref};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tiny_skia_path::Rect;

#[derive(PartialEq, Eq, Debug, Hash)]
struct Repr {
    stream: Arc<Stream>,
    mask_type: MaskType,
    custom_bbox: Option<Rect>,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct Mask(Arc<Repr>);

impl Mask {
    pub fn new(stream: Arc<Stream>, mask_type: MaskType) -> Self {
        Self(Arc::new(Repr {
            stream,
            mask_type,
            custom_bbox: None,
        }))
    }

    pub fn new_from_shading(
        gradient_properties: GradientProperties,
        shading_transform: TransformWrapper,
        bbox: Rect,
        serializer_context: Rc<RefCell<SerializerContext>>,
    ) -> Option<Self> {
        match &gradient_properties {
            GradientProperties::RadialAxialGradient(rag) => {
                if rag.stops.iter().all(|s| s.opacity.get() == 1.0) {
                    return None;
                }
            }
            GradientProperties::PostScriptGradient(psg) => {
                if psg.stops.iter().all(|s| s.opacity.get() == 1.0) {
                    return None;
                }
            }
        }

        let shading_function = ShadingFunction::new(gradient_properties, true);

        let shading_stream = {
            let mut builder = StreamBuilder::new(serializer_context);
            builder.concat_transform(&shading_transform.0);
            builder.draw_shading(&shading_function);
            builder.finish()
        };

        Some(Self(Arc::new(Repr {
            stream: Arc::new(shading_stream),
            mask_type: MaskType::Luminosity,
            custom_bbox: Some(bbox),
        })))
    }

    pub fn custom_bbox(&self) -> Option<Rect> {
        self.0.custom_bbox
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum MaskType {
    Luminosity,
    Alpha,
}

impl MaskType {
    pub fn to_name(self) -> Name<'static> {
        match self {
            MaskType::Alpha => Name(b"Alpha"),
            MaskType::Luminosity => Name(b"Luminosity"),
        }
    }
}

impl Object for Mask {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let x_ref = sc.add(XObject::new(
            self.0.stream.clone(),
            false,
            true,
            self.0.custom_bbox,
        ));

        let mut dict = sc.chunk_mut().indirect(root_ref).dict();
        dict.pair(Name(b"Type"), Name(b"Mask"));
        dict.pair(Name(b"S"), self.0.mask_type.to_name());
        dict.pair(Name(b"G"), x_ref);
    }
}

impl RegisterableObject for Mask {}
