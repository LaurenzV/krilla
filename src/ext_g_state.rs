use crate::resource::PDFResource;
use crate::serialize::{ObjectSerialize, PdfObject, RefAllocator, SerializeSettings};
use pdf_writer::{Chunk, Finish, Ref};
use std::sync::Arc;
use strict_num::NormalizedF32;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct Repr {
    non_stroking_alpha: Option<NormalizedF32>,
    stroking_alpha: Option<NormalizedF32>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ExtGState(Arc<Repr>);

impl ExtGState {
    pub fn new(
        non_stroking_alpha: Option<NormalizedF32>,
        stroking_alpha: Option<NormalizedF32>,
    ) -> Self {
        Self(Arc::new(Repr {
            non_stroking_alpha,
            stroking_alpha,
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

        ext_st.finish();

        root_ref
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
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

impl CompositeMode {
    pub fn is_pdf_blend_mode(&self) -> bool {
        use CompositeMode::*;
        matches!(
            self,
            SourceOver
                | Multiply
                | Screen
                | Overlay
                | Darken
                | Lighten
                | ColorDodge
                | ColorBurn
                | HardLight
                | SoftLight
                | Difference
                | Exclusion
                | Hue
                | Saturation
                | Color
                | Luminosity
        )
    }
}
