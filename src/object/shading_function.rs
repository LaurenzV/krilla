use crate::color::PdfColorExt;
use crate::paint::{GradientProperties, GradientType, SpreadMethod, Stop};
use crate::serialize::{Object, SerializerContext};
use crate::transform::TransformWrapper;
use crate::util::RectExt;
use pdf_writer::types::FunctionShadingType;
use pdf_writer::{Finish, Name, Ref};
use std::sync::Arc;
use tiny_skia_path::{NormalizedF32, Rect};

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    pub(crate) properties: GradientProperties,
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

    fn serialize_postscript_shading(self, sc: &mut SerializerContext, root_ref: Ref) {
        let mut bbox = self.0.properties.bbox;

        let function_ref = select_postscript_function(&self.0.properties, sc, &bbox);
        let cs_ref = sc.srgb();

        let mut shading = sc.chunk_mut().function_shading(root_ref);
        // TODO: Readd axial/radial shading.
        shading.shading_type(FunctionShadingType::Function);
        shading.insert(Name(b"ColorSpace")).primitive(cs_ref);

        shading.function(function_ref);

        shading.domain([bbox.left(), bbox.right(), bbox.top(), bbox.bottom()]);
        shading.finish();
    }

    fn serialize_axial_radial_shading(self, sc: &mut SerializerContext, root_ref: Ref) {
        let function_ref = select_axial_radial_function(&self.0.properties, sc);
        let cs_ref = sc.srgb();

        let mut shading = sc.chunk_mut().function_shading(root_ref);
        if self.0.properties.shading_type == FunctionShadingType::Radial {
            shading.shading_type(FunctionShadingType::Radial);
        } else {
            shading.shading_type(FunctionShadingType::Axial);
        }
        shading.insert(Name(b"ColorSpace")).primitive(cs_ref);

        shading.function(function_ref);
        if self.0.properties.shading_type == FunctionShadingType::Radial {
            shading.coords(
                self.0
                    .properties
                    .coords
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|n| n.get()),
            );
        } else {
            shading.coords([
                self.0.properties.min.get(),
                0.0,
                self.0.properties.max.get(),
                0.0,
            ]);
        }
        shading.extend([true, true]);
        shading.finish();
    }
}

impl Object for ShadingFunction {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        if self.0.properties.gradient_type == GradientType::Sweep
            || self.0.properties.gradient_type == GradientType::Linear
        {
            self.serialize_postscript_shading(sc, root_ref);
        } else {
            self.serialize_axial_radial_shading(sc, root_ref);
        }
    }

    fn is_cached(&self) -> bool {
        true
    }
}

fn select_axial_radial_function(
    properties: &GradientProperties,
    sc: &mut SerializerContext,
) -> Ref {
    debug_assert!(properties.stops.len() > 1);

    if properties.stops.len() == 2 {
        serialize_exponential(
            &properties.stops[0].color.to_normalized_pdf_components(),
            &properties.stops[1].color.to_normalized_pdf_components(),
            sc,
        )
    } else {
        serialize_stitching(&properties.stops, sc)
    }
}

fn select_postscript_function(
    properties: &GradientProperties,
    sc: &mut SerializerContext,
    bbox: &Rect,
) -> Ref {
    debug_assert!(properties.stops.len() > 1);

    if properties.gradient_type == GradientType::Linear {
        serialize_linear_postscript(properties, sc, bbox)
    } else if properties.gradient_type == GradientType::Sweep {
        serialize_sweep_postscript(properties, sc, bbox)
    } else {
        serialize_radial_postscript(properties, sc, bbox)
    }
}

// Not working yet
fn serialize_radial_postscript(
    properties: &GradientProperties,
    sc: &mut SerializerContext,
    bbox: &Rect,
) -> Ref {
    let root_ref = sc.new_ref();

    let min: f32 = properties.min.get();
    let max: f32 = properties.max.get();

    // TODO: Improve formatting of PS code.
    let start_code = [
        "{".to_string(),
        // Stack: x y
        "80 exch 80 sub dup mul 3 1 roll sub dup mul add sqrt 120 div 0 0".to_string(),
    ];

    let end_code = ["}".to_string()];

    let mut code = Vec::new();
    code.extend(start_code);
    // code.push(encode_spread_method(min, max, properties.spread_method));
    // code.push(encode_stops(&properties.stops, min, max));
    code.extend(end_code);

    let code = code.join(" ").into_bytes();
    let mut postscript_function = sc.chunk_mut().post_script_function(root_ref, &code);
    postscript_function.domain([bbox.left(), bbox.right(), bbox.top(), bbox.bottom()]);
    postscript_function.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);

    root_ref
}

fn serialize_sweep_postscript(
    properties: &GradientProperties,
    sc: &mut SerializerContext,
    bbox: &Rect,
) -> Ref {
    let root_ref = sc.new_ref();

    let min: f32 = properties.min.get();
    let max: f32 = properties.max.get();

    // TODO: Improve formatting of PS code.
    let start_code = [
        "{".to_string(),
        // Stack: x y
        "exch".to_string(),
        // y x
        // Make sure x is never 0.
        "dup dup 0.0001 lt exch -0.0001 gt and {pop 0.0001} if ".to_string(),
        // Get the angle
        "atan".to_string(),
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

fn serialize_linear_postscript(
    properties: &GradientProperties,
    sc: &mut SerializerContext,
    bbox: &Rect,
) -> Ref {
    let root_ref = sc.new_ref();

    let min: f32 = properties.min.get();
    let max: f32 = properties.max.get();

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
    // Our algorithm requires the stops to be padded.
    let mut stops = stops.iter().cloned().collect::<Vec<_>>();

    if let Some(first) = stops.first() {
        let mut first = first.clone();
        first.offset = NormalizedF32::ZERO;
        stops.insert(0, first);
    }

    if let Some(last) = stops.last() {
        let mut last = last.clone();
        last.offset = NormalizedF32::ONE;
        stops.push(last);
    }

    encode_stops_impl(&stops, min, max)
}

fn encode_stops_impl(stops: &[Stop], min: f32, max: f32) -> String {
    let encode_two_stops = |c0: &[f32], c1: &[f32], min: f32, max: f32| {
        if min == max {
            return format!(
                "pop {}",
                c0.iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }

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

        snippets.join(" ")
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
        let length = max - min;
        let stops_min = min + length * stops[0].offset.get();
        let stops_max = min + length * stops[1].offset.get();
        // Write the if conditions to find the corresponding set of two stops.
        format!(
            "dup {} le {{{}}} {{{}}} ifelse",
            stops_max,
            encode_two_stops(
                &stops[0].color.to_pdf_components(),
                &stops[1].color.to_pdf_components(),
                stops_min,
                stops_max,
            ),
            encode_stops_impl(&stops[1..], min, max)
        )
    };
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
