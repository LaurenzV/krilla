use crate::color::PdfColorExt;
use crate::paint::Stop;
use crate::serialize::{ObjectSerialize, PdfObject, RefAllocator, SerializeSettings};
use pdf_writer::{Chunk, Finish, Ref};
use std::sync::Arc;
use strict_num::NormalizedF32;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct StitchingFunction(Arc<Vec<Stop>>);

impl ObjectSerialize for StitchingFunction {
    fn serialize_into(
        self,
        chunk: &mut Chunk,
        ref_allocator: &mut RefAllocator,
        _: &SerializeSettings,
    ) -> Ref {
        let root_ref = ref_allocator.new_ref();
        let mut functions = vec![];
        let mut bounds = vec![];
        let mut encode = vec![];
        let mut count = 0;

        for window in self.0.windows(2) {
            let (first, second) = (&window[0], &window[1]);
            bounds.push(second.offset.get());

            let c0_components = first
                .color
                .to_pdf_components()
                .into_iter()
                .map(|n| NormalizedF32::new(n).unwrap())
                .collect::<Vec<_>>();
            let c1_components = second
                .color
                .to_pdf_components()
                .into_iter()
                .map(|n| NormalizedF32::new(n).unwrap())
                .collect::<Vec<_>>();
            debug_assert!(c0_components.len() == c1_components.len());
            count = c0_components.len();

            let exp_ref =
                ref_allocator.cached_ref(PdfObject::ExponentialFunction(ExponentialFunction(
                    Arc::new(ExponentialFunctionRepr::new(c0_components, c1_components)),
                )));

            functions.push(exp_ref);
            encode.extend([0.0, 1.0]);
        }

        bounds.pop();
        let mut stitching_function = chunk.stitching_function(root_ref);
        stitching_function.domain([0.0, 1.0]);
        stitching_function.range([0.0, 1.0].repeat(count));
        stitching_function.functions(functions);
        stitching_function.bounds(bounds);
        stitching_function.encode(encode);

        root_ref
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ExponentialFunctionRepr {
    c0: Vec<NormalizedF32>,
    c1: Vec<NormalizedF32>,
}

impl ExponentialFunctionRepr {
    pub fn new(c0: Vec<NormalizedF32>, c1: Vec<NormalizedF32>) -> Self {
        Self { c0, c1 }
    }
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
