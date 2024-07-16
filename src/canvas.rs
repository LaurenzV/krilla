use crate::bytecode::{ByteCode, Instruction};
use crate::color::PdfColorExt;
use crate::ext_g_state::{CompositeMode, ExtGState};
use crate::paint::{GradientProperties, GradientPropertiesExt, Paint, TilingPattern};
use crate::resource::{PdfColorSpace, PdfPattern, ResourceDictionary};
use crate::serialize::{ObjectSerialize, PageSerialize, SerializeSettings, SerializerContext};
use crate::shading::ShadingPattern;
use crate::transform::FiniteTransform;
use crate::util::{LineCapExt, LineJoinExt, NameExt, RectExt, TransformExt};
use crate::{ext_g_state, Fill, FillRule, LineCap, LineJoin, Stroke};
use pdf_writer::types::BlendMode;
use pdf_writer::types::ColorSpaceOperand::Pattern;
use pdf_writer::{Chunk, Content, Finish, Pdf, Ref};
use std::sync::Arc;
use tiny_skia_path::{NormalizedF32, Path, PathSegment, Rect, Size, Transform};

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Canvas {
    pub(crate) byte_code: ByteCode,
    pub(crate) size: Size,
}

impl Canvas {
    pub fn new(size: Size) -> Self {
        Self {
            byte_code: ByteCode::new(),
            size,
        }
    }

    pub fn stroke_path(
        &mut self,
        path: Path,
        transform: tiny_skia_path::Transform,
        stroke: Stroke,
    ) {
        self.byte_code.push(Instruction::StrokePath(Box::new((
            path.into(),
            transform.try_into().unwrap(),
            stroke,
        ))));
    }

    pub fn fill_path(&mut self, path: Path, transform: tiny_skia_path::Transform, fill: Fill) {
        self.byte_code.push(Instruction::FillPath(Box::new((
            path.into(),
            transform.try_into().unwrap(),
            fill,
        ))));
    }

    pub fn push_layer(&mut self) {
        self.byte_code.push(Instruction::PushLayer);
    }

    pub fn pop_layer(&mut self) {
        self.byte_code.push(Instruction::PopLayer);
    }

    pub fn set_clip_path(&mut self, path: Path, clip_rule: FillRule) {
        self.byte_code
            .push(Instruction::ClipPath(Box::new((path.into(), clip_rule))));
    }

    pub fn draw_canvas(
        &mut self,
        canvas: Canvas,
        transform: Transform,
        composite_mode: CompositeMode,
        opacity: NormalizedF32,
        isolated: bool,
    ) {
        self.byte_code.push(Instruction::DrawCanvas(Box::new((
            canvas,
            transform.try_into().unwrap(),
            composite_mode,
            opacity,
            isolated,
        ))));
    }
}

#[derive(Clone)]
struct GraphicsState {
    ext_g_state: ext_g_state::Repr,
    ctm: Transform,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ext_g_state: ext_g_state::Repr::default(),
            ctm: Transform::identity(),
        }
    }
}

impl GraphicsState {
    fn add_ext_g_state(&mut self, other: &ext_g_state::Repr) {
        self.ext_g_state.add_ext_g_state(other);
    }

    fn concat_transform(&mut self, transform: Transform) {
        self.ctm = self.ctm.pre_concat(transform);
        println!("result: {:?}", self.ctm);
    }

    fn transform(&self) -> Transform {
        self.ctm
    }
}

struct GraphicsStates {
    graphics_states: Vec<GraphicsState>,
}

impl GraphicsStates {
    fn new() -> Self {
        GraphicsStates {
            graphics_states: vec![GraphicsState::default()],
        }
    }

    fn cur(&self) -> &GraphicsState {
        self.graphics_states.last().unwrap()
    }

    fn cur_mut(&mut self) -> &mut GraphicsState {
        self.graphics_states.last_mut().unwrap()
    }

    fn save_state(&mut self) {
        let state = self.cur();
        self.graphics_states.push(state.clone())
    }

    fn restore_state(&mut self) {
        self.graphics_states.pop();
    }

    fn add_ext_g_state(&mut self, other: &ext_g_state::Repr) {
        self.cur_mut().add_ext_g_state(other);
    }

    fn transform(&mut self, transform: Transform) {
        self.cur_mut().concat_transform(transform);
    }

    fn transform_bbox(&self, bbox: Rect) -> Rect {
        bbox.transform(self.cur().transform()).unwrap()
    }
}

pub struct CanvasPdfSerializer {
    resource_dictionary: ResourceDictionary,
    content: Content,
    graphics_states: GraphicsStates,
    bbox: Rect,
    base_opacity: NormalizedF32,
}

impl CanvasPdfSerializer {
    pub fn new() -> Self {
        Self {
            resource_dictionary: ResourceDictionary::new(),
            content: Content::new(),
            graphics_states: GraphicsStates::new(),
            bbox: Rect::from_xywh(0.0, 0.0, 0.0, 0.0).unwrap(),
            base_opacity: NormalizedF32::new(1.0).unwrap(),
        }
    }

    pub fn serialize_instructions(&mut self, instructions: &[Instruction]) {
        for op in instructions {
            match op {
                Instruction::PushLayer => self.save_state(),
                Instruction::PopLayer => self.restore_state(),
                Instruction::StrokePath(stroke_data) => self.stroke_path(
                    &stroke_data.0 .0,
                    &stroke_data.1.try_into().unwrap(),
                    &stroke_data.2,
                ),
                Instruction::FillPath(fill_data) => {
                    self.fill_path(
                        &fill_data.0 .0,
                        &fill_data.1.try_into().unwrap(),
                        &fill_data.2,
                    );
                }
                Instruction::DrawCanvas(canvas_data) => {
                    self.draw_canvas(
                        &canvas_data.0,
                        &canvas_data.1.try_into().unwrap(),
                        canvas_data.2,
                        canvas_data.3,
                        canvas_data.4,
                    );
                }
                Instruction::ClipPath(clip_data) => {
                    self.set_clip_path(&clip_data.0 .0, &clip_data.1)
                }
            }
        }
    }

    pub fn set_base_opacity(&mut self, alpha: NormalizedF32) {
        if alpha.get() != 1.0 {
            self.base_opacity = self.base_opacity * alpha;
            // fill/stroke opacities are always set locally when drawing a path,
            // so here it will always be None, thus we can just apply it directly.
            let state = ExtGState::new(Some(self.base_opacity), Some(self.base_opacity), None);
            self.graphics_states.add_ext_g_state(&state);
        }
    }

    pub fn transform(&mut self, transform: &tiny_skia_path::Transform) {
        if !transform.is_identity() {
            self.graphics_states.transform(*transform);
            self.content.transform(transform.to_pdf_transform());
        }
    }

    // TODO: Panic if q_nesting level is uneven
    pub fn finish(self) -> (Vec<u8>, ResourceDictionary, Rect) {
        (self.content.finish(), self.resource_dictionary, self.bbox)
    }

    pub fn save_state(&mut self) {
        self.graphics_states.save_state();
        self.content.save_state();
    }

    pub fn restore_state(&mut self) {
        self.graphics_states.restore_state();
        self.content.restore_state();
    }

    pub fn set_fill_opacity(&mut self, alpha: NormalizedF32) {
        if alpha.get() != 1.0 {
            let state = ExtGState::new(Some(alpha * self.base_opacity), None, None);
            self.graphics_states.add_ext_g_state(&state);

            let ext = self.resource_dictionary.register_ext_g_state(state);
            self.content.set_parameters(ext.to_pdf_name());
        }
    }

    pub fn set_stroke_opacity(&mut self, alpha: NormalizedF32) {
        if alpha.get() != 1.0 {
            let state = ExtGState::new(None, Some(alpha * self.base_opacity), None);
            self.graphics_states.add_ext_g_state(&state);

            let ext = self.resource_dictionary.register_ext_g_state(state);
            self.content.set_parameters(ext.to_pdf_name());
        }
    }

    pub fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        if blend_mode != BlendMode::Normal {
            let state = ExtGState::new(None, None, Some(blend_mode));
            self.graphics_states.add_ext_g_state(&state);

            let ext = self.resource_dictionary.register_ext_g_state(state);
            self.content.set_parameters(ext.to_pdf_name());
        }
    }

    pub fn fill_path(&mut self, path: &Path, transform: &Transform, fill: &Fill) {
        self.save_state();
        self.transform(transform);

        self.bbox
            .expand(&self.graphics_states.transform_bbox(path.bounds()));

        self.set_fill_opacity(fill.opacity);

        let pattern_transform = |transform: FiniteTransform| -> FiniteTransform {
            let mut transform: Transform = transform.into();
            transform = transform.post_concat(self.graphics_states.cur().transform());
            transform.try_into().unwrap()
        };

        let mut write_gradient = |gradient_props: GradientProperties,
                                  transform: FiniteTransform| {
            let shading_pattern = ShadingPattern::new(
                gradient_props,
                self.graphics_states.cur().transform().try_into().unwrap(),
                transform,
            );
            let color_space = self
                .resource_dictionary
                .register_pattern(PdfPattern::ShadingPattern(shading_pattern));
            self.content.set_fill_color_space(Pattern);
            self.content
                .set_fill_pattern(None, color_space.to_pdf_name());
        };

        match &fill.paint {
            Paint::Color(c) => {
                let color_space = self
                    .resource_dictionary
                    .register_color_space(c.get_pdf_color_space());
                self.content.set_fill_color_space(color_space.to_pdf_name());
                self.content.set_fill_color(c.to_pdf_components());
            }
            Paint::LinearGradient(lg) => {
                let (gradient_props, transform) = lg.gradient_properties(path.bounds());
                write_gradient(gradient_props, transform);
            }
            Paint::RadialGradient(rg) => {
                let (gradient_props, transform) = rg.gradient_properties(path.bounds());
                write_gradient(gradient_props, transform);
            }
            Paint::Pattern(pat) => {
                let mut pat = pat.clone();
                let transform = pat.transform;

                Arc::make_mut(&mut pat).transform = pattern_transform(transform);

                let color_space = self
                    .resource_dictionary
                    .register_pattern(PdfPattern::TilingPattern(TilingPattern(pat.clone())));
                self.content.set_fill_color_space(Pattern);
                self.content
                    .set_fill_pattern(None, color_space.to_pdf_name());
            }
        }

        draw_path(path.segments(), &mut self.content);
        match fill.rule {
            FillRule::NonZero => self.content.fill_nonzero(),
            FillRule::EvenOdd => self.content.fill_even_odd(),
        };
        self.restore_state();
    }

    pub fn set_clip_path(&mut self, path: &Path, clip_rule: &FillRule) {
        draw_path(path.segments(), &mut self.content);

        match clip_rule {
            FillRule::NonZero => self.content.clip_nonzero(),
            FillRule::EvenOdd => self.content.clip_even_odd(),
        };

        self.content.end_path();
    }

    pub fn stroke_path(&mut self, path: &Path, transform: &Transform, stroke: &Stroke) {
        let path_bbox = path.bounds().transform(*transform).unwrap();
        self.bbox.expand(&path_bbox);

        self.save_state();
        self.transform(transform);

        self.set_stroke_opacity(stroke.opacity);

        let pattern_transform = |transform: FiniteTransform| -> FiniteTransform {
            let mut transform: Transform = transform.into();
            transform = transform.post_concat(self.graphics_states.cur().transform());
            transform.try_into().unwrap()
        };

        let mut write_gradient = |gradient_props: GradientProperties,
                                  transform: FiniteTransform| {
            let transform = pattern_transform(transform);
            let shading_pattern = ShadingPattern::new(
                gradient_props,
                self.graphics_states.cur().transform().try_into().unwrap(),
                transform,
            );
            let color_space = self
                .resource_dictionary
                .register_pattern(PdfPattern::ShadingPattern(shading_pattern));
            self.content.set_stroke_color_space(Pattern);
            self.content
                .set_stroke_pattern(None, color_space.to_pdf_name());
        };

        match &stroke.paint {
            Paint::Color(c) => {
                let color_space = self
                    .resource_dictionary
                    .register_color_space(c.get_pdf_color_space());
                self.content
                    .set_stroke_color_space(color_space.to_pdf_name());
                self.content.set_stroke_color(c.to_pdf_components());
            }
            Paint::LinearGradient(lg) => {
                let (gradient_props, transform) = lg.gradient_properties(path.bounds());
                write_gradient(gradient_props, transform);
            }
            Paint::RadialGradient(rg) => {
                let (gradient_props, transform) = rg.gradient_properties(path.bounds());
                write_gradient(gradient_props, transform);
            }
            Paint::Pattern(pat) => {
                let mut pat = pat.clone();
                let transform = pat.transform;

                Arc::make_mut(&mut pat).transform = pattern_transform(transform);
                let color_space = self
                    .resource_dictionary
                    .register_pattern(PdfPattern::TilingPattern(TilingPattern(pat.clone())));
                self.content.set_stroke_color_space(Pattern);
                self.content
                    .set_stroke_pattern(None, color_space.to_pdf_name());
            }
        }

        // Only write if they don't correspond to the default values as defined in the
        // PDF specification.
        if stroke.width.get() != 1.0 {
            self.content.set_line_width(stroke.width.get());
        }

        if stroke.miter_limit.get() != 10.0 {
            self.content.set_miter_limit(stroke.miter_limit.get());
        }

        if stroke.line_cap != LineCap::Butt {
            self.content.set_line_cap(stroke.line_cap.to_pdf_line_cap());
        }

        if stroke.line_join != LineJoin::Miter {
            self.content
                .set_line_join(stroke.line_join.to_pdf_line_join());
        }

        if let Some(stroke_dash) = &stroke.dash {
            self.content.set_dash_pattern(
                stroke_dash.array.iter().map(|n| n.get()),
                stroke_dash.offset.get(),
            );
        }

        draw_path(path.segments(), &mut self.content);
        self.content.stroke();

        self.restore_state();
    }

    pub fn draw_canvas(
        &mut self,
        canvas: &Canvas,
        transform: &Transform,
        composite_mode: CompositeMode,
        opacity: NormalizedF32,
        isolated: bool,
    ) {
        // TODO: Handle nested opacities
        // TODO: Handle transforms on gradients/patterns
        // TODO: Handle embedding as XObject
        self.save_state();
        self.set_base_opacity(opacity);
        self.transform(transform);
        if let Ok(blend_mode) = composite_mode.try_into() {
            self.set_blend_mode(blend_mode);
        } else {
            unimplemented!();
        }
        self.serialize_instructions(canvas.byte_code.instructions());
        self.restore_state()
    }
}

impl ObjectSerialize for Canvas {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let mut chunk = Chunk::new();
        let (content_stream, mut resource_dictionary, bbox) = {
            let mut serializer = CanvasPdfSerializer::new();
            serializer.serialize_instructions(self.byte_code.instructions());
            serializer.finish()
        };

        let mut x_object = chunk.form_xobject(root_ref, &content_stream);
        resource_dictionary.to_pdf_resources(sc, &mut x_object.resources());
        x_object.bbox(bbox.to_pdf_rect());
        x_object.finish();
    }
}

impl PageSerialize for Canvas {
    fn serialize(self, serialize_settings: SerializeSettings) -> Pdf {
        let mut sc = SerializerContext::new(serialize_settings);

        let catalog_ref = sc.new_ref();
        let page_tree_ref = sc.new_ref();
        let page_ref = sc.new_ref();
        let content_ref = sc.new_ref();

        let mut chunk = Chunk::new();
        chunk.pages(page_tree_ref).count(1).kids([page_ref]);

        let (content_stream, mut resource_dictionary, _) = {
            let mut serializer = CanvasPdfSerializer::new();
            // TODO: Update bbox?
            serializer.transform(&Transform::from_row(
                1.0,
                0.0,
                0.0,
                -1.0,
                0.0,
                self.size.height(),
            ));
            serializer.serialize_instructions(self.byte_code.instructions());

            serializer.finish()
        };
        chunk.stream(content_ref, &content_stream);

        let mut page = chunk.page(page_ref);
        resource_dictionary.to_pdf_resources(&mut sc, &mut page.resources());

        page.media_box(self.size.to_rect(0.0, 0.0).unwrap().to_pdf_rect());
        page.parent(page_tree_ref);
        page.contents(content_ref);
        page.finish();

        let mut pdf = Pdf::new();
        pdf.catalog(catalog_ref).pages(page_tree_ref);
        pdf.extend(&chunk);
        pdf.extend(sc.current_chunk());

        pdf
    }
}

/// Draws a path into a content stream. Note that this does not perform any stroking/filling,
/// it only creates a subpath.
fn draw_path(path_data: impl Iterator<Item = PathSegment>, content: &mut Content) {
    // Taken from resvg
    fn calc(n1: f32, n2: f32) -> f32 {
        (n1 + n2 * 2.0) / 3.0
    }

    let mut p_prev = None;

    for operation in path_data {
        match operation {
            PathSegment::MoveTo(p) => {
                content.move_to(p.x, p.y);
                p_prev = Some(p);
            }
            PathSegment::LineTo(p) => {
                content.line_to(p.x, p.y);
                p_prev = Some(p);
            }
            PathSegment::QuadTo(p1, p2) => {
                // Since PDF doesn't support quad curves, we need to convert them into
                // cubic.
                let prev = p_prev.unwrap();
                content.cubic_to(
                    calc(prev.x, p1.x),
                    calc(prev.y, p1.y),
                    calc(p2.x, p1.x),
                    calc(p2.y, p1.y),
                    p2.x,
                    p2.y,
                );
                p_prev = Some(p2);
            }
            PathSegment::CubicTo(p1, p2, p3) => {
                content.cubic_to(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
                p_prev = Some(p3);
            }
            PathSegment::Close => {
                content.close_path();
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::canvas::Canvas;
    use crate::color::Color;
    use crate::ext_g_state::CompositeMode;
    use crate::paint::{LinearGradient, Paint, Pattern, Stop, StopOffset};
    use crate::serialize::{ObjectSerialize, SerializeSettings};
    use crate::{Fill, FillRule, Stroke};
    use std::sync::Arc;
    use tiny_skia_path::{FiniteF32, NormalizedF32, Path, PathBuilder, Size, Transform};

    fn dummy_path(w: f32) -> Path {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(w, 0.0);
        builder.line_to(w, w);
        builder.line_to(0.0, w);
        builder.close();

        builder.finish().unwrap()
    }

    #[test]
    fn canvas_stroke() {
        let mut canvas = Canvas::new(Size::from_wh(100.0, 100.0).unwrap());
        canvas.stroke_path(
            dummy_path(100.0),
            Transform::from_scale(2.0, 2.0),
            Stroke::default(),
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = canvas.serialize(serialize_settings).0;
        write("pattern", &chunk.as_bytes());
    }

    #[test]
    fn canvas_page() {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(100.0, 100.0).unwrap());
        canvas.stroke_path(
            dummy_path(100.0),
            Transform::from_scale(0.5, 0.5),
            Stroke::default(),
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();
        write("canvas_page", &finished);
    }

    #[test]
    fn fill() {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(100.0, 100.0).unwrap());
        canvas.fill_path(
            dummy_path(100.0),
            Transform::from_scale(2.0, 2.0),
            Fill {
                paint: Paint::Color(Color::new_rgb(200, 0, 0)),
                opacity: NormalizedF32::new(0.25).unwrap(),
                ..Fill::default()
            },
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write("fill", &finished);
    }

    #[test]
    fn blend() {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        canvas.fill_path(
            dummy_path(100.0),
            Transform::from_translate(25.0, 25.0),
            Fill {
                paint: Paint::Color(Color::new_rgb(255, 0, 0)),
                opacity: NormalizedF32::new(0.25).unwrap(),
                ..Fill::default()
            },
        );

        let mut second = Canvas::new(Size::from_wh(100.0, 100.0).unwrap());
        second.fill_path(
            dummy_path(100.0),
            Transform::from_translate(-25.0, -25.0),
            Fill {
                paint: Paint::Color(Color::new_rgb(255, 255, 0)),
                opacity: NormalizedF32::new(1.0).unwrap(),
                ..Fill::default()
            },
        );

        canvas.draw_canvas(
            second,
            Transform::from_translate(100.0, 100.0),
            CompositeMode::Difference,
            NormalizedF32::ONE,
            false,
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write("blend", &finished);
    }

    #[test]
    fn nested_opacity() {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        canvas.fill_path(
            dummy_path(100.0),
            Transform::identity(),
            Fill {
                paint: Paint::Color(Color::new_rgb(255, 255, 0)),
                opacity: NormalizedF32::new(0.5).unwrap(),
                ..Fill::default()
            },
        );

        let mut second = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        second.fill_path(
            dummy_path(100.0),
            Transform::identity(),
            Fill {
                paint: Paint::Color(Color::new_rgb(255, 255, 0)),
                opacity: NormalizedF32::new(0.5).unwrap(),
                ..Fill::default()
            },
        );

        canvas.draw_canvas(
            second,
            Transform::from_translate(100.0, 100.0),
            CompositeMode::Difference,
            NormalizedF32::new(0.5).unwrap(),
            false,
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write("nested_opacity", &finished);
    }

    #[test]
    fn gradient_fill() {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        canvas.fill_path(
            dummy_path(100.0),
            Transform::from_scale(1.0, 1.0).try_into().unwrap(),
            Fill {
                paint: Paint::LinearGradient(LinearGradient {
                    x1: FiniteF32::new(40.0).unwrap(),
                    y1: Default::default(),
                    x2: FiniteF32::new(60.0).unwrap(),
                    y2: Default::default(),
                    transform: Transform::from_translate(0.0, 30.0)
                        .pre_concat(Transform::from_scale(0.5, 0.5))
                        .pre_concat(Transform::from_rotate_at(45.0, 90.0, 90.0))
                        .try_into()
                        .unwrap(),
                    spread_method: Default::default(),
                    stops: vec![
                        Stop {
                            offset: NormalizedF32::new(0.0).unwrap(),
                            color: Color::new_rgb(255, 0, 0),
                            opacity: NormalizedF32::ONE,
                        },
                        Stop {
                            offset: NormalizedF32::new(0.5).unwrap(),
                            color: Color::new_rgb(0, 255, 0),
                            opacity: NormalizedF32::ONE,
                        },
                        Stop {
                            offset: NormalizedF32::new(1.0).unwrap(),
                            color: Color::new_rgb(0, 0, 255),
                            opacity: NormalizedF32::ONE,
                        },
                    ],
                }),
                opacity: NormalizedF32::ONE,
                ..Fill::default()
            },
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write("gradient_fill", &finished);
    }

    #[test]
    fn clip_path() {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        canvas.push_layer();
        canvas.set_clip_path(dummy_path(100.0), FillRule::NonZero);
        canvas.fill_path(
            dummy_path(200.0),
            Transform::from_scale(1.0, 1.0),
            Fill {
                paint: Paint::Color(Color::new_rgb(200, 0, 0)),
                opacity: NormalizedF32::new(0.25).unwrap(),
                ..Fill::default()
            },
        );
        canvas.pop_layer();

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write("clip_path", &finished);
    }

    #[test]
    fn pattern() {
        use crate::serialize::PageSerialize;

        let mut pattern_canvas = Canvas::new(Size::from_wh(10.0, 10.0).unwrap());
        pattern_canvas.fill_path(
            dummy_path(5.0),
            Transform::default(),
            Fill {
                paint: Paint::Color(Color::new_rgb(0, 255, 0)),
                ..Fill::default()
            },
        );

        pattern_canvas.fill_path(
            dummy_path(5.0),
            Transform::from_translate(5.0, 5.0),
            Fill {
                paint: Paint::Color(Color::new_rgb(0, 0, 255)),
                ..Fill::default()
            },
        );

        let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        canvas.fill_path(
            dummy_path(200.0),
            Transform::from_scale(2.0, 2.0).try_into().unwrap(),
            Fill {
                paint: Paint::Pattern(Arc::new(Pattern {
                    canvas: Arc::new(pattern_canvas),
                    transform: Transform::from_rotate_at(45.0, 2.5, 2.5)
                        .try_into()
                        .unwrap(),
                })),
                opacity: NormalizedF32::ONE,
                ..Fill::default()
            },
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write("pattern", &finished);
    }

    fn write(name: &str, data: &[u8]) {
        let _ = std::fs::write(format!("out/{name}.txt"), &data);
        let _ = std::fs::write(format!("out/{name}.pdf"), &data);
    }
}
