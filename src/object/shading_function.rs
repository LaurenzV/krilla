use crate::color::rgb;
use crate::object::color::Color;
use crate::object::{ChunkContainerFn, Object};
use crate::paint::SpreadMethod;
use crate::paint::{LinearGradient, RadialGradient, SweepGradient};
use crate::resource::RegisterableResource;
use crate::serialize::SerializerContext;
use crate::util::{RectExt, RectWrapper};
use crate::validation::ValidationError;
use pdf_writer::types::{FunctionShadingType, PostScriptOp};
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use bumpalo::Bump;
use tiny_skia_path::{NormalizedF32, Point, Rect, Transform};

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

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct RadialAxialGradient {
    pub coords: Vec<f32>,
    pub shading_type: FunctionShadingType,
    pub stops: Vec<Stop>,
}

impl Eq for RadialAxialGradient {}

impl Hash for RadialAxialGradient {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for el in &self.coords {
            el.to_bits().hash(state);
        }

        self.shading_type.hash(state);
        self.stops.hash(state);
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct PostScriptGradient {
    pub min: f32,
    pub max: f32,
    pub stops: Vec<Stop>,
    pub domain: RectWrapper,
    pub spread_method: SpreadMethod,
    pub gradient_type: GradientType,
}

impl Eq for PostScriptGradient {}

impl Hash for PostScriptGradient {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.min.to_bits().hash(state);
        self.max.to_bits().hash(state);
        self.stops.hash(state);
        self.domain.hash(state);
        self.spread_method.hash(state);
        self.gradient_type.hash(state);
    }
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
    fn gradient_properties(self, bbox: Rect) -> (GradientProperties, Transform);
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

impl GradientPropertiesExt for LinearGradient {
    fn gradient_properties(self, bbox: Rect) -> (GradientProperties, Transform) {
        if self.spread_method == SpreadMethod::Pad {
            (
                GradientProperties::RadialAxialGradient(RadialAxialGradient {
                    coords: vec![self.x1, self.y1, self.x2, self.y2],
                    shading_type: FunctionShadingType::Axial,
                    stops: self.stops.0.into_iter().collect::<Vec<Stop>>(),
                }),
                self.transform,
            )
        } else {
            let p1 = Point::from_xy(self.x1, self.y1);
            let p2 = Point::from_xy(self.x2, self.y2);

            let (ts, min, max) = get_point_ts(p1, p2);
            (
                GradientProperties::PostScriptGradient(PostScriptGradient {
                    min,
                    max,
                    stops: self.stops.0.into_iter().collect::<Vec<Stop>>(),
                    domain: RectWrapper(get_expanded_bbox(bbox, self.transform.pre_concat(ts))),
                    spread_method: self.spread_method,
                    gradient_type: GradientType::Linear,
                }),
                self.transform.pre_concat(ts),
            )
        }
    }
}

impl GradientPropertiesExt for SweepGradient {
    fn gradient_properties(self, bbox: Rect) -> (GradientProperties, Transform) {
        let min = self.start_angle;
        let max = self.end_angle;

        let transform = self
            .transform
            .post_concat(Transform::from_translate(self.cx, self.cy));

        (
            GradientProperties::PostScriptGradient(PostScriptGradient {
                min,
                max,
                stops: self.stops.0.into_iter().collect::<Vec<Stop>>(),
                domain: RectWrapper(get_expanded_bbox(bbox, transform)),
                spread_method: self.spread_method,
                gradient_type: GradientType::Sweep,
            }),
            transform,
        )
    }
}

impl GradientPropertiesExt for RadialGradient {
    fn gradient_properties(self, _: Rect) -> (GradientProperties, Transform) {
        // TODO: Support other spread methods
        (
            GradientProperties::RadialAxialGradient(RadialAxialGradient {
                coords: vec![self.fx, self.fy, self.fr, self.cx, self.cy, self.cr],
                shading_type: FunctionShadingType::Radial,
                stops: self.stops.0.into_iter().collect::<Vec<Stop>>(),
            }),
            self.transform,
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

impl RegisterableResource<crate::resource::ShadingFunction> for ShadingFunction {}

impl Object for ShadingFunction {
    fn chunk_container(&self) -> ChunkContainerFn {
        Box::new(|cc| &mut cc.shading_functions)
    }

    fn serialize(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        match &self.0.properties {
            GradientProperties::RadialAxialGradient(rag) => {
                serialize_axial_radial_shading(sc, &mut chunk, root_ref, rag, self.0.use_opacities)
            }
            GradientProperties::PostScriptGradient(psg) => {
                sc.register_validation_error(ValidationError::ContainsPostScript);
                serialize_postscript_shading(sc, &mut chunk, root_ref, psg, self.0.use_opacities)
            }
        }

        chunk
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

    let bump = Bump::new();
    let function_ref = select_postscript_function(post_script_gradient, chunk, sc, &bump, use_opacities);
    let cs = if use_opacities {
        rgb::Color::luma_based_color_space(sc.serialize_settings.no_device_cs)
    } else {
        post_script_gradient.stops[0].color.color_space(sc, false)
    };

    let mut shading = chunk.function_shading(root_ref);
    shading.shading_type(FunctionShadingType::Function);

    shading.insert(Name(b"ColorSpace")).primitive(sc.add_cs(cs));
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
    let cs = if use_opacities {
        rgb::Color::luma_based_color_space(sc.serialize_settings.no_device_cs)
    } else {
        radial_axial_gradient.stops[0].color.color_space(sc, false)
    };

    let mut shading = chunk.function_shading(root_ref);
    if radial_axial_gradient.shading_type == FunctionShadingType::Radial {
        shading.shading_type(FunctionShadingType::Radial);
    } else {
        shading.shading_type(FunctionShadingType::Axial);
    }
    shading.insert(Name(b"ColorSpace")).primitive(sc.add_cs(cs));

    shading.function(function_ref);
    shading.coords(radial_axial_gradient.coords.iter().copied());
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
                    .to_pdf_color(false)
                    .into_iter()
                    .collect::<Vec<_>>(),
                stops[1]
                    .color
                    .to_pdf_color(false)
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
    bump: &Bump,
    use_opacities: bool,
) -> Ref {
    debug_assert!(properties.stops.len() > 1);

    if properties.gradient_type == GradientType::Linear {
        serialize_linear_postscript(properties, chunk, sc, use_opacities)
    } else if properties.gradient_type == GradientType::Sweep {
        serialize_sweep_postscript(properties, chunk, sc, bump, use_opacities)
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
    bump: &Bump,
    use_opacities: bool,
) -> Ref {
    use pdf_writer::types::PostScriptOp::*;

    let root_ref = sc.new_ref();

    let min: f32 = properties.min;
    let max: f32 = properties.max;

    let mut code = vec![];
    code.extend([
        // Stack: x y
        Exch,
        // y x
        // Make sure x is never 0.
        Dup,
        Dup,
        Real(0.0001),
        Lt,
        Exch,
        Real(-0.0001),
        Gt,
        And,
        If(bump.alloc([Pop, Real(0.0001)])),
        // Get the angle
        Atan,
    ]);

    encode_spread_method(min, max, &mut code, bump, properties.spread_method);
    encode_postscript_stops(&properties.stops, min, max, &mut code, bump, use_opacities);

    let encoded = PostScriptOp::encode(&code);
    let mut postscript_function = chunk.post_script_function(root_ref, &encoded);
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
    use pdf_writer::types::PostScriptOp::*;

    let bump = Bump::new();
    let root_ref = sc.new_ref();

    let min: f32 = properties.min;
    let max: f32 = properties.max;

    let mut code = vec![];
    code.extend([
        // Stack: x y
        // Ignore the y coordinate. We account for it in the gradient transform.
        Pop,
        // x
    ]);

    encode_spread_method(min, max, &mut code, &bump, properties.spread_method);
    encode_postscript_stops(&properties.stops, min, max, &mut code, &bump, use_opacities);

    let encoded = PostScriptOp::encode(&code);
    let mut postscript_function = chunk.post_script_function(root_ref, &encoded);
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
fn encode_spread_method<'a>(
    min: f32,
    max: f32,
    code: &mut Vec<PostScriptOp<'a>>,
    bump: &'a Bump,
    spread_method: SpreadMethod,
) {
    use pdf_writer::types::PostScriptOp::*;

    if spread_method == SpreadMethod::Pad {
        return;
    }

    let length = max - min;

    code.extend([
        // We do the following:
        // 1. Normalize by doing n = x - min.
        // 2. Calculate the "interval" we are in by doing i = floor(n / length)
        // 3. Calculate the offset by doing o = n - i * length
        // 4. If the spread method is repeat, we additionally calculate o = length - 0 if i % 2 == 1
        // 5. Calculate the final value with x_new = min + o.

        // Current stack:
        // x
        Real(length),
        Real(min),
        // x length min
        Integer(2),
        Index,
        // x length min x
        Integer(1),
        Index,
        // x length min x min
        Sub,
        // x length min n
        Dup,
        // x length min n n
        Integer(3),
        Index,
        // x length min n n length
        Div,
        // x length min n {n/length}
        Floor,
        // x length min n i
        Exch,
        // x length min i n
        Integer(1),
        Index,
        // x length min i n i
        Integer(4),
        Index,
        // x length min i n i length
        Mul,
        // x length min i n {i * length}
        Sub,
        // x length min i o
        Exch,
        // x length min o i
        Cvi,
        Abs,
        // x length min o abs(i)
        Integer(2),
        Mod,
        // x length min o {abs(i) % 2}
        // See https://github.com/google/skia/blob/645b77ce61449951cb9f3cf754b47d4977b68e1a/src/pdf/SkPDFGradientShader.cpp#L402-L408
        // for why we check > 0 instead of == 1.
        Integer(0),
        Gt,
        // x length min o {(abs(i) % 2) > 0}
        if spread_method == SpreadMethod::Reflect {
            If(bump.alloc([Integer(2), Index, Exch, Sub]))
        } else {
            Pop
        },
        // x length min o
        Add,
        // x length x_new
        Integer(3),
        Integer(1),
        Roll,
        // x_new x length
        Pop,
        Pop,
        // x_new
    ]);
}

/// Postscript code that, given an x coordinate between the min and max
/// of a gradient, returns the interpolated color value depending on where it
/// lies within the stops.
fn encode_postscript_stops<'a>(
    stops: &[Stop],
    min: f32,
    max: f32,
    code: &mut Vec<PostScriptOp<'a>>,
    bump: &'a Bump,
    use_opacities: bool,
) {
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

    encode_stops_impl(&stops, min, max, code, bump, use_opacities);
}

fn encode_stops_impl<'a>(
    stops: &[Stop],
    min: f32,
    max: f32,
    code: &mut Vec<PostScriptOp<'a>>,
    bump: &'a Bump,
    use_opacities: bool,
) {
    use pdf_writer::types::PostScriptOp::*;

    let encode_two_stops =
        |c0: &[f32], c1: &[f32], min: f32, max: f32, code: &mut Vec<PostScriptOp>| {
            if min == max {
                code.push(Pop);
                code.extend(c0.iter().map(|n| Real(*n)));
                return;
            }

            debug_assert_eq!(c0.len(), c1.len());

            // Normalize the x coordinate to be between 0 and 1.
            code.extend([Real(min), Sub, Real(max), Real(min), Sub, Div]);

            for i in 0..c0.len() {
                // Interpolate each color component c0 + x_norm * (x1 - c0).
                code.extend([
                    Integer(i as i32),
                    Index,
                    Real(c0[i]),
                    Exch,
                    Real(c1[i]),
                    Real(c0[i]),
                    Sub,
                    Mul,
                    Add,
                ]);
                // x_norm, c0, c1, ...
            }
            // Remove x_norm from the stack.
            code.extend([Integer((c0.len() + 1) as i32), Integer(-1), Roll, Pop]);
            // c0, c1, c2, ...
        };

    if stops.len() == 1 {
        if use_opacities {
            code.push(Real(stops[0].opacity.get()));
        } else {
            code.extend(stops[0].color.to_pdf_color(false).into_iter().map(Real));
        }
    } else {
        let length = max - min;
        let stops_min = min + length * stops[0].offset.get();
        let stops_max = min + length * stops[1].offset.get();
        // Write the if conditions to find the corresponding set of two stops.

        let mut if_stops = bump.alloc(vec![]);
        if use_opacities {
            encode_two_stops(
                &[stops[0].opacity.get()],
                &[stops[1].opacity.get()],
                stops_min,
                stops_max,
                &mut if_stops,
            )
        } else {
            encode_two_stops(
                &stops[0]
                    .color
                    .to_pdf_color(false)
                    .into_iter()
                    .collect::<Vec<_>>(),
                &stops[1]
                    .color
                    .to_pdf_color(false)
                    .into_iter()
                    .collect::<Vec<_>>(),
                stops_min,
                stops_max,
                &mut if_stops,
            )
        };
        let mut else_stops = bump.alloc(vec![]);
        encode_stops_impl(&stops[1..], min, max, &mut else_stops, bump, use_opacities);

        code.extend([Dup, Real(stops_max), Le, IfElse(if_stops, else_stops)]);
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
                first
                    .color
                    .to_pdf_color(false)
                    .into_iter()
                    .collect::<Vec<_>>(),
                second
                    .color
                    .to_pdf_color(false)
                    .into_iter()
                    .collect::<Vec<_>>(),
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
