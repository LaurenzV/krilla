use crate::color::PdfColorExt;
use crate::paint::{GradientProperties, SpreadMethod, Stop};
use crate::serialize::{Object, SerializerContext};
use crate::transform::TransformWrapper;
use crate::util::RectExt;
use pdf_writer::types::FunctionShadingType;
use pdf_writer::{Finish, Name, Ref};
use std::sync::Arc;
use tiny_skia_path::Rect;

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    properties: GradientProperties,
    shading_transform: TransformWrapper,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ShadingFunction(Arc<Repr>);

impl ShadingFunction {
    pub fn new(properties: GradientProperties, shading_transform: TransformWrapper) -> Self {
        Self(Arc::new(Repr {
            properties,
            shading_transform,
        }))
    }

    pub fn shading_transform(&self) -> TransformWrapper {
        self.0.shading_transform
    }
}

impl Object for ShadingFunction {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let mut bbox = self.0.properties.bbox;
        // We need to make sure the shading covers the whole bbox of the object after
        // the transform as been applied. In order to know that, we need to calculate the
        // resulting bbox from the inverted transform.
        bbox.expand(
            &bbox
                .transform(self.0.shading_transform.0.invert().unwrap())
                .unwrap(),
        );

        let function_ref = serialize_stop_function(&self.0.properties, sc, &bbox);
        let cs_ref = sc.srgb();

        let mut shading = sc.chunk_mut().function_shading(root_ref);
        // TODO: Readd axial/radial shading.
        shading.shading_type(FunctionShadingType::Function);
        shading.insert(Name(b"ColorSpace")).primitive(cs_ref);

        shading.function(function_ref);

        shading.domain([bbox.left(), bbox.right(), bbox.top(), bbox.bottom()]);
        // shading.coords(self.0.coords.iter().map(|n| n.get()));
        // shading.extend([true, true]);
        shading.finish();
    }

    fn is_cached(&self) -> bool {
        true
    }
}

fn serialize_stop_function(
    properties: &GradientProperties,
    sc: &mut SerializerContext,
    bbox: &Rect,
) -> Ref {
    debug_assert!(properties.stops.len() > 1);
    select_function(properties, sc, bbox)
}

fn select_function(
    properties: &GradientProperties,
    sc: &mut SerializerContext,
    bbox: &Rect,
) -> Ref {
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

fn serialize_postscript(
    properties: &GradientProperties,
    sc: &mut SerializerContext,
    bbox: &Rect,
) -> Ref {
    let root_ref = sc.new_ref();

    // Assumes that y0 = y1 and x1 <= x2
    // TODO: Fix the above
    let min: f32 = properties.coords[0].get();
    let max: f32 = properties.coords[2].get();

    // TODO: Improve formatting of PS code.
    let start_code = [
        "{".to_string(),
        // Stack: x y
        // Ignore the y coordinate. We account for it in the gradient transform.
        "pop".to_string(),
        // x
    ];

    let end_code = ["}".to_string()];

    let mut code = Vec::new();
    code.extend(start_code);
    code.push(encode_spread_method(min, max, properties.spread_method));
    code.push(encode_stops(&properties.stops, min, max));
    code.extend(end_code);

    let code = code.join(" ").into_bytes();
    let mut postscript_function = sc.chunk_mut().post_script_function(root_ref, &code);
    postscript_function.domain([bbox.left(), bbox.right(), bbox.top(), bbox.bottom()]);
    postscript_function.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);

    root_ref
}

/// Postscript code that, given an arbitrary x coordinate, normalizes it to an x coordinate
/// between min and max that yields the correct color, depending on the spread mode. In the case
/// of the `Pad` spread methods, the coordinate will not be normalized since the Postscript functions
/// assign the correct value by default.
fn encode_spread_method(min: f32, max: f32, spread_method: SpreadMethod) -> String {
    if spread_method == SpreadMethod::Pad {
        return "".to_string();
    }

    let length = max - min;

    [
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
            if spread_method == SpreadMethod::Reflect {
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
    ]
    .join(" ")
}

/// Postscript code that, given an x coordinate between the min and max
/// of a gradient, returns the interpolated color value depending on where it
/// lies within the stops.
fn encode_stops(stops: &[Stop], min: f32, max: f32) -> String {
    // Our algorithm requires the first stop to be padded, since we work in windows of step size 2.
    let mut padded_stops = stops.iter().cloned().collect::<Vec<_>>();
    let first = padded_stops[0].clone();
    padded_stops.insert(0, first);

    let encode_two_stops = |c0: &[f32], c1: &[f32]| {
        debug_assert_eq!(c0.len(), c1.len());
        debug_assert!(c0.len() > 1);

        let mut snippets = vec![
            // Normalize the x coordinate to be between 0 and 1.
            format!("{min} sub {max} {min} sub div"),
        ];

        for i in 0..c0.len() {
            // Interpolate each color component c0 + x_norm * (x1 - c0).
            snippets.push(format!(
                "{} index {} exch {} {} sub mul add",
                i, c0[i], c1[i], c0[i]
            ));
            // x_norm, c0, c1, ...
        }
        // Remove x_norm from the stack.
        snippets.push(format!("{} -1 roll pop", c0.len() + 1));
        // c0, c1, c2, ...

        snippets
    };

    return if stops.len() == 1 {
        stops[0]
            .color
            .to_pdf_components()
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        let denormalized_offset = min + stops[1].offset.get() * (max - min);
        // Write the if conditions to find the corresponding set of two stops.
        format!(
            "dup {} le {{{}}} {{{}}} ifelse",
            denormalized_offset,
            encode_two_stops(
                &stops[0].color.to_pdf_components(),
                &stops[1].color.to_pdf_components()
            )
            .join(" "),
            encode_stops(&stops[1..], min, max)
        )
    };
}

// fn serialize_stitching(stops: &[Stop], sc: &mut SerializerContext) -> Ref {
//     let root_ref = sc.new_ref();
//     let mut functions = vec![];
//     let mut bounds = vec![];
//     let mut encode = vec![];
//     let mut count = 0;
//
//     for window in stops.windows(2) {
//         let (first, second) = (&window[0], &window[1]);
//         bounds.push(second.offset.get());
//
//         let c0_components = first.color.to_normalized_pdf_components();
//         let c1_components = second.color.to_normalized_pdf_components();
//         debug_assert!(c0_components.len() == c1_components.len());
//         count = c0_components.len();
//
//         let exp_ref = serialize_exponential(&c0_components, &c1_components, sc);
//
//         functions.push(exp_ref);
//         encode.extend([0.0, 1.0]);
//     }
//
//     bounds.pop();
//     let mut stitching_function = sc.chunk_mut().stitching_function(root_ref);
//     stitching_function.domain([0.0, 1.0]);
//     stitching_function.range([0.0, 1.0].repeat(count));
//     stitching_function.functions(functions);
//     stitching_function.bounds(bounds);
//     stitching_function.encode(encode);
//
//     root_ref
// }
//
// fn serialize_exponential(
//     first_comps: &[NormalizedF32],
//     second_comps: &[NormalizedF32],
//     sc: &mut SerializerContext,
// ) -> Ref {
//     let root_ref = sc.new_ref();
//     debug_assert_eq!(first_comps.len(), second_comps.len());
//     let num_components = first_comps.len();
//
//     let mut exp = sc.chunk_mut().exponential_function(root_ref);
//
//     exp.range([0.0, 1.0].repeat(num_components));
//     exp.c0(first_comps.into_iter().map(|n| n.get()));
//     exp.c1(second_comps.into_iter().map(|n| n.get()));
//     exp.domain([0.0, 1.0]);
//     exp.n(1.0);
//     exp.finish();
//     root_ref
// }
