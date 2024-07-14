use crate::resource::PDFResource;
use crate::serialize::{ObjectSerialize, PdfObject, RefAllocator, SerializeSettings};
use pdf_writer::types::BlendMode;
use pdf_writer::{Chunk, Finish, Ref};
use std::sync::Arc;
use strict_num::NormalizedF32;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct Repr {
    non_stroking_alpha: Option<NormalizedF32>,
    stroking_alpha: Option<NormalizedF32>,
    blend_mode: Option<BlendMode>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ExtGState(Arc<Repr>);

impl ExtGState {
    pub fn new(
        non_stroking_alpha: Option<NormalizedF32>,
        stroking_alpha: Option<NormalizedF32>,
        blend_mode: Option<BlendMode>,
    ) -> Self {
        Self(Arc::new(Repr {
            non_stroking_alpha,
            stroking_alpha,
            blend_mode,
        }))
    }
}

impl PDFResource for ExtGState {
    fn get_name() -> &'static str {
        "gs"
    }
}

impl ObjectSerialize for ExtGState {
    fn serialize_into(
        self,
        chunk: &mut Chunk,
        ref_allocator: &mut RefAllocator,
        _: &SerializeSettings,
    ) -> Ref {
        let root_ref = ref_allocator.cached_ref(PdfObject::ExtGState(self.clone()));

        let mut ext_st = chunk.ext_graphics(root_ref);
        if let Some(nsa) = self.0.non_stroking_alpha {
            ext_st.non_stroking_alpha(nsa.get());
        }

        if let Some(sa) = self.0.stroking_alpha {
            ext_st.stroking_alpha(sa.get());
        }

        if let Some(bm) = self.0.blend_mode {
            ext_st.blend_mode(bm);
        }

        ext_st.finish();

        root_ref
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CompositeMode {
    /// The composite mode 'Clear'.
    Clear,
    /// The composite mode 'Source'.
    Source,
    /// The composite mode 'Destination'.
    Destination,
    /// The composite mode 'SourceOver'.
    #[default]
    SourceOver,
    /// The composite mode 'DestinationOver'.
    DestinationOver,
    /// The composite mode 'SourceIn'.
    SourceIn,
    /// The composite mode 'DestinationIn'.
    DestinationIn,
    /// The composite mode 'SourceOut'.
    SourceOut,
    /// The composite mode 'DestinationOut'.
    DestinationOut,
    /// The composite mode 'SourceAtop'.
    SourceAtop,
    /// The composite mode 'DestinationAtop'.
    DestinationAtop,
    /// The composite mode 'Xor'.
    Xor,
    /// The composite mode 'Plus'.
    Plus,
    /// The composite mode 'Screen'.
    Screen,
    /// The composite mode 'Overlay'.
    Overlay,
    /// The composite mode 'Darken'.
    Darken,
    /// The composite mode 'Lighten'.
    Lighten,
    /// The composite mode 'ColorDodge'.
    ColorDodge,
    /// The composite mode 'ColorBurn'.
    ColorBurn,
    /// The composite mode 'HardLight'.
    HardLight,
    /// The composite mode 'SoftLight'.
    SoftLight,
    /// The composite mode 'Difference'.
    Difference,
    /// The composite mode 'Exclusion'.
    Exclusion,
    /// The composite mode 'Multiply'.
    Multiply,
    /// The composite mode 'Hue'.
    Hue,
    /// The composite mode 'Saturation'.
    Saturation,
    /// The composite mode 'Color'.
    Color,
    /// The composite mode 'Luminosity'.
    Luminosity,
}

impl TryInto<BlendMode> for CompositeMode {
    type Error = ();

    fn try_into(self) -> Result<BlendMode, Self::Error> {
        use CompositeMode::*;
        match self {
            SourceOver => Ok(BlendMode::Normal),
            Multiply => Ok(BlendMode::Multiply),
            Screen => Ok(BlendMode::Screen),
            Overlay => Ok(BlendMode::Overlay),
            Darken => Ok(BlendMode::Darken),
            Lighten => Ok(BlendMode::Lighten),
            ColorDodge => Ok(BlendMode::ColorDodge),
            ColorBurn => Ok(BlendMode::ColorBurn),
            HardLight => Ok(BlendMode::HardLight),
            SoftLight => Ok(BlendMode::SoftLight),
            Difference => Ok(BlendMode::Difference),
            Exclusion => Ok(BlendMode::Exclusion),
            Hue => Ok(BlendMode::Hue),
            Saturation => Ok(BlendMode::Saturation),
            Color => Ok(BlendMode::Color),
            Luminosity => Ok(BlendMode::Luminosity),
            _ => Err(()),
        }
    }
}
