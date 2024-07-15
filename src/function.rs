use crate::serialize::{ObjectSerialize, PdfObject, RefAllocator, SerializeSettings};
use pdf_writer::{Chunk, Finish, Ref};
use std::sync::Arc;
use strict_num::NormalizedF32;

// #[derive(Debug, Hash, Eq, PartialEq, Clone)]
// pub struct StitchingFunction(Box<Vec<Stop>>);

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ExponentialFunctionRepr {
    c0: Vec<NormalizedF32>,
    c1: Vec<NormalizedF32>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ExponentialFunction(Arc<ExponentialFunctionRepr>);

impl ObjectSerialize for ExponentialFunction {
    fn serialize_into(
        self,
        chunk: &mut Chunk,
        ref_allocator: &mut RefAllocator,
        serialize_settings: &SerializeSettings,
    ) -> Ref {
        let root_ref = ref_allocator.cached_ref(PdfObject::ExponentialFunction(self.clone()));
        debug_assert_eq!(self.0.c0.len(), self.0.c1.len());
        let num_components = self.0.c0.len();

        let mut exp = chunk.exponential_function(root_ref);

        exp.range([0.0, 1.0].repeat(num_components));
        exp.c0(self.0.c0.clone().into_iter().map(|n| n.get()));
        exp.c1(self.0.c1.clone().into_iter().map(|n| n.get()));
        exp.domain([0.0, 1.0]);
        exp.n(1.0);
        exp.finish();
        root_ref
    }
}
