use crate::color::PdfColorExt;
use crate::paint::Stop;
use crate::serialize::{ObjectSerialize, RefAllocator};
use pdf_writer::{Chunk, Finish, Ref};
use strict_num::NormalizedF32;

fn serialize_stop_function(stops: Vec<Stop>, chunk: &mut Chunk, ref_allocator: &mut RefAllocator) -> Ref {
    debug_assert!(stops.len() > 1);

    fn pad_stops(mut stops: Vec<Stop>) -> Vec<Stop> {
        // We manually pad the stops if necessary so that they are always in the range from 0-1
        if let Some(first) = stops.first() {
            if first.offset != 0.0 {
                let mut new_stop = *first;
                new_stop.offset = NormalizedF32::ZERO;
                stops.insert(0, new_stop);
            }
        }

        if let Some(last) = stops.last() {
            if last.offset != 1.0 {
                let mut new_stop = *last;
                new_stop.offset = NormalizedF32::ONE;
                stops.push(new_stop);
            }
        }

        stops
    }

    let stops = pad_stops(stops);
    select_function(&stops, chunk, ref_allocator)
}

fn select_function(stops: &[Stop], chunk: &mut Chunk, ref_allocator: &mut RefAllocator) -> Ref {
    if stops.len() == 2 {
        serialize_exponential(
            &stops[0].color.to_normalized_pdf_components(),
            &stops[1].color.to_normalized_pdf_components(),
            chunk,
            ref_allocator,
        )
    } else {
        serialize_stitching(stops, chunk, ref_allocator)
    }
}

fn serialize_stitching(stops: &[Stop], chunk: &mut Chunk, ref_allocator: &mut RefAllocator) -> Ref {
    let root_ref = ref_allocator.new_ref();
    let mut functions = vec![];
    let mut bounds = vec![];
    let mut encode = vec![];
    let mut count = 0;

    for window in stops.windows(2) {
        let (first, second) = (&window[0], &window[1]);
        bounds.push(second.offset.get());

        let c0_components = first.color.to_normalized_pdf_components();
        let c1_components = second.color.to_normalized_pdf_components();
        debug_assert!(c0_components.len() == c1_components.len());
        count = c0_components.len();

        let exp_ref = serialize_exponential(&c0_components, &c1_components, chunk, ref_allocator);

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

fn serialize_exponential(
    first_comps: &[NormalizedF32],
    second_comps: &[NormalizedF32],
    chunk: &mut Chunk,
    ref_allocator: &mut RefAllocator,
) -> Ref {
    let root_ref = ref_allocator.new_ref();
    debug_assert_eq!(first_comps.len(), second_comps.len());
    let num_components = first_comps.len();

    let mut exp = chunk.exponential_function(root_ref);

    exp.range([0.0, 1.0].repeat(num_components));
    exp.c0(first_comps.into_iter().map(|n| n.get()));
    exp.c1(second_comps.into_iter().map(|n| n.get()));
    exp.domain([0.0, 1.0]);
    exp.n(1.0);
    exp.finish();
    root_ref
}
