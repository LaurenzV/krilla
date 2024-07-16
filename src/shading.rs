use crate::color::PdfColorExt;
use crate::paint::{GradientProperties, Stop};
use crate::resource::PdfColorSpace;
use crate::serialize::{CacheableObject, ObjectSerialize, SerializerContext};
use crate::transform::FiniteTransform;
use crate::util::{RectExt, TransformExt};
use pdf_writer::types::FunctionShadingType;
use pdf_writer::{Finish, Name, Ref};
use std::sync::Arc;
use tiny_skia_path::{NormalizedF32, Rect};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ShadingPattern(Arc<GradientProperties>, FiniteTransform);

impl ShadingPattern {
    pub fn new(gradient_properties: GradientProperties, transform: FiniteTransform) -> Self {
        Self(Arc::new(gradient_properties), transform)
    }
}

impl ObjectSerialize for ShadingPattern {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let shading_function = ShadingFunction(self.0.clone(), self.1);
        let shading_ref = sc.add_cached(CacheableObject::ShadingFunction(shading_function));
        let mut shading_pattern = sc.chunk_mut().shading_pattern(root_ref);
        shading_pattern.pair(Name(b"Shading"), shading_ref);
        shading_pattern.matrix(self.1.to_pdf_transform());
        shading_pattern.finish();
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ShadingFunction(Arc<GradientProperties>, FiniteTransform);

impl ObjectSerialize for ShadingFunction {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let mut bbox = self.0.bbox;
        // bbox.expand(&bbox.transform(self.1.into()).unwrap());

        let function_ref = serialize_stop_function(self.0.as_ref(), sc, &bbox);

        let cs_ref = sc.add_cached(CacheableObject::PdfColorSpace(PdfColorSpace::SRGB));

        let mut shading = sc.chunk_mut().function_shading(root_ref);
        shading.shading_type(FunctionShadingType::Function);
        shading.insert(Name(b"ColorSpace")).primitive(cs_ref);

        shading.function(function_ref);

        shading.domain([bbox.left(), bbox.right(), bbox.top(), bbox.bottom()]);
        // shading.coords(self.0.coords.iter().map(|n| n.get()));
        // shading.extend([true, true]);
        shading.finish();
    }
}

fn serialize_stop_function(properties: &GradientProperties, sc: &mut SerializerContext, bbox: &Rect) -> Ref {
    debug_assert!(properties.stops.len() > 1);

    // fn pad_stops(mut stops: Vec<Stop>) -> Vec<Stop> {
    //     // We manually pad the stops if necessary so that they are always in the range from 0-1
    //     if let Some(first) = stops.first() {
    //         if first.offset != 0.0 {
    //             let mut new_stop = *first;
    //             new_stop.offset = NormalizedF32::ZERO;
    //             stops.insert(0, new_stop);
    //         }
    //     }
    //
    //     if let Some(last) = stops.last() {
    //         if last.offset != 1.0 {
    //             let mut new_stop = *last;
    //             new_stop.offset = NormalizedF32::ONE;
    //             stops.push(new_stop);
    //         }
    //     }
    //
    //     stops
    // }

    // let stops = pad_stops(stops);
    select_function(properties, sc, bbox)
}

fn select_function(properties: &GradientProperties, sc: &mut SerializerContext, bbox: &Rect) -> Ref {
    // if stops.len() == 2 {
    //     serialize_exponential(
    //         &stops[0].color.to_normalized_pdf_components(),
    //         &stops[1].color.to_normalized_pdf_components(),
    //         sc,
    //     )
    // } else {
    //     serialize_stitching(stops, sc)
    // }
    serialize_postscript(properties, sc, bbox)
}

fn serialize_postscript(properties: &GradientProperties, sc: &mut SerializerContext, bbox: &Rect) -> Ref {
    let root_ref = sc.new_ref();

    // Assumes that y0 = y1 and x1 <= x2

    let min: f32 = properties.coords[0].get();
    let max: f32 = properties.coords[2].get();
    let length = max - min;

    let mirror = false;

    let start_code = [
        "{".to_string(),
        // Stack: x y
        // Ignore the y coordinate. We account for it in the gradient transform.
        "pop".to_string(),
        // x
    ];

    let spread_method_program = [
        // We do the following:
        // 1. Normalize by doing n = x - min.
        // 2. Calculate the "interval" we are in by doing i = floor(n / length)
        // 3. Calculate the offset by doing o = n - i * length
        // 4. If the spread method is repeat, we additionally calculate o = length - 0 if i % 2 == 1
        // 5. Calculate the final value with x_new = min + o.

        // Current stack:
        // x
        format!("{length} {min}"),
        // x length min
        "2 index".to_string(),
        // x length min x
        "1 index".to_string(),
        // x length min x min
        "sub".to_string(),
        // x length min n
        "dup".to_string(),
        // x length min n n
        "3 index".to_string(),
        // x length min n n length
        "div".to_string(),
        // x length min n {n/length}
        "floor".to_string(),
        // x length min n i
        "exch".to_string(),
        // x length min i n
        "1 index".to_string(),
        // x length min i n i
        "4 index".to_string(),
        // x length min i n i length
        "mul".to_string(),
        // x length min i n {i * length}
        "sub".to_string(),
        // x length min i o
        "exch".to_string(),
        // x length min o i
        "cvi".to_string(),
        "abs".to_string(),
        // x length min o abs(i)
        "2 mod".to_string(),
        // x length min o {abs(i) % 2}
        // See https://github.com/google/skia/blob/645b77ce61449951cb9f3cf754b47d4977b68e1a/src/pdf/SkPDFGradientShader.cpp#L402-L408
        // for why we check > 0 instead of == 1.
        "0 gt".to_string(),
        // x length min o {(abs(i) % 2) > 0}
        format!(
            "{}",
            if mirror {
                "{2 index exch sub} if"
            } else {
                "pop"
            }
        ),
        // x length min o
        "add".to_string(),
        // x length x_new
        "3 1 roll".to_string(),
        // x_new x length
        "pop pop".to_string(),
        // x_new
    ];

    fn encode_stops(stops: &[Stop], min: f32, max: f32) -> String {
        let encode_two_stops = |c0: &[f32], c1: &[f32]| {
            debug_assert_eq!(c0.len(), c1.len());
            debug_assert!(c0.len() > 1);

            let mut snippets = vec![
                // Normalize x_new to be between 0 and 1.
                format!("{min} sub {max} {min} sub div"),
            ];

            for i in 0..c0.len() {
                snippets.push(format!(
                    "{} index {} exch {} {} sub mul add",
                    i, c0[i], c1[i], c0[i]
                ));
                // x_norm, c0, c1, ...
            }
            snippets.push(format!("{} -1 roll pop", c0.len() + 1));
            // c0, c1, c2, ...

            snippets
        };

        return if stops.len() == 1 {
            stops[0].color.to_pdf_components().iter().map(|n| n.to_string()).collect::<Vec<_>>().join(" ")
        } else {
            let denormalized_offset = min + stops[1].offset.get() * (max - min);
            format!(
                "dup {} le {{{}}} {{{}}} ifelse",
                denormalized_offset,
                encode_two_stops(&stops[0].color.to_pdf_components(), &stops[1].color.to_pdf_components()).join(" "),
                encode_stops(&stops[1..], min, max)
            )
        };
    }

    let end_code = ["}".to_string()];

    let mut padded_stops = properties.stops.clone();
    let first = padded_stops[0].clone();
    padded_stops.insert(0, first);

    let mut code = Vec::new();
    code.extend(start_code);
    code.extend(spread_method_program);
    code.extend(vec![encode_stops(&padded_stops, min, max)]);
    code.extend(end_code);

    let code = code.join(" ").into_bytes();
    let mut postscript_function = sc.chunk_mut().post_script_function(root_ref, &code);
    postscript_function.domain([bbox.left(), bbox.right(), bbox.top(), bbox.bottom()]);
    postscript_function.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);

    root_ref
}

fn serialize_stitching(stops: &[Stop], sc: &mut SerializerContext) -> Ref {
    let root_ref = sc.new_ref();
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

        let exp_ref = serialize_exponential(&c0_components, &c1_components, sc);

        functions.push(exp_ref);
        encode.extend([0.0, 1.0]);
    }

    bounds.pop();
    let mut stitching_function = sc.chunk_mut().stitching_function(root_ref);
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
    sc: &mut SerializerContext,
) -> Ref {
    let root_ref = sc.new_ref();
    debug_assert_eq!(first_comps.len(), second_comps.len());
    let num_components = first_comps.len();

    let mut exp = sc.chunk_mut().exponential_function(root_ref);

    exp.range([0.0, 1.0].repeat(num_components));
    exp.c0(first_comps.into_iter().map(|n| n.get()));
    exp.c1(second_comps.into_iter().map(|n| n.get()));
    exp.domain([0.0, 1.0]);
    exp.n(1.0);
    exp.finish();
    root_ref
}
