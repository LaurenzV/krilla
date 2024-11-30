use crate::object::mask::Mask;
use crate::object::{ChunkContainerFn, Object};
use crate::serialize::SerializerContext;
use crate::validation::ValidationError;
use pdf_writer::types::BlendMode;
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::sync::Arc;
use tiny_skia_path::NormalizedF32;

/// The inner representation of an external graphics state.
#[derive(Debug, Hash, PartialEq, Eq, Default, Clone)]
struct Repr {
    /// The non-stroking alpha.
    non_stroking_alpha: Option<NormalizedF32>,
    /// The stroking alpha.
    stroking_alpha: Option<NormalizedF32>,
    /// The blend mode.
    blend_mode: Option<BlendMode>,
    /// An active mask.
    mask: Option<Ref>,
}

/// A graphics state containing information about
/// - The current stroking alpha.
/// - The current non-stroking alpha.
/// - The current blend mode.
/// - The current mask.
///
/// This struct provides exposes a builder pattern for setting the various properties
/// individually.
///
/// This type is cheap to clone.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
pub struct ExtGState(Arc<Repr>);

impl ExtGState {
    /// Create a new, empty graphics state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new graphics state with a stroking alpha.
    #[must_use]
    pub fn stroking_alpha(mut self, stroking_alpha: NormalizedF32) -> Self {
        Arc::make_mut(&mut self.0).stroking_alpha = Some(stroking_alpha);
        self
    }

    /// Create a new graphics state with a non-stroking alpha.
    #[must_use]
    pub fn non_stroking_alpha(mut self, non_stroking_alpha: NormalizedF32) -> Self {
        Arc::make_mut(&mut self.0).non_stroking_alpha = Some(non_stroking_alpha);
        self
    }

    /// Create a new graphics state with a blend mode.
    #[must_use]
    pub fn blend_mode(mut self, blend_mode: BlendMode) -> Self {
        Arc::make_mut(&mut self.0).blend_mode = Some(blend_mode);
        self
    }

    /// Create a new graphics state with a mask.
    #[must_use]
    pub fn mask(mut self, mask: Mask, sc: &mut SerializerContext) -> Self {
        let mask_ref = sc.add_object(mask);
        Arc::make_mut(&mut self.0).mask = Some(mask_ref);
        self
    }

    /// Check whether the graphics state is empty.
    pub fn empty(&self) -> bool {
        self.0.mask.is_none()
            && self.0.stroking_alpha.is_none()
            && self.0.non_stroking_alpha.is_none()
            && self.0.blend_mode.is_none()
    }

    /// Integrate another graphics state into the current one. This is done by replacing
    /// all active properties of the other graphics state in the current graphics state, while
    /// leaving the inactive ones unchanged.
    pub fn combine(&mut self, other: &ExtGState) {
        if let Some(stroking_alpha) = other.0.stroking_alpha {
            Arc::make_mut(&mut self.0).stroking_alpha = Some(stroking_alpha);
        }

        if let Some(non_stroking_alpha) = other.0.non_stroking_alpha {
            Arc::make_mut(&mut self.0).non_stroking_alpha = Some(non_stroking_alpha);
        }

        if let Some(blend_mode) = other.0.blend_mode {
            Arc::make_mut(&mut self.0).blend_mode = Some(blend_mode);
        }

        if let Some(mask) = other.0.mask {
            Arc::make_mut(&mut self.0).mask = Some(mask);
        }
    }
}

impl Object for ExtGState {
    fn chunk_container(&self) -> ChunkContainerFn {
        Box::new(|cc| &mut cc.ext_g_states)
    }

    fn serialize(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        let mut ext_st = chunk.ext_graphics(root_ref);
        if let Some(nsa) = self.0.non_stroking_alpha {
            if nsa != NormalizedF32::ONE {
                sc.register_validation_error(ValidationError::Transparency);
            }

            ext_st.non_stroking_alpha(nsa.get());
        }

        if let Some(sa) = self.0.stroking_alpha {
            if sa != NormalizedF32::ONE {
                sc.register_validation_error(ValidationError::Transparency);
            }

            ext_st.stroking_alpha(sa.get());
        }

        if let Some(bm) = self.0.blend_mode {
            if bm != BlendMode::Normal {
                sc.register_validation_error(ValidationError::Transparency);
            }

            ext_st.blend_mode(bm);
        }

        if let Some(mask_ref) = self.0.mask {
            sc.register_validation_error(ValidationError::Transparency);

            ext_st.pair(Name(b"SMask"), mask_ref);
        }

        ext_st.finish();

        chunk
    }
}

#[cfg(test)]
mod tests {
    use crate::object::ext_g_state::ExtGState;
    use crate::object::mask::Mask;
    use crate::serialize::SerializerContext;
    use crate::stream::Stream;

    use crate::mask::MaskType;
    use krilla_macros::snapshot;
    use pdf_writer::types::BlendMode;
    use usvg::NormalizedF32;

    #[snapshot]
    pub fn ext_g_state_empty(sc: &mut SerializerContext) {
        let ext_state = ExtGState::new();
        sc.add_object(ext_state);
    }

    #[snapshot]
    pub fn ext_g_state_default_values(sc: &mut SerializerContext) {
        let ext_state = ExtGState::new()
            .non_stroking_alpha(NormalizedF32::ONE)
            .stroking_alpha(NormalizedF32::ONE)
            .blend_mode(BlendMode::Normal);
        sc.add_object(ext_state);
    }

    #[snapshot]
    pub fn ext_g_state_all_set(sc: &mut SerializerContext) {
        let mask = Mask::new(Stream::empty(), MaskType::Luminosity);
        let ext_state = ExtGState::new()
            .non_stroking_alpha(NormalizedF32::new(0.4).unwrap())
            .stroking_alpha(NormalizedF32::new(0.6).unwrap())
            .blend_mode(BlendMode::Difference)
            .mask(mask, sc);
        sc.add_object(ext_state);
    }
}
