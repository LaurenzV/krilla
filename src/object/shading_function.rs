use crate::chunk_container::ChunkContainer;
use crate::error::KrillaResult;
use crate::object::color::{Color, ColorSpace};
use crate::object::Object;
use crate::paint::SpreadMethod;
use crate::paint::{LinearGradient, RadialGradient, SweepGradient};
use crate::serialize::SerializerContext;
use crate::util::TransformWrapper;
use crate::util::{RectExt, RectWrapper};
use pdf_writer::types::FunctionShadingType;
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, NormalizedF32, Point, Rect, Transform};

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum GradientType {
    Sweep,
    Linear,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub(crate) struct Stop {
    pub offset: NormalizedF32,
    pub color: Color,
    pub opacity: NormalizedF32,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) struct RadialAxialGradient {
    pub coords: Vec<FiniteF32>,
    pub shading_type: FunctionShadingType,
    pub stops: Vec<Stop>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) struct PostScriptGradient {
    pub min: FiniteF32,
    pub max: FiniteF32,
    pub stops: Vec<Stop>,
    pub domain: RectWrapper,
    pub spread_method: SpreadMethod,
    pub gradient_type: GradientType,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) enum GradientProperties {
    RadialAxialGradient(RadialAxialGradient),
    PostScriptGradient(PostScriptGradient),
}

impl GradientProperties {
    // Check if the gradient could be encoded as a solid fill instead.
    pub fn single_stop_color(&self) -> Option<(Color, NormalizedF32)> {
        match self {
            GradientProperties::RadialAxialGradient(rag) => {
                if rag.stops.len() == 1 {
                    return Some((rag.stops[0].color, rag.stops[0].opacity));
                }
            }
            GradientProperties::PostScriptGradient(psg) => {
                if psg.stops.len() == 1 {
                    return Some((psg.stops[0].color, psg.stops[0].opacity));
                }
            }
        }

        None
    }
}

pub(crate) trait GradientPropertiesExt {
    fn gradient_properties(self, bbox: Rect) -> (GradientProperties, TransformWrapper);
}

fn get_expanded_bbox(mut bbox: Rect, shading_transform: Transform) -> Rect {
    // We need to make sure the shading covers the whole bbox of the object after
    // the transform as been applied. In order to know that, we need to calculate the
    // resulting bbox from the inverted transform.
    bbox.expand(&bbox.transform(shading_transform.invert().unwrap()).unwrap());
    bbox
}

/// WHen writing a PostScript shading, we assume that both points are on a horizontal
/// line. Here, we calculate by how much we need to rotate the second point so that it
/// is horizontal to the first point, as well as the position of the rotated point.
fn get_point_ts(start: Point, end: Point) -> (Transform, f32, f32) {
    let dist = start.distance(end);

    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let angle = dy.atan2(dx).to_degrees();

    (
        Transform::from_rotate_at(angle, start.x, start.y),
        start.x,
        start.x + dist,
    )
}

impl<C> GradientPropertiesExt for LinearGradient<C>
where
    C: ColorSpace,
{
    fn gradient_properties(self, bbox: Rect) -> (GradientProperties, TransformWrapper) {
        if self.spread_method == SpreadMethod::Pad {
            (
                GradientProperties::RadialAxialGradient(RadialAxialGradient {
                    coords: vec![
                        FiniteF32::new(self.x1).unwrap(),
                        FiniteF32::new(self.y1).unwrap(),
                        FiniteF32::new(self.x2).unwrap(),
                        FiniteF32::new(self.y2).unwrap(),
                    ],
                    shading_type: FunctionShadingType::Axial,
                    stops: self
                        .stops
                        .into_iter()
                        .map(|s| s.into())
                        .collect::<Vec<Stop>>(),
                }),
                TransformWrapper(self.transform),
            )
        } else {
            let p1 = Point::from_xy(self.x1, self.y1);
            let p2 = Point::from_xy(self.x2, self.y2);

            let (ts, min, max) = get_point_ts(p1, p2);
            (
                GradientProperties::PostScriptGradient(PostScriptGradient {
                    min: FiniteF32::new(min).unwrap(),
                    max: FiniteF32::new(max).unwrap(),
                    stops: self
                        .stops
                        .into_iter()
                        .map(|s| s.into())
                        .collect::<Vec<Stop>>(),
                    domain: RectWrapper(get_expanded_bbox(bbox, self.transform.pre_concat(ts))),
                    spread_method: self.spread_method,
                    gradient_type: GradientType::Linear,
                }),
                TransformWrapper(self.transform.pre_concat(ts)),
            )
        }
    }
}

impl<C> GradientPropertiesExt for SweepGradient<C>
where
    C: ColorSpace,
{
    fn gradient_properties(self, bbox: Rect) -> (GradientProperties, TransformWrapper) {
        let min = self.start_angle;
        let max = self.end_angle;

        let transform = self
            .transform
            .post_concat(Transform::from_translate(self.cx, self.cy));

        (
            GradientProperties::PostScriptGradient(PostScriptGradient {
                min: FiniteF32::new(min).unwrap(),
                max: FiniteF32::new(max).unwrap(),
                stops: self
                    .stops
                    .into_iter()
                    .map(|s| s.into())
                    .collect::<Vec<Stop>>(),
                domain: RectWrapper(get_expanded_bbox(bbox, transform)),
                spread_method: self.spread_method,
                gradient_type: GradientType::Sweep,
            }),
            TransformWrapper(transform),
        )
    }
}

impl<C> GradientPropertiesExt for RadialGradient<C>
where
    C: ColorSpace,
{
    fn gradient_properties(self, _: Rect) -> (GradientProperties, TransformWrapper) {
        // TODO: Support other spread methods
        (
            GradientProperties::RadialAxialGradient(RadialAxialGradient {
                coords: vec![
                    FiniteF32::new(self.fx).unwrap(),
                    FiniteF32::new(self.fy).unwrap(),
                    FiniteF32::new(self.fr).unwrap(),
                    FiniteF32::new(self.cx).unwrap(),
                    FiniteF32::new(self.cy).unwrap(),
                    FiniteF32::new(self.cr).unwrap(),
                ],
                shading_type: FunctionShadingType::Radial,
                stops: self
                    .stops
                    .into_iter()
                    .map(|s| s.into())
                    .collect::<Vec<Stop>>(),
            }),
            TransformWrapper(self.transform),
        )
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    pub properties: GradientProperties,
    pub use_opacities: bool,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub(crate) struct ShadingFunction(Arc<Repr>);

impl ShadingFunction {
    pub fn new(properties: GradientProperties, use_opacities: bool) -> Self {
        Self(Arc::new(Repr {
            properties,
            use_opacities,
        }))
    }
}

impl Object for ShadingFunction {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.shading_functions
    }

    fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        match &self.0.properties {
            GradientProperties::RadialAxialGradient(rag) => {
                serialize_axial_radial_shading(sc, &mut chunk, root_ref, rag, self.0.use_opacities)
            }
            GradientProperties::PostScriptGradient(psg) => {
                serialize_postscript_shading(sc, &mut chunk, root_ref, psg, self.0.use_opacities)
            }
        }

        Ok(chunk)
    }
}

fn serialize_postscript_shading(
    sc: &mut SerializerContext,
    chunk: &mut Chunk,
    root_ref: Ref,
    post_script_gradient: &PostScriptGradient,
    use_opacities: bool,
) {
    let domain = post_script_gradient.domain;

    let function_ref = select_postscript_function(post_script_gradient, chunk, sc, use_opacities);
    let cs = if use_opacities { sc.gray() } else { sc.rgb() };

    let mut shading = chunk.function_shading(root_ref);
    shading.shading_type(FunctionShadingType::Function);

    shading.insert(Name(b"ColorSpace")).primitive(cs);
    // Write the identity matrix, because ghostscript has a bug where
    // it thinks the entry is mandatory.
    shading.matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    shading.function(function_ref);

    shading.domain([domain.left(), domain.right(), domain.top(), domain.bottom()]);
    shading.finish();
}

fn serialize_axial_radial_shading(
    sc: &mut SerializerContext,
    chunk: &mut Chunk,
    root_ref: Ref,
    radial_axial_gradient: &RadialAxialGradient,
    use_opacities: bool,
) {
    let function_ref =
        select_axial_radial_function(radial_axial_gradient, chunk, sc, use_opacities);
    let cs_ref = if use_opacities { sc.gray() } else { sc.rgb() };

    let mut shading = chunk.function_shading(root_ref);
    if radial_axial_gradient.shading_type == FunctionShadingType::Radial {
        shading.shading_type(FunctionShadingType::Radial);
    } else {
        shading.shading_type(FunctionShadingType::Axial);
    }
    shading.insert(Name(b"ColorSpace")).primitive(cs_ref);

    shading.function(function_ref);
    shading.coords(radial_axial_gradient.coords.iter().map(|n| n.get()));
    shading.extend([true, true]);
    shading.finish();
}

fn select_axial_radial_function(
    properties: &RadialAxialGradient,
    chunk: &mut Chunk,
    sc: &mut SerializerContext,
    use_opacities: bool,
) -> Ref {
    debug_assert!(properties.stops.len() > 1);

    let mut stops = properties.stops.clone();

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

    if stops.len() == 2 {
        if use_opacities {
            serialize_exponential(
                vec![stops[0].opacity.get()],
                vec![stops[1].opacity.get()],
                chunk,
                sc,
            )
        } else {
            serialize_exponential(
                stops[0]
                    .color
                    .to_pdf_color()
                    .into_iter()
                    .collect::<Vec<_>>(),
                stops[1]
                    .color
                    .to_pdf_color()
                    .into_iter()
                    .collect::<Vec<_>>(),
                chunk,
                sc,
            )
        }
    } else {
        serialize_stitching(&stops, chunk, sc, use_opacities)
    }
}

fn select_postscript_function(
    properties: &PostScriptGradient,
    chunk: &mut Chunk,
    sc: &mut SerializerContext,
    use_opacities: bool,
) -> Ref {
    debug_assert!(properties.stops.len() > 1);

    if properties.gradient_type == GradientType::Linear {
        serialize_linear_postscript(properties, chunk, sc, use_opacities)
    } else if properties.gradient_type == GradientType::Sweep {
        serialize_sweep_postscript(properties, chunk, sc, use_opacities)
    } else {
        todo!();
        // serialize_radial_postscript(properties, sc, bbox)
    }
}

// Not working yet
// fn serialize_radial_postscript(
//     properties: &GradientProperties,
//     sc: &mut SerializerContext,
//     bbox: &Rect,
// ) -> Ref {
// let root_ref = sc.new_ref();
//
// // TODO: Improve formatting of PS code.
// let start_code = [
//     "{".to_string(),
//     // Stack: x y
//     "80 exch 80 sub dup mul 3 1 roll sub dup mul add sqrt 120 div 0 0".to_string(),
// ];
//
// let end_code = ["}".to_string()];
//
// let mut code = Vec::new();
// code.extend(start_code);
// // code.push(encode_spread_method(min, max, properties.spread_method));
// // code.push(encode_stops(&properties.stops, min, max));
// code.extend(end_code);
//
// let code = code.join(" ").into_bytes();
// let mut postscript_function = sc.chunk_mut().post_script_function(root_ref, &code);
// postscript_function.domain([bbox.left(), bbox.right(), bbox.top(), bbox.bottom()]);
// postscript_function.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
//
// root_ref
// }

fn serialize_sweep_postscript(
    properties: &PostScriptGradient,
    chunk: &mut Chunk,
    sc: &mut SerializerContext,
    use_opacities: bool,
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
    code.push(encode_postscript_stops(
        &properties.stops,
        min,
        max,
        use_opacities,
    ));
    code.extend(end_code);

    let code = code.join(" ").into_bytes();
    let mut postscript_function = chunk.post_script_function(root_ref, &code);
    postscript_function.domain([
        properties.domain.left(),
        properties.domain.right(),
        properties.domain.top(),
        properties.domain.bottom(),
    ]);

    if use_opacities {
        postscript_function.range([0.0, 1.0]);
    } else {
        postscript_function.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
    }

    root_ref
}

fn serialize_linear_postscript(
    properties: &PostScriptGradient,
    chunk: &mut Chunk,
    sc: &mut SerializerContext,
    use_opacities: bool,
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
    code.push(encode_postscript_stops(
        &properties.stops,
        min,
        max,
        use_opacities,
    ));
    code.extend(end_code);

    let code = code.join(" ").into_bytes();
    let mut postscript_function = chunk.post_script_function(root_ref, &code);
    postscript_function.domain([
        properties.domain.left(),
        properties.domain.right(),
        properties.domain.top(),
        properties.domain.bottom(),
    ]);

    if use_opacities {
        postscript_function.range([0.0, 1.0]);
    } else {
        postscript_function.range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
    }

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
        (if spread_method == SpreadMethod::Reflect {
            "{2 index exch sub} if"
        } else {
            "pop"
        })
        .to_string(),
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
fn encode_postscript_stops(stops: &[Stop], min: f32, max: f32, use_opacities: bool) -> String {
    // Our algorithm requires the stops to be padded.
    let mut stops = stops.to_vec();

    if let Some(first) = stops.first() {
        let mut first = *first;
        first.offset = NormalizedF32::ZERO;
        stops.insert(0, first);
    }

    if let Some(last) = stops.last() {
        let mut last = *last;
        last.offset = NormalizedF32::ONE;
        stops.push(last);
    }

    encode_stops_impl(&stops, min, max, use_opacities)
}

fn encode_stops_impl(stops: &[Stop], min: f32, max: f32, use_opacities: bool) -> String {
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

    if stops.len() == 1 {
        if use_opacities {
            stops[0].opacity.to_string()
        } else {
            stops[0]
                .color
                .to_pdf_color()
                .into_iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        }
    } else {
        let length = max - min;
        let stops_min = min + length * stops[0].offset.get();
        let stops_max = min + length * stops[1].offset.get();
        // Write the if conditions to find the corresponding set of two stops.

        let encoded_stops = if use_opacities {
            encode_two_stops(
                &[stops[0].opacity.get()],
                &[stops[1].opacity.get()],
                stops_min,
                stops_max,
            )
        } else {
            encode_two_stops(
                &stops[0]
                    .color
                    .to_pdf_color()
                    .into_iter()
                    .collect::<Vec<_>>(),
                &stops[1]
                    .color
                    .to_pdf_color()
                    .into_iter()
                    .collect::<Vec<_>>(),
                stops_min,
                stops_max,
            )
        };

        format!(
            "dup {} le {{{}}} {{{}}} ifelse",
            stops_max,
            encoded_stops,
            encode_stops_impl(&stops[1..], min, max, use_opacities)
        )
    }
}

fn serialize_stitching(
    stops: &[Stop],
    chunk: &mut Chunk,
    sc: &mut SerializerContext,
    use_opacities: bool,
) -> Ref {
    let root_ref = sc.new_ref();
    let mut functions = vec![];
    let mut bounds = vec![];
    let mut encode = vec![];
    let mut count = 0;

    for window in stops.windows(2) {
        let (first, second) = (&window[0], &window[1]);
        bounds.push(second.offset.get());

        let (c0_components, c1_components) = if use_opacities {
            (vec![first.opacity.get()], vec![second.opacity.get()])
        } else {
            (
                first.color.to_pdf_color().into_iter().collect::<Vec<_>>(),
                second.color.to_pdf_color().into_iter().collect::<Vec<_>>(),
            )
        };
        debug_assert!(c0_components.len() == c1_components.len());
        count = c0_components.len();

        let exp_ref = serialize_exponential(c0_components, c1_components, chunk, sc);

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
    first_comps: Vec<f32>,
    second_comps: Vec<f32>,
    chunk: &mut Chunk,
    sc: &mut SerializerContext,
) -> Ref {
    let root_ref = sc.new_ref();
    debug_assert_eq!(first_comps.len(), second_comps.len());
    let num_components = first_comps.len();

    let mut exp = chunk.exponential_function(root_ref);

    exp.range([0.0, 1.0].repeat(num_components));
    exp.c0(first_comps);
    exp.c1(second_comps);
    exp.domain([0.0, 1.0]);
    exp.n(1.0);
    exp.finish();
    root_ref
}

// No tests because we test directly via shading pattern.
