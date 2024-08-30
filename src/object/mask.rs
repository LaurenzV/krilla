use crate::chunk_container::ChunkContainer;
use crate::error::KrillaResult;
use crate::object::shading_function::{GradientProperties, ShadingFunction};
use crate::object::xobject::XObject;
use crate::serialize::{Object, SerializerContext};
use crate::stream::Stream;
use crate::surface::StreamBuilder;
use crate::transform::TransformWrapper;
use crate::util::RectWrapper;
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
    custom_bbox: Option<RectWrapper>,
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
            custom_bbox: Some(RectWrapper(bbox)),
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
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.masks
    }

    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let x_ref = sc.add_object(XObject::new(
            self.stream.clone(),
            false,
            true,
            self.custom_bbox.map(|c| c.0),
        ))?;

        let mut dict = chunk.indirect(root_ref).dict();
        dict.pair(Name(b"Type"), Name(b"Mask"));
        dict.pair(Name(b"S"), self.mask_type.to_name());
        dict.pair(Name(b"G"), x_ref);

        dict.finish();

        Ok(chunk)
    }
}

#[cfg(test)]
mod tests {
    use crate::object::mask::Mask;
    use crate::rgb::Rgb;
    use crate::serialize::{SerializeSettings, SerializerContext};
    use crate::surface::StreamBuilder;
    use crate::tests::check_snapshot;
    use crate::{rgb, Fill, MaskType, Paint};
    use krilla_macros::snapshot;
    use tiny_skia_path::{PathBuilder, Rect};
    use usvg::NormalizedF32;

    fn mask_impl(mask_type: MaskType, sc: &mut SerializerContext) {
        let mut stream_builder = StreamBuilder::new(sc);
        let mut surface = stream_builder.surface();

        let mut builder = PathBuilder::new();
        builder.push_rect(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap());
        let path = builder.finish().unwrap();

        surface.fill_path(
            &path,
            Fill {
                paint: Paint::<Rgb>::Color(rgb::Color::new(255, 0, 0)),
                opacity: NormalizedF32::new(0.5).unwrap(),
                rule: Default::default(),
            },
        );
        surface.finish();
        let mask = Mask::new(stream_builder.finish(), mask_type);
        sc.add_object(mask).unwrap();
    }

    #[snapshot]
    pub fn mask_luminosity(sc: &mut SerializerContext) {
        mask_impl(MaskType::Luminosity, sc);
    }

    #[snapshot]
    pub fn mask_alpha(sc: &mut SerializerContext) {
        mask_impl(MaskType::Alpha, sc);
    }
}
