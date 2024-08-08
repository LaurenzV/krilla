use crate::object::shading_function::{GradientProperties, ShadingFunction};
use crate::object::xobject::XObject;
use crate::serialize::{Object, RegisterableObject, SerializerContext};
use crate::stream::Stream;
use crate::surface::StreamBuilder;
use crate::transform::TransformWrapper;
use pdf_writer::{Chunk, Finish, Name, Ref};
use tiny_skia_path::Rect;

// TODO: Remove clone and see what it breaks, fix them
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Mask {
    stream: Stream,
    mask_type: MaskType,
    custom_bbox: Option<Rect>,
}

impl Mask {
    pub fn new(stream: Stream, mask_type: MaskType) -> Self {
        Self {
            stream,
            mask_type,
            custom_bbox: None,
        }
    }

    pub fn new_from_shading(
        gradient_properties: GradientProperties,
        shading_transform: TransformWrapper,
        bbox: Rect,
        serializer_context: &mut SerializerContext,
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
            let mut surface = builder.surface();
            surface.push_transform(&shading_transform.0);
            surface.draw_shading(&shading_function);
            surface.pop_transform();
            surface.finish();
            builder.finish()
        };

        Some(Self {
            stream: shading_stream,
            mask_type: MaskType::Luminosity,
            custom_bbox: Some(bbox),
        })
    }

    pub fn custom_bbox(&self) -> Option<Rect> {
        self.custom_bbox
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
    fn serialize_into(self, sc: &mut SerializerContext) -> (Ref, Chunk) {
        let root_ref = sc.new_ref();
        let mut chunk = Chunk::new();

        let x_ref = sc.add(XObject::new(self.stream, false, true, self.custom_bbox));

        let mut dict = chunk.indirect(root_ref).dict();
        dict.pair(Name(b"Type"), Name(b"Mask"));
        dict.pair(Name(b"S"), self.mask_type.to_name());
        dict.pair(Name(b"G"), x_ref);

        dict.finish();

        (root_ref, chunk)
    }
}

impl RegisterableObject for Mask {}
