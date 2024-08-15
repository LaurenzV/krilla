use crate::object::shading_function::{GradientProperties, ShadingFunction};
use crate::object::xobject::XObject;
use crate::serialize::{Object, RegisterableObject, SerializerContext};
use crate::stream::Stream;
use crate::surface::StreamBuilder;
use crate::transform::TransformWrapper;
use pdf_writer::{Chunk, Finish, Name, Ref};
use tiny_skia_path::Rect;

/// A mask.
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Mask {
    /// The stream of the mask.
    stream: Stream,
    /// The type of the mask.
    mask_type: MaskType,
    /// A custom bbox of the mask. The only reason we need this is that for gradients with
    /// transparencies, we create a custom mask where we call the shading operator. In this case,
    /// we want to manually set the bbox of the underlying XObject to match the shape that the
    /// gradient is being applied to.
    custom_bbox: Option<Rect>,
}

impl Mask {
    /// Create a new mask.
    pub fn new(stream: Stream, mask_type: MaskType) -> Self {
        Self {
            stream,
            mask_type,
            custom_bbox: None,
        }
    }

    /// Create a new mask for a shading to encode the opacity channels.
    pub(crate) fn new_from_shading(
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
            surface.pop();
            surface.finish();
            builder.finish()
        };

        Some(Self {
            stream: shading_stream,
            mask_type: MaskType::Luminosity,
            custom_bbox: Some(bbox),
        })
    }
}

/// A mask type.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum MaskType {
    /// A luminosity mask.
    Luminosity,
    /// An alpha mask.
    Alpha,
}

impl MaskType {
    /// Return the PDF name of the mask type.
    pub fn to_name(self) -> Name<'static> {
        match self {
            MaskType::Alpha => Name(b"Alpha"),
            MaskType::Luminosity => Name(b"Luminosity"),
        }
    }
}

impl Object for Mask {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        let x_ref = sc.add(XObject::new(self.stream, false, true, self.custom_bbox));

        let mut dict = chunk.indirect(root_ref).dict();
        dict.pair(Name(b"Type"), Name(b"Mask"));
        dict.pair(Name(b"S"), self.mask_type.to_name());
        dict.pair(Name(b"G"), x_ref);

        dict.finish();

        chunk
    }
}

impl RegisterableObject for Mask {}


#[cfg(test)]
mod tests {
    use tiny_skia_path::{PathBuilder, Rect};
    use usvg::NormalizedF32;
    use crate::{Fill, MaskType, Paint, rgb};
    use crate::object::ext_g_state::ExtGState;
    use crate::object::mask::Mask;
    use crate::rgb::Rgb;
    use crate::serialize::{SerializeSettings, SerializerContext};
    use crate::stream::ContentBuilder;
    use crate::surface::{StreamBuilder, Surface};
    use crate::test_utils::check_snapshot;

    fn sc() -> SerializerContext {
        let settings = SerializeSettings::default_test();
        SerializerContext::new(settings)
    }

    fn mask_impl(mask_type: MaskType, name: &str) {
        let mut sc = sc();

        let mut stream_builder = StreamBuilder::new(&mut sc);
        let mut surface = stream_builder.surface();

        let mut builder = PathBuilder::new();
        builder.push_rect(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap());
        let path = builder.finish().unwrap();

        surface.fill_path(&path, Fill {
            paint: Paint::<Rgb>::Color(rgb::Color::new(255, 0, 0)),
            opacity: NormalizedF32::new(0.5).unwrap(),
            rule: Default::default(),
        });
        surface.finish();
        let mask = Mask::new(stream_builder.finish(), mask_type);
        sc.add(mask);

        check_snapshot(&format!("mask/{}", name), sc.finish().as_bytes());
    }

    #[test]
    pub fn luminosity() {
        mask_impl(MaskType::Luminosity, "luminosity");
    }

    #[test]
    pub fn alpha() {
        mask_impl(MaskType::Alpha, "alpha");
    }

}