use crate::bytecode::{ByteCode, Instruction};
use crate::color::PdfColorExt;
use crate::graphics_state::GraphicsStates;
use crate::object::ext_g_state::ExtGState;
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_pattern::ShadingPattern;
use crate::object::tiling_pattern::TilingPattern;
use crate::object::xobject::XObject;
use crate::paint::{GradientProperties, GradientPropertiesExt, Paint};
use crate::resource::{PatternResource, Resource, ResourceDictionary, XObjectResource};
use crate::serialize::{Object, PageSerialize, SerializeSettings, SerializerContext};
use crate::transform::TransformWrapper;
use crate::util::{LineCapExt, LineJoinExt, NameExt, RectExt, TransformExt};
use crate::{Fill, FillRule, LineCap, LineJoin, Stroke};
use pdf_writer::types::BlendMode;
use pdf_writer::{Chunk, Content, Finish, Pdf, Ref};
use std::sync::Arc;
use tiny_skia_path::{NormalizedF32, Path, PathSegment, Rect, Size, Transform};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
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
            TransformWrapper(transform),
            stroke,
        ))));
    }

    pub fn fill_path(&mut self, path: Path, transform: tiny_skia_path::Transform, fill: Fill) {
        self.byte_code.push(Instruction::FillPath(Box::new((
            path.into(),
            TransformWrapper(transform),
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

    pub fn draw_image(&mut self, image: Image, size: Size, transform: Transform) {
        self.byte_code.push(Instruction::DrawImage(Box::new((
            image,
            size,
            TransformWrapper(transform),
        ))))
    }

    pub fn draw_canvas(
        &mut self,
        canvas: Canvas,
        transform: Transform,
        composite_mode: BlendMode,
        opacity: NormalizedF32,
        isolated: bool,
        mask: Option<Mask>,
    ) {
        self.byte_code.push(Instruction::DrawCanvas(Box::new((
            Arc::new(canvas),
            TransformWrapper(transform),
            composite_mode,
            opacity,
            isolated,
            mask,
        ))));
    }
}

pub struct CanvasPdfSerializer<'a> {
    resource_dictionary: &'a mut ResourceDictionary,
    content: Content,
    graphics_states: GraphicsStates,
    bbox: Rect,
    base_opacity: NormalizedF32,
    isolate: bool,
}

impl<'a> CanvasPdfSerializer<'a> {
    pub fn new(resource_dictionary: &'a mut ResourceDictionary) -> Self {
        Self {
            resource_dictionary,
            content: Content::new(),
            graphics_states: GraphicsStates::new(),
            bbox: Rect::from_xywh(0.0, 0.0, 0.0, 0.0).unwrap(),
            base_opacity: NormalizedF32::new(1.0).unwrap(),
            isolate: false,
        }
    }

    pub fn serialize_instructions(&mut self, instructions: &[Instruction]) {
        for op in instructions {
            match op {
                Instruction::PushLayer => self.save_state(),
                Instruction::PopLayer => self.restore_state(),
                Instruction::StrokePath(stroke_data) => {
                    self.stroke_path(&stroke_data.0 .0, &stroke_data.1 .0, &stroke_data.2)
                }
                Instruction::FillPath(fill_data) => {
                    self.fill_path(&fill_data.0 .0, &fill_data.1 .0, &fill_data.2);
                }
                Instruction::DrawImage(image_data) => {
                    self.draw_image(image_data.0.clone(), image_data.1, &image_data.2 .0)
                }
                Instruction::DrawCanvas(canvas_data) => {
                    self.draw_canvas(
                        canvas_data.0.clone(),
                        &canvas_data.1 .0,
                        canvas_data.2,
                        canvas_data.3,
                        canvas_data.4,
                        canvas_data.5.clone(),
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
            let state = ExtGState::new()
                .stroking_alpha(alpha)
                .non_stroking_alpha(alpha);
            self.graphics_states.combine(&state);
        }
    }

    pub fn transform(&mut self, transform: &tiny_skia_path::Transform) {
        if !transform.is_identity() {
            self.graphics_states.transform(*transform);
            self.content.transform(transform.to_pdf_transform());
        }
    }

    // TODO: Panic if q_nesting level is uneven
    pub fn finish(self) -> (Vec<u8>, Rect) {
        (self.content.finish(), self.bbox)
    }

    pub fn save_state(&mut self) {
        self.graphics_states.save_state();
        self.content.save_state();
    }

    pub fn restore_state(&mut self) {
        self.graphics_states.restore_state();
        self.content.restore_state();
    }

    pub fn set_mask(&mut self, mask: Mask) {
        let state = ExtGState::new().mask(mask);
        self.graphics_states.combine(&state);

        let ext = self
            .resource_dictionary
            .register_resource(Resource::ExtGState(state));
        self.content.set_parameters(ext.to_pdf_name());
    }

    pub fn set_fill_opacity(&mut self, alpha: NormalizedF32) {
        if alpha.get() != 1.0 {
            let state = ExtGState::new().non_stroking_alpha(alpha * self.base_opacity);
            self.graphics_states.combine(&state);

            let ext = self
                .resource_dictionary
                .register_resource(Resource::ExtGState(state));
            self.content.set_parameters(ext.to_pdf_name());
        }
    }

    pub fn set_stroke_opacity(&mut self, alpha: NormalizedF32) {
        if alpha.get() != 1.0 {
            let state = ExtGState::new().stroking_alpha(alpha * self.base_opacity);
            self.graphics_states.combine(&state);

            let ext = self
                .resource_dictionary
                .register_resource(Resource::ExtGState(state));
            self.content.set_parameters(ext.to_pdf_name());
        }
    }

    pub fn set_blend_mode(&mut self, blend_mode: BlendMode) {
        if blend_mode != BlendMode::Normal {
            let state = ExtGState::new().blend_mode(blend_mode);
            self.graphics_states.combine(&state);

            let ext = self
                .resource_dictionary
                .register_resource(Resource::ExtGState(state));
            self.content.set_parameters(ext.to_pdf_name());
        }
    }

    pub fn fill_path(&mut self, path: &Path, transform: &Transform, fill: &Fill) {
        self.save_state();
        self.transform(transform);

        self.bbox
            .expand(&self.graphics_states.transform_bbox(path.bounds()));

        self.set_fill_opacity(fill.opacity);

        let pattern_transform = |transform: TransformWrapper| -> TransformWrapper {
            TransformWrapper(
                transform
                    .0
                    .post_concat(self.graphics_states.cur().transform()),
            )
        };

        let mut write_gradient = |gradient_props: GradientProperties,
                                  transform: TransformWrapper| {
            let shading_pattern = ShadingPattern::new(
                gradient_props,
                TransformWrapper(self.graphics_states.cur().transform()),
                transform,
            );
            let color_space = self
                .resource_dictionary
                .register_resource(Resource::Pattern(PatternResource::ShadingPattern(
                    shading_pattern,
                )));
            self.content
                .set_fill_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
            self.content
                .set_fill_pattern(None, color_space.to_pdf_name());
        };

        match &fill.paint {
            Paint::Color(c) => {
                let color_space = self
                    .resource_dictionary
                    .register_resource(Resource::ColorSpace(c.get_pdf_color_space()));
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
                    .register_resource(Resource::Pattern(PatternResource::TilingPattern(
                        TilingPattern::new(pat.canvas.clone(), pat.transform),
                    )));
                self.content
                    .set_fill_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
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

        let pattern_transform = |transform: TransformWrapper| -> TransformWrapper {
            TransformWrapper(
                transform
                    .0
                    .post_concat(self.graphics_states.cur().transform()),
            )
        };

        let mut write_gradient = |gradient_props: GradientProperties,
                                  transform: TransformWrapper| {
            let transform = pattern_transform(transform);
            let shading_pattern = ShadingPattern::new(
                gradient_props,
                TransformWrapper(self.graphics_states.cur().transform()),
                transform,
            );
            let color_space = self
                .resource_dictionary
                .register_resource(Resource::Pattern(PatternResource::ShadingPattern(
                    shading_pattern,
                )));
            self.content
                .set_stroke_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
            self.content
                .set_stroke_pattern(None, color_space.to_pdf_name());
        };

        match &stroke.paint {
            Paint::Color(c) => {
                let color_space = self
                    .resource_dictionary
                    .register_resource(Resource::ColorSpace(c.get_pdf_color_space()));
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
                    .register_resource(Resource::Pattern(PatternResource::TilingPattern(
                        TilingPattern::new(pat.canvas.clone(), pat.transform),
                    )));

                self.content
                    .set_stroke_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
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

    pub fn draw_image(&mut self, image: Image, size: Size, transform: &Transform) {
        let image_name = self
            .resource_dictionary
            .register_resource(Resource::XObject(XObjectResource::Image(image)));
        // Apply user-supplied transform and scale the image from 1x1 to the actual dimensions.
        let transform = transform.pre_concat(Transform::from_row(
            size.width(),
            0.0,
            0.0,
            -size.height(),
            0.0,
            size.height(),
        ));
        self.save_state();
        self.transform(&transform);
        self.content.x_object(image_name.to_pdf_name());
        self.restore_state()
    }

    pub fn draw_canvas(
        &mut self,
        canvas: Arc<Canvas>,
        transform: &Transform,
        composite_mode: BlendMode,
        opacity: NormalizedF32,
        isolated: bool,
        mask: Option<Mask>,
    ) {
        // TODO: Handle embedding as XObject
        // TODO: Nested masks
        self.save_state();
        self.set_base_opacity(opacity);
        self.transform(transform);
        if let Ok(blend_mode) = composite_mode.try_into() {
            self.set_blend_mode(blend_mode);
        } else {
            unimplemented!();
        }

        if mask.is_some() || isolated {
            let x_object = XObject::new(canvas.clone(), isolated, mask.is_some());

            if let Some(mask) = mask {
                self.set_mask(mask);
            }
            let name = self
                .resource_dictionary
                .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
            self.content.x_object(name.to_pdf_name());
        } else {
            self.serialize_instructions(canvas.byte_code.instructions());
        }

        self.restore_state()
    }
}

// TODO: Add ProcSet?
// TODO: Deduplicate with other implementations
// impl ObjectSerialize for Canvas {
//     fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
//         let mut chunk = Chunk::new();
//         let mut resource_dictionary = ResourceDictionary::new();
//         let (content_stream, bbox) = {
//             let mut serializer = CanvasPdfSerializer::new(&mut resource_dictionary);
//             serializer.serialize_instructions(self.byte_code.instructions());
//             serializer.finish()
//         };
//
//         let mut x_object = chunk.form_xobject(root_ref, &content_stream);
//         resource_dictionary.to_pdf_resources(sc, &mut x_object.resources());
//         x_object.bbox(bbox.to_pdf_rect());
//         x_object.finish();
//     }
// }

impl PageSerialize for Canvas {
    fn serialize(self, serialize_settings: SerializeSettings) -> Pdf {
        let mut sc = SerializerContext::new(serialize_settings);

        let catalog_ref = sc.new_ref();
        let page_tree_ref = sc.new_ref();
        let page_ref = sc.new_ref();
        let content_ref = sc.new_ref();

        let mut chunk = Chunk::new();
        chunk.pages(page_tree_ref).count(1).kids([page_ref]);

        let mut resource_dictionary = ResourceDictionary::new();
        let (content_stream, _) = {
            let mut serializer = CanvasPdfSerializer::new(&mut resource_dictionary);
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
        pdf.extend(sc.chunk());

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
    use crate::object::image::Image;
    use crate::object::mask::{Mask, MaskType};
    use crate::paint::{LinearGradient, Paint, Pattern, SpreadMethod, Stop, StopOffset};
    use crate::serialize::{PageSerialize, SerializeSettings};
    use crate::transform::TransformWrapper;
    use crate::{Fill, FillRule, Stroke};
    use pdf_writer::types::BlendMode;
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

        let chunk = canvas.serialize(serialize_settings);
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
            BlendMode::Difference,
            NormalizedF32::ONE,
            false,
            None,
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
            BlendMode::Difference,
            NormalizedF32::new(0.5).unwrap(),
            false,
            None,
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
                    transform: TransformWrapper(
                        Transform::from_translate(0.0, 30.0)
                            .pre_concat(Transform::from_scale(0.5, 0.5))
                            .pre_concat(Transform::from_rotate_at(45.0, 90.0, 90.0)),
                    ),
                    spread_method: SpreadMethod::Reflect,
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
                    transform: TransformWrapper(Transform::from_rotate_at(45.0, 2.5, 2.5)),
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

    #[test]
    fn mask_luminance() {
        use crate::serialize::PageSerialize;

        let mut mask_canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        mask_canvas.fill_path(
            dummy_path(200.0),
            Transform::default(),
            Fill {
                paint: Paint::Color(Color::new_rgb(255, 0, 0)),
                opacity: NormalizedF32::new(1.0).unwrap(),
                ..Fill::default()
            },
        );

        let mut path_canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        path_canvas.fill_path(
            dummy_path(200.0),
            Transform::identity().try_into().unwrap(),
            Fill {
                paint: Paint::Color(Color::new_rgb(255, 0, 0)),
                opacity: NormalizedF32::ONE,
                ..Fill::default()
            },
        );

        let mut final_canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        final_canvas.draw_canvas(
            path_canvas,
            Transform::identity(),
            BlendMode::Normal,
            NormalizedF32::ONE,
            false,
            Some(Mask::new(Arc::new(mask_canvas), MaskType::Luminance)),
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = PageSerialize::serialize(final_canvas, serialize_settings);
        let finished = chunk.finish();

        write("mask_luminance", &finished);
    }

    #[test]
    fn png_image() {
        use crate::serialize::PageSerialize;
        let image_data = include_bytes!("../data/image.png");
        let dynamic_image = image::load_from_memory(image_data).unwrap();
        let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        canvas.draw_image(
            Image::new(&dynamic_image),
            Size::from_wh(50.0, 50.0).unwrap(),
            Transform::from_translate(20.0, 20.0),
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write("png_image", &finished);
    }

    fn write(name: &str, data: &[u8]) {
        let _ = std::fs::write(format!("out/{name}.txt"), &data);
        let _ = std::fs::write(format!("out/{name}.pdf"), &data);
    }
}
