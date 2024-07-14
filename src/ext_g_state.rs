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
