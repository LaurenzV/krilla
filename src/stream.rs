// use crate::font::Font;
use crate::graphics_state::GraphicsStates;
use crate::object::ext_g_state::ExtGState;
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_function::{GradientProperties, GradientPropertiesExt, ShadingFunction};
use crate::object::shading_pattern::ShadingPattern;
use crate::object::tiling_pattern::TilingPattern;
use crate::object::xobject::XObject;
use crate::resource::{
    PatternResource, Resource, ResourceDictionary, ResourceDictionaryBuilder, XObjectResource,
};
use crate::serialize::{PDFGlyph, SerializerContext};
use crate::transform::TransformWrapper;
use crate::util::{calculate_stroke_bbox, LineCapExt, LineJoinExt, NameExt, RectExt, TransformExt};
use crate::{Color, Fill, FillRule, LineCap, LineJoin, Paint, PathWrapper, PdfColorExt, Stroke};
use pdf_writer::types::TextRenderingMode;
use pdf_writer::{Content, Str};
use skrifa::GlyphId;
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, NormalizedF32, Path, PathSegment, Rect, Size, Transform};

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Stream {
    content: Vec<u8>,
    bbox: Rect,
    resource_dictionary: ResourceDictionary,
}

impl Stream {
    pub fn content(&self) -> &[u8] {
        self.content.as_slice()
    }

    pub fn bbox(&self) -> Rect {
        self.bbox
    }

    pub fn resource_dictionary(&self) -> &ResourceDictionary {
        &self.resource_dictionary
    }
}

pub struct StreamBuilder<'a> {
    rd_builder: ResourceDictionaryBuilder,
    serializer_context: &'a mut SerializerContext,
    content: Content,
    graphics_states: GraphicsStates,
    bbox: Rect,
}

impl<'a> StreamBuilder<'a> {
    pub fn new(serializer_context: &'a mut SerializerContext) -> Self {
        Self {
            rd_builder: ResourceDictionaryBuilder::new(),
            serializer_context,
            content: Content::new(),
            graphics_states: GraphicsStates::new(),
            bbox: Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
        }
    }

    pub fn serializer_context(&mut self) -> &mut SerializerContext {
        &mut self.serializer_context
    }

    pub fn finish(self) -> Stream {
        Stream {
            content: self.content.finish(),
            bbox: self.bbox,
            resource_dictionary: self.rd_builder.finish(),
        }
    }

    pub fn sub_builder(&'a mut self) -> StreamBuilder<'a> {
        StreamBuilder::new(&mut self.serializer_context)
    }

    pub fn concat_transform(&mut self, transform: &Transform) {
        self.graphics_states.transform(*transform);
    }

    pub fn save_graphics_state(&mut self) {
        self.graphics_states.save_state();
    }

    pub fn restore_graphics_state(&mut self) {
        self.graphics_states.restore_state();
    }

    pub fn set_blend_mode(&mut self, blend_mode: pdf_writer::types::BlendMode) {
        if blend_mode != pdf_writer::types::BlendMode::Normal {
            let state = ExtGState::new().blend_mode(blend_mode);
            self.graphics_states.combine(&state);
        }
    }

    pub fn draw_fill_path(&mut self, path: &Path, fill: &Fill) {
        if path.bounds().width() == 0.0 || path.bounds().height() == 0.0 {
            return;
        }

        self.bbox
            .expand(&self.graphics_states.transform_bbox(path.bounds()));

        self.apply_isolated_op(|sb| {
            sb.content_set_fill_properties(path.bounds(), fill);
            sb.content_draw_path(path.segments());

            match fill.rule {
                FillRule::NonZero => sb.content.fill_nonzero(),
                FillRule::EvenOdd => sb.content.fill_even_odd(),
            };
        });
    }

    pub fn draw_stroke_path(&mut self, path: &Path, stroke: &Stroke) {
        if path.bounds().width() == 0.0 && path.bounds().height() == 0.0 {
            return;
        }

        let stroke_bbox = calculate_stroke_bbox(stroke, path).unwrap();
        self.bbox
            .expand(&self.graphics_states.transform_bbox(stroke_bbox));

        self.apply_isolated_op(|sb| {
            sb.content_set_stroke_properties(stroke_bbox, stroke);
            sb.content_draw_path(path.segments());
            sb.content.stroke();
        });
    }

    pub fn push_clip_path(&mut self, path: &Path, clip_rule: &FillRule) {
        self.content_save_state();
        self.content_draw_path(path.segments());

        match clip_rule {
            FillRule::NonZero => self.content.clip_nonzero(),
            FillRule::EvenOdd => self.content.clip_even_odd(),
        };

        self.content.end_path();
    }

    pub fn pop_clip_path(&mut self) {
        self.content_restore_state();
    }

    // pub fn draw_fill_glyph(
    //     &mut self,
    //     glyph_id: GlyphId,
    //     font: Font,
    //     size: FiniteF32,
    //     transform: &Transform,
    //     fill: &Fill,
    // ) {
    //     let (font_resource, gid) = self.serializer_context.map_glyph(font.clone(), glyph_id);
    //     let font_name = self
    //         .rd_builder
    //         .register_resource(Resource::Font(font_resource));
    //
    //     self.apply_isolated_op(|sb| {
    //         // TODO: Figure out proper bbox
    //         sb.content_set_fill_properties(Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(), fill);
    //
    //         sb.content.begin_text();
    //         sb.content.set_font(font_name.to_pdf_name(), size.get());
    //         sb.content.set_text_rendering_mode(TextRenderingMode::Fill);
    //         match gid {
    //             PDFGlyph::ColorGlyph(gid) => {
    //                 sb.content.set_text_matrix(transform.to_pdf_transform());
    //                 sb.content.show(Str(&[gid]));
    //             }
    //             PDFGlyph::CID(cid) => {
    //                 let transform = transform.pre_concat(Transform::from_scale(1.0, -1.0));
    //                 sb.content.set_text_matrix(transform.to_pdf_transform());
    //                 sb.content
    //                     .show(Str(&[(cid >> 8) as u8, (cid & 0xff) as u8]));
    //             }
    //         }
    //         sb.content.end_text();
    //     })
    // }

    pub fn draw_masked(&mut self, mask: Mask, stream: Arc<Stream>) {
        self.apply_isolated_op(move |sb| {
            let state = ExtGState::new().mask(mask);
            sb.graphics_states.combine(&state);

            let x_object = XObject::new(stream, false, true, None);
            let x_object_name = sb
                .rd_builder
                .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
            sb.content.x_object(x_object_name.to_pdf_name());
        })
    }

    pub fn draw_opacified(&mut self, opacity: NormalizedF32, stream: Arc<Stream>) {
        self.apply_isolated_op(move |sb| {
            let ext_state = ExtGState::new()
                .stroking_alpha(opacity)
                .non_stroking_alpha(opacity);
            sb.graphics_states.combine(&ext_state);

            let x_object = XObject::new(stream, true, false, None);
            let x_object_name = sb
                .rd_builder
                .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
            sb.content.x_object(x_object_name.to_pdf_name());
        })
    }

    pub fn draw_isolated(&mut self, stream: Stream) {
        self.apply_isolated_op(|sb| {
            let x_object = XObject::new(Arc::new(stream), true, false, None);
            let x_object_name = sb
                .rd_builder
                .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
            sb.content.x_object(x_object_name.to_pdf_name());
        });
    }

    pub fn draw_image(&mut self, image: Image, size: Size) {
        self.save_graphics_state();
        // Scale the image from 1x1 to the actual dimensions.
        let transform =
            Transform::from_row(size.width(), 0.0, 0.0, -size.height(), 0.0, size.height());
        self.concat_transform(&transform);

        self.apply_isolated_op(move |sb| {
            let image_name = sb
                .rd_builder
                .register_resource(Resource::XObject(XObjectResource::Image(image)));

            sb.content.x_object(image_name.to_pdf_name());
        });

        self.restore_graphics_state();
    }

    pub(crate) fn draw_shading(&mut self, shading: &ShadingFunction) {
        self.apply_isolated_op(|sb| {
            let sh = sb
                .rd_builder
                .register_resource(Resource::Shading(shading.clone()));
            sb.content.shading(sh.to_pdf_name());
        })
    }

    fn set_fill_opacity(&mut self, alpha: NormalizedF32) {
        if alpha.get() != 1.0 {
            let state = ExtGState::new().non_stroking_alpha(alpha);
            self.graphics_states.combine(&state);
        }
    }

    fn set_stroke_opacity(&mut self, alpha: NormalizedF32) {
        if alpha.get() != 1.0 {
            let state = ExtGState::new().stroking_alpha(alpha);
            self.graphics_states.combine(&state);
        }
    }

    fn apply_isolated_op(&mut self, mut op: impl FnOnce(&mut Self)) {
        self.save_graphics_state();
        self.content_save_state();
        self.content_set_ext_state();
        self.content_set_transform();

        op(self);

        self.content_restore_state();
        self.restore_graphics_state();
    }

    fn content_save_state(&mut self) {
        self.content.save_state();
    }

    fn content_restore_state(&mut self) {
        self.content.restore_state();
    }

    fn content_set_ext_state(&mut self) {
        let state = self.graphics_states.cur().ext_g_state().clone();
        let ext = self
            .rd_builder
            .register_resource(Resource::ExtGState(state));
        self.content.set_parameters(ext.to_pdf_name());
    }

    fn content_set_transform(&mut self) {
        let transform = self.graphics_states.cur().transform();

        if transform != Transform::identity() {
            self.content.transform(transform.to_pdf_transform());
        }
    }

    fn content_set_fill_stroke_properties(
        &mut self,
        bounds: Rect,
        paint: &Paint,
        opacity: NormalizedF32,
        mut set_pattern_fn: impl FnMut(&mut Content, String),
        mut set_solid_fn: impl FnMut(&mut Content, String, &Color),
    ) {
        let pattern_transform = |transform: TransformWrapper| -> TransformWrapper {
            TransformWrapper(
                transform
                    .0
                    .post_concat(self.graphics_states.cur().transform()),
            )
        };

        let mut write_gradient = |gradient_props: GradientProperties,
                                  transform: TransformWrapper| {
            let shading_mask = Mask::new_from_shading(gradient_props.clone(), transform, bounds);

            let shading_pattern = ShadingPattern::new(
                gradient_props,
                TransformWrapper(
                    self.graphics_states
                        .cur()
                        .transform()
                        .pre_concat(transform.0),
                ),
            );
            let color_space = self.rd_builder.register_resource(Resource::Pattern(
                PatternResource::ShadingPattern(shading_pattern),
            ));

            if let Some(shading_mask) = shading_mask {
                let state = ExtGState::new().mask(shading_mask);

                let ext = self
                    .rd_builder
                    .register_resource(Resource::ExtGState(state));
                self.content.set_parameters(ext.to_pdf_name());
            }

            set_pattern_fn(&mut self.content, color_space);
        };

        match paint {
            Paint::Color(c) => {
                let color_space = self
                    .rd_builder
                    .register_resource(Resource::ColorSpace(c.get_pdf_color_space()));
                set_solid_fn(&mut self.content, color_space, c);
            }
            Paint::LinearGradient(lg) => {
                let (gradient_props, transform) = lg.gradient_properties(bounds);
                write_gradient(gradient_props, transform);
            }
            Paint::RadialGradient(rg) => {
                let (gradient_props, transform) = rg.gradient_properties(bounds);
                write_gradient(gradient_props, transform);
            }
            Paint::SweepGradient(sg) => {
                let (gradient_props, transform) = sg.gradient_properties(bounds);
                write_gradient(gradient_props, transform);
            }
            Paint::Pattern(pat) => {
                let mut pat = pat.clone();
                let transform = pat.transform;

                Arc::make_mut(&mut pat).transform = pattern_transform(transform);

                let color_space = self.rd_builder.register_resource(Resource::Pattern(
                    PatternResource::TilingPattern(TilingPattern::new(
                        pat.stream.clone(),
                        pat.transform,
                        opacity,
                        pat.width,
                        pat.height,
                    )),
                ));
                set_pattern_fn(&mut self.content, color_space);
            }
        }
    }

    fn content_set_fill_properties(&mut self, bounds: Rect, fill: &Fill) {
        // PDF viewers don't show patterns with fill/stroke opacities consistently.
        // Because of this, the opacity is accounted for in the pattern itself.
        if !matches!(fill.paint, Paint::Pattern(_)) {
            self.set_fill_opacity(fill.opacity);
        }

        fn set_pattern_fn(content: &mut Content, color_space: String) {
            content.set_fill_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
            content.set_fill_pattern(None, color_space.to_pdf_name());
        }

        fn set_solid_fn(content: &mut Content, color_space: String, color: &Color) {
            content.set_fill_color_space(color_space.to_pdf_name());
            content.set_fill_color(color.to_pdf_components());
        }

        self.content_set_fill_stroke_properties(
            bounds,
            &fill.paint,
            fill.opacity,
            set_pattern_fn,
            set_solid_fn,
        );
    }

    fn content_set_stroke_properties(&mut self, bounds: Rect, stroke: &Stroke) {
        // See comment in `set_fill_properties`
        if !matches!(stroke.paint, Paint::Pattern(_)) {
            self.set_stroke_opacity(stroke.opacity);
        }

        fn set_pattern_fn(content: &mut Content, color_space: String) {
            content.set_stroke_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
            content.set_stroke_pattern(None, color_space.to_pdf_name());
        }

        fn set_solid_fn(content: &mut Content, color_space: String, color: &Color) {
            content.set_stroke_color_space(color_space.to_pdf_name());
            content.set_stroke_color(color.to_pdf_components());
        }

        self.content_set_fill_stroke_properties(
            bounds,
            &stroke.paint,
            stroke.opacity,
            set_pattern_fn,
            set_solid_fn,
        );

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
    }

    fn content_draw_path(&mut self, path_data: impl Iterator<Item = PathSegment>) {
        // Taken from resvg
        fn calc(n1: f32, n2: f32) -> f32 {
            (n1 + n2 * 2.0) / 3.0
        }

        let mut p_prev = None;

        for operation in path_data {
            match operation {
                PathSegment::MoveTo(p) => {
                    self.content.move_to(p.x, p.y);
                    p_prev = Some(p);
                }
                PathSegment::LineTo(p) => {
                    self.content.line_to(p.x, p.y);
                    p_prev = Some(p);
                }
                PathSegment::QuadTo(p1, p2) => {
                    // Since PDF doesn't support quad curves, we need to convert them into
                    // cubic.
                    let prev = p_prev.unwrap();
                    self.content.cubic_to(
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
                    self.content.cubic_to(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
                    p_prev = Some(p3);
                }
                PathSegment::Close => {
                    self.content.close_path();
                }
            };
        }
    }
}