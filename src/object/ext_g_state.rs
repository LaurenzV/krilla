use crate::mask::Mask;
use crate::resource::PDFResource;
use crate::serialize::{CacheableObject, ObjectSerialize, SerializerContext};
use pdf_writer::types::BlendMode;
use pdf_writer::{Finish, Name, Ref};
use tiny_skia_path::NormalizedF32;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub(crate) struct ExtGState {
    non_stroking_alpha: Option<NormalizedF32>,
    stroking_alpha: Option<NormalizedF32>,
    blend_mode: Option<BlendMode>,
    mask: Option<Mask>,
}

impl ExtGState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stroking_alpha(&mut self, stroking_alpha: NormalizedF32) -> Self {
        self.stroking_alpha = Some(stroking_alpha);
        Self
    }

    pub fn non_stroking_alpha(&mut self, non_stroking_alpha: NormalizedF32) -> Self {
        self.non_stroking_alpha = Some(non_stroking_alpha);
        Self
    }

    pub fn blend_mode(&mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = Some(blend_mode);
        Self
    }

    pub fn mask(&mut self, mask: Mask) -> Self {
        self.mask = Some(mask);
        Self
    }

    pub fn combine(&mut self, other: &ExtGState) {
        if let Some(non_stroking_alpha) = other.non_stroking_alpha {
            self.non_stroking_alpha = Some(non_stroking_alpha);
        }

        if let Some(stroking_alpha) = other.stroking_alpha {
            self.stroking_alpha = Some(stroking_alpha);
        }

        if let Some(blend_mode) = other.blend_mode {
            self.blend_mode = Some(blend_mode)
        }

        if let Some(mask) = other.mask.clone() {
            self.mask = Some(mask);
        }
    }
}

impl PDFResource for ExtGState {
    fn get_name() -> &'static str {
        "G"
    }
}

impl ObjectSerialize for ExtGState {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let mask_ref = self
            .mask
            .clone()
            .map(|ma| sc.add_cached(CacheableObject::Mask(ma)));

        let mut ext_st = sc.chunk_mut().ext_graphics(root_ref);
        if let Some(nsa) = self.non_stroking_alpha {
            ext_st.non_stroking_alpha(nsa.get());
        }

        if let Some(sa) = self.stroking_alpha {
            ext_st.stroking_alpha(sa.get());
        }

        if let Some(bm) = self.blend_mode {
            ext_st.blend_mode(bm);
        }

        if let Some(mask_ref) = mask_ref {
            ext_st.pair(Name(b"SMask"), mask_ref);
        }

        ext_st.finish();
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
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
