use crate::chunk_container::ChunkContainer;
use crate::object::mask::Mask;
use crate::serialize::{Object, SerializerContext};
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
    mask: Option<Arc<Mask>>,
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
    pub fn mask(mut self, mask: Mask) -> Self {
        Arc::make_mut(&mut self.0).mask = Some(Arc::new(mask));
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

        if let Some(mask) = other.0.mask.clone() {
            Arc::make_mut(&mut self.0).mask = Some(mask);
        }
    }
}

impl Object for ExtGState {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.ext_g_states
    }

    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        let mask_ref = self
            .0
            .mask
            .clone()
            .map(|ma| sc.add_object(Arc::unwrap_or_clone(ma)));

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

        if let Some(mask_ref) = mask_ref {
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
    use crate::serialize::{SerializeSettings, SerializerContext};
    use crate::stream::Stream;
    use crate::test_utils::check_snapshot;
    use crate::MaskType;
    use pdf_writer::types::BlendMode;
    use usvg::NormalizedF32;

    fn sc() -> SerializerContext {
        let settings = SerializeSettings::default_test();
        SerializerContext::new(settings)
    }

    #[test]
    pub fn empty() {
        let mut sc = sc();
        let ext_state = ExtGState::new();
        sc.add_object(ext_state);
        check_snapshot("ext_g_state/empty", sc.finish().as_bytes());
    }

    #[test]
    pub fn default_values() {
        let mut sc = sc();
        let ext_state = ExtGState::new()
            .non_stroking_alpha(NormalizedF32::ONE)
            .stroking_alpha(NormalizedF32::ONE)
            .blend_mode(BlendMode::Normal);
        sc.add_object(ext_state);
        check_snapshot("ext_g_state/default_values", sc.finish().as_bytes());
    }

    #[test]
    pub fn all_set() {
        let mut sc = sc();
        let mask = Mask::new(Stream::empty(), MaskType::Luminosity);
        let ext_state = ExtGState::new()
            .non_stroking_alpha(NormalizedF32::new(0.4).unwrap())
            .stroking_alpha(NormalizedF32::new(0.6).unwrap())
            .blend_mode(BlendMode::Difference)
            .mask(mask);
        sc.add_object(ext_state);
        check_snapshot("ext_g_state/all_set", sc.finish().as_bytes());
    }
}
