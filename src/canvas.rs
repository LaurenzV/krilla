use crate::blend_mode::BlendMode;
use crate::bytecode::{into_composited, ByteCode, Instruction};
use crate::color::PdfColorExt;
use crate::graphics_state::GraphicsStates;
use crate::object::ext_g_state::ExtGState;
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_function::{GradientProperties, GradientPropertiesExt, ShadingFunction};
use crate::object::shading_pattern::ShadingPattern;
use crate::object::tiling_pattern::TilingPattern;
use crate::object::xobject::XObject;
use crate::paint::Paint;
use crate::resource::{PatternResource, Resource, ResourceDictionary, XObjectResource};
use crate::serialize::{PageSerialize, SerializeSettings, SerializerContext};
use crate::transform::TransformWrapper;
use crate::util::{
    calculate_stroke_bbox, deflate, LineCapExt, LineJoinExt, NameExt, RectExt, TransformExt,
};
use crate::MaskType::Luminosity;
use crate::{Color, Fill, FillRule, LineCap, LineJoin, PathWrapper, Stroke};
use pdf_writer::{Chunk, Content, Filter, Finish, Pdf};
use std::sync::Arc;
use tiny_skia_path::{NormalizedF32, Path, PathSegment, Rect, Size, Transform};

pub trait Surface {
    fn masked(&mut self, mask: Mask) -> Masked;
    fn stroke_path(&mut self, path: Path, transform: tiny_skia_path::Transform, stroke: Stroke);
    fn fill_path(&mut self, path: Path, transform: tiny_skia_path::Transform, fill: Fill);
    fn blended(&mut self, blend_mode: BlendMode) -> Blended;
    fn opacified(&mut self, opacity: NormalizedF32) -> Opacified;
    fn clipped_many(&mut self, paths: Vec<(Path, FillRule)>) -> Clipped;
    fn clipped(&mut self, path: Path, clip_rule: FillRule) -> Clipped;
    fn isolated(&mut self) -> Isolated;
    fn transformed(&mut self, transform: Transform) -> Transformed;
    fn draw_image(&mut self, image: Image, size: Size, transform: Transform);
    fn draw_canvas(&mut self, canvas: Canvas);
}

macro_rules! canvas_impl {
    ($type:ident $(<$($lifetime:tt),+>)?) => {
        impl$(<$($lifetime),+>)? Surface for $type $(<$($lifetime),+>)? {

            fn masked(&mut self, mask: Mask) -> Masked {
                Masked::new(&mut self.byte_code, mask)
            }

            fn stroke_path(
                &mut self,
                path: Path,
                transform: tiny_skia_path::Transform,
                stroke: Stroke,
            ) {
                let mut transformed = self.transformed(transform);
                transformed.byte_code.push_stroke_path(path.into(), stroke);
            }

            fn fill_path(&mut self, path: Path, transform: tiny_skia_path::Transform, fill: Fill) {
                let mut transformed = self.transformed(transform);
                transformed.byte_code.push_fill_path(path.into(), fill);
            }

            fn blended(&mut self, blend_mode: BlendMode) -> Blended {
                Blended::new(&mut self.byte_code, blend_mode)
            }

            fn opacified(&mut self, opacity: NormalizedF32) -> Opacified {
                Opacified::new(&mut self.byte_code, opacity)
            }

            fn clipped_many(&mut self, paths: Vec<(Path, FillRule)>) -> Clipped {
                Clipped::new(&mut self.byte_code, paths)
            }

            fn clipped(&mut self, path: Path, clip_rule: FillRule) -> Clipped {
                Clipped::new(&mut self.byte_code, vec![(path, clip_rule)])
            }

            fn isolated(&mut self) -> Isolated {
                Isolated::new(&mut self.byte_code)
            }

            fn transformed(&mut self, transform: Transform) -> Transformed {
                Transformed::new(&mut self.byte_code, transform)
            }

            fn draw_image(&mut self, image: Image, size: Size, transform: Transform) {
                let mut transformed = self.transformed(transform);
                transformed.byte_code.push_image(image, size);
            }

            fn draw_canvas(&mut self, canvas: Canvas) {
                self.byte_code.extend(&canvas.byte_code);
            }
        }
    };
}

// Apply the macro for Canvas and Test
canvas_impl!(Canvas);
canvas_impl!(Masked<'a>);
canvas_impl!(Blended<'a>);
canvas_impl!(Clipped<'a>);
canvas_impl!(Transformed<'a>);
canvas_impl!(Opacified<'a>);
canvas_impl!(Isolated<'a>);

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Masked<'a> {
    parent_byte_code: &'a mut ByteCode,
    pub(crate) byte_code: ByteCode,
    mask: Mask,
}

impl<'a> Masked<'a> {
    pub(crate) fn new(parent_byte_code: &'a mut ByteCode, mask: Mask) -> Self {
        Self {
            parent_byte_code,
            byte_code: ByteCode::new(),
            mask,
        }
    }

    pub fn finish(self) {
        drop(self);
    }
}

impl Drop for Masked<'_> {
    fn drop(&mut self) {
        self.parent_byte_code
            .push_masked(self.mask.clone(), self.byte_code.clone())
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Blended<'a> {
    parent_byte_code: &'a mut ByteCode,
    pub(crate) byte_code: ByteCode,
    blend_mode: BlendMode,
}

impl<'a> Blended<'a> {
    pub fn new(parent_byte_code: &'a mut ByteCode, blend_mode: BlendMode) -> Self {
        Self {
            parent_byte_code,
            byte_code: ByteCode::new(),
            blend_mode,
        }
    }

    pub fn finish(self) {
        drop(self);
    }
}

impl Drop for Blended<'_> {
    fn drop(&mut self) {
        // TODO: Make code more efficient
        if self.blend_mode != BlendMode::SourceOver {
            match self.blend_mode {
                BlendMode::Clear => {
                    self.parent_byte_code.clear();
                }
                BlendMode::Source => {
                    self.parent_byte_code.clear();
                    self.parent_byte_code.extend(&self.byte_code);
                }
                BlendMode::Destination => {}
                // These are only best-effort implementations of the composition modes.
                // In particular, they assume that the source/destination are fully opaque.
                // Something better does not seem possible in PDF.
                BlendMode::DestinationOver => {
                    std::mem::swap(self.parent_byte_code, &mut self.byte_code);
                    self.parent_byte_code.extend(&self.byte_code);
                }
                BlendMode::SourceIn => {
                    let mut byte_code = ByteCode::new();
                    std::mem::swap(self.parent_byte_code, &mut byte_code);
                    let mask = Mask::new(Arc::new(into_composited(&byte_code, false)), Luminosity);
                    self.parent_byte_code
                        .push_masked(mask, self.byte_code.clone());
                }
                BlendMode::DestinationIn => {
                    let mut byte_code = ByteCode::new();
                    std::mem::swap(self.parent_byte_code, &mut byte_code);

                    let mask = Mask::new(
                        Arc::new(into_composited(&self.byte_code, false)),
                        Luminosity,
                    );
                    self.parent_byte_code.push_masked(mask, byte_code);
                }
                BlendMode::SourceOut => {
                    let path = self.byte_code.bbox().to_clip_path();

                    let mut temp_code = ByteCode::new();
                    std::mem::swap(self.parent_byte_code, &mut temp_code);

                    let mut mask_code = ByteCode::new();
                    mask_code.push_fill_path(
                        PathWrapper(path),
                        Fill {
                            paint: Paint::Color(Color::white()),
                            ..Fill::default()
                        },
                    );
                    mask_code.extend(&into_composited(&temp_code, true));

                    let mask = Mask::new(Arc::new(mask_code), Luminosity);
                    self.parent_byte_code
                        .push_masked(mask, self.byte_code.clone());
                }
                BlendMode::DestinationOut => {
                    let path = self.parent_byte_code.bbox().to_clip_path();

                    let mut temp_code = ByteCode::new();
                    std::mem::swap(self.parent_byte_code, &mut temp_code);

                    let mut mask_code = ByteCode::new();
                    mask_code.push_fill_path(
                        PathWrapper(path),
                        Fill {
                            paint: Paint::Color(Color::white()),
                            ..Fill::default()
                        },
                    );
                    mask_code.extend(&into_composited(&self.byte_code, true));

                    let mask = Mask::new(Arc::new(mask_code), Luminosity);
                    self.parent_byte_code.push_masked(mask, temp_code);
                }
                BlendMode::SourceAtop => {
                    let mask = Mask::new(
                        Arc::new(into_composited(&self.parent_byte_code.clone(), false)),
                        Luminosity,
                    );

                    self.parent_byte_code
                        .push_masked(mask, self.byte_code.clone());
                }
                BlendMode::DestinationAtop => {
                    let mask = Mask::new(
                        Arc::new(into_composited(&self.byte_code.clone(), false)),
                        Luminosity,
                    );

                    let mut byte_code = ByteCode::new();
                    std::mem::swap(self.parent_byte_code, &mut byte_code);

                    self.parent_byte_code.extend(&self.byte_code.clone());
                    self.parent_byte_code.push_masked(mask, byte_code)
                }
                BlendMode::Xor => {
                    let mask = Mask::new(
                        Arc::new(into_composited(&self.parent_byte_code.clone(), false)),
                        Luminosity,
                    );

                    let mut overlapped_byte_code = ByteCode::new();
                    overlapped_byte_code.push_masked(mask, into_composited(&self.byte_code, true));

                    let mut bbox = self.parent_byte_code.bbox();
                    bbox.expand(&self.byte_code.bbox());

                    let mut mask_code = ByteCode::new();
                    mask_code.push_fill_path(
                        PathWrapper(bbox.to_clip_path()),
                        Fill {
                            paint: Paint::Color(Color::white()),
                            ..Fill::default()
                        },
                    );
                    mask_code.extend(&overlapped_byte_code);
                    let mask = Mask::new(Arc::new(mask_code), Luminosity);

                    let mut combined = std::mem::replace(self.parent_byte_code, ByteCode::new());
                    combined.extend(&self.byte_code);

                    self.parent_byte_code.push_masked(mask, combined);
                }
                BlendMode::Plus => {}
                // All other blend modes will be translate into their respective PDF blend mode.
                _ => self
                    .parent_byte_code
                    .push_blended(self.blend_mode, self.byte_code.clone()),
            }
        } else {
            self.parent_byte_code.extend(&self.byte_code)
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Opacified<'a> {
    parent_byte_code: &'a mut ByteCode,
    pub(crate) byte_code: ByteCode,
    opacity: NormalizedF32,
}

impl<'a> Opacified<'a> {
    pub fn new(parent_byte_code: &'a mut ByteCode, opacity: NormalizedF32) -> Self {
        Self {
            parent_byte_code,
            byte_code: ByteCode::new(),
            opacity,
        }
    }

    pub fn finish(self) {
        drop(self);
    }
}

impl Drop for Opacified<'_> {
    fn drop(&mut self) {
        if self.opacity != NormalizedF32::ONE {
            self.parent_byte_code
                .push_opacified(self.opacity, self.byte_code.clone())
        } else {
            self.parent_byte_code.extend(&self.byte_code)
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Clipped<'a> {
    parent_byte_code: &'a mut ByteCode,
    pub(crate) byte_code: ByteCode,
    paths: Vec<(PathWrapper, FillRule)>,
}

impl<'a> Clipped<'a> {
    pub fn new(parent_byte_code: &'a mut ByteCode, paths: Vec<(Path, FillRule)>) -> Self {
        Self {
            parent_byte_code,
            paths: paths
                .into_iter()
                .map(|p| (PathWrapper(p.0), p.1))
                .collect::<Vec<_>>(),
            byte_code: ByteCode::new(),
        }
    }

    pub fn finish(self) {
        drop(self);
    }
}

impl Drop for Clipped<'_> {
    fn drop(&mut self) {
        self.parent_byte_code
            .push_clipped(self.paths.clone(), self.byte_code.clone());
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Transformed<'a> {
    parent_byte_code: &'a mut ByteCode,
    pub(crate) byte_code: ByteCode,
    transform: TransformWrapper,
}

impl<'a> Transformed<'a> {
    pub(crate) fn new(parent_byte_code: &'a mut ByteCode, transform: Transform) -> Self {
        Self {
            parent_byte_code,
            byte_code: ByteCode::new(),
            transform: TransformWrapper(transform),
        }
    }

    pub fn finish(self) {
        drop(self);
    }
}

impl Drop for Transformed<'_> {
    fn drop(&mut self) {
        if self.transform.0 != Transform::identity() {
            self.parent_byte_code
                .push_transformed(self.transform, self.byte_code.clone())
        } else {
            self.parent_byte_code.extend(&self.byte_code);
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Isolated<'a> {
    parent_byte_code: &'a mut ByteCode,
    byte_code: ByteCode,
}

impl<'a> Isolated<'a> {
    pub(crate) fn new(parent_byte_code: &'a mut ByteCode) -> Self {
        Self {
            parent_byte_code,
            byte_code: ByteCode::new(),
        }
    }

    pub fn finish(self) {
        drop(self);
    }
}

impl Drop for Isolated<'_> {
    fn drop(&mut self) {
        self.parent_byte_code.push_isolated(self.byte_code.clone())
    }
}

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
}

pub struct CanvasPdfSerializer<'a> {
    resource_dictionary: &'a mut ResourceDictionary,
    content: Content,
    graphics_states: GraphicsStates,
    bbox: Rect,
}

impl<'a> CanvasPdfSerializer<'a> {
    pub fn new(resource_dictionary: &'a mut ResourceDictionary) -> Self {
        Self {
            resource_dictionary,
            content: Content::new(),
            graphics_states: GraphicsStates::new(),
            bbox: Rect::from_xywh(0.0, 0.0, 0.0, 0.0).unwrap(),
        }
    }

    pub fn new_with(resource_dictionary: &'a mut ResourceDictionary, content: Content) -> Self {
        Self {
            resource_dictionary,
            content,
            graphics_states: GraphicsStates::new(),
            bbox: Rect::from_xywh(0.0, 0.0, 0.0, 0.0).unwrap(),
        }
    }

    pub fn serialize_bytecode(&mut self, bytecode: &ByteCode) {
        self.bbox.expand(&bytecode.bbox());

        for op in bytecode.instructions() {
            match op {
                Instruction::StrokePath(stroke_data) => {
                    self.stroke_path(&stroke_data.0 .0, &stroke_data.1)
                }
                Instruction::FillPath(fill_data) => {
                    self.fill_path(&fill_data.0 .0, &fill_data.1);
                }
                Instruction::DrawImage(image_data) => {
                    self.draw_image(image_data.0.clone(), image_data.1)
                }
                Instruction::Blended(blend_data) => self.draw_blended(blend_data.0, &blend_data.1),
                Instruction::Transformed(transform_data) => {
                    self.draw_transformed(transform_data.0 .0, &transform_data.1)
                }
                Instruction::Masked(mask_data) => {
                    self.draw_masked(mask_data.0.clone(), &mask_data.1)
                }
                Instruction::DrawShade(shade_data) => self.draw_shading(&shade_data),
                Instruction::Clipped(clip_data) => {
                    self.draw_clipped(clip_data.0.as_slice(), &clip_data.1)
                }
                Instruction::Opacified(opacity_data) => {
                    self.draw_opacified(opacity_data.0, opacity_data.1.clone())
                }
                Instruction::Isolated(isolated_data) => {
                    self.draw_isolated(isolated_data.clone());
                }
                Instruction::DrawGlyph(_) => todo!(),
            }
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
            let state = ExtGState::new().non_stroking_alpha(alpha);
            self.graphics_states.combine(&state);

            let ext = self
                .resource_dictionary
                .register_resource(Resource::ExtGState(state));
            self.content.set_parameters(ext.to_pdf_name());
        }
    }

    pub fn set_stroke_opacity(&mut self, alpha: NormalizedF32) {
        if alpha.get() != 1.0 {
            let state = ExtGState::new().stroking_alpha(alpha);
            self.graphics_states.combine(&state);

            let ext = self
                .resource_dictionary
                .register_resource(Resource::ExtGState(state));
            self.content.set_parameters(ext.to_pdf_name());
        }
    }

    pub fn set_pdf_blend_mode(&mut self, blend_mode: pdf_writer::types::BlendMode) {
        if blend_mode != pdf_writer::types::BlendMode::Normal {
            let state = ExtGState::new().blend_mode(blend_mode);
            self.graphics_states.combine(&state);

            let ext = self
                .resource_dictionary
                .register_resource(Resource::ExtGState(state));
            self.content.set_parameters(ext.to_pdf_name());
        }
    }

    pub fn fill_path(&mut self, path: &Path, fill: &Fill) {
        if path.bounds().width() == 0.0 || path.bounds().height() == 0.0 {
            return;
        }

        self.save_state();

        // PDF viewers don't show patterns with fill/stroke opacities consistently.
        // Because of this, the opacity is accounted for in the pattern itself.
        if !matches!(fill.paint, Paint::Pattern(_)) {
            self.set_fill_opacity(fill.opacity);
        }

        let pattern_transform = |transform: TransformWrapper| -> TransformWrapper {
            TransformWrapper(
                transform
                    .0
                    .post_concat(self.graphics_states.cur().transform()),
            )
        };

        let mut write_gradient = |gradient_props: GradientProperties,
                                  transform: TransformWrapper| {
            let shading_mask =
                Mask::new_from_shading(gradient_props.clone(), transform, path.bounds());

            let shading_pattern = ShadingPattern::new(
                gradient_props,
                TransformWrapper(
                    self.graphics_states
                        .cur()
                        .transform()
                        .pre_concat(transform.0),
                ),
            );
            let color_space = self
                .resource_dictionary
                .register_resource(Resource::Pattern(PatternResource::ShadingPattern(
                    shading_pattern,
                )));

            if let Some(shading_mask) = shading_mask {
                // TODO: use set_mask instead?
                let state = ExtGState::new().mask(shading_mask);

                let ext = self
                    .resource_dictionary
                    .register_resource(Resource::ExtGState(state));
                self.content.set_parameters(ext.to_pdf_name());
            }

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
            Paint::SweepGradient(sg) => {
                let (gradient_props, transform) = sg.gradient_properties(path.bounds());
                write_gradient(gradient_props, transform);
            }
            Paint::Pattern(pat) => {
                let mut pat = pat.clone();
                let transform = pat.transform;

                Arc::make_mut(&mut pat).transform = pattern_transform(transform);

                let color_space = self
                    .resource_dictionary
                    .register_resource(Resource::Pattern(PatternResource::TilingPattern(
                        TilingPattern::new(pat.canvas.clone(), pat.transform, fill.opacity),
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

    pub fn stroke_path(&mut self, path: &Path, stroke: &Stroke) {
        if path.bounds().width() == 0.0 && path.bounds().height() == 0.0 {
            return;
        }

        // TODO: can this be removed?
        let stroke_bbox = calculate_stroke_bbox(stroke, path).unwrap();

        self.save_state();

        // See comment in `fill_path`
        if !matches!(stroke.paint, Paint::Pattern(_)) {
            self.set_stroke_opacity(stroke.opacity);
        }

        let pattern_transform = |transform: TransformWrapper| -> TransformWrapper {
            TransformWrapper(
                transform
                    .0
                    .post_concat(self.graphics_states.cur().transform()),
            )
        };

        let mut write_gradient = |gradient_props: GradientProperties,
                                  transform: TransformWrapper| {
            let shading_mask =
                Mask::new_from_shading(gradient_props.clone(), transform, stroke_bbox);

            let shading_pattern = ShadingPattern::new(
                gradient_props,
                TransformWrapper(
                    self.graphics_states
                        .cur()
                        .transform()
                        .pre_concat(transform.0),
                ),
            );
            let color_space = self
                .resource_dictionary
                .register_resource(Resource::Pattern(PatternResource::ShadingPattern(
                    shading_pattern,
                )));

            if let Some(shading_mask) = shading_mask {
                // TODO: use set_mask instead?
                let state = ExtGState::new().mask(shading_mask);

                let ext = self
                    .resource_dictionary
                    .register_resource(Resource::ExtGState(state));
                self.content.set_parameters(ext.to_pdf_name());
            }

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
                let (gradient_props, transform) = lg.gradient_properties(stroke_bbox);
                write_gradient(gradient_props, transform);
            }
            Paint::RadialGradient(rg) => {
                let (gradient_props, transform) = rg.gradient_properties(stroke_bbox);
                write_gradient(gradient_props, transform);
            }
            Paint::SweepGradient(sg) => {
                let (gradient_props, transform) = sg.gradient_properties(stroke_bbox);
                write_gradient(gradient_props, transform);
            }
            Paint::Pattern(pat) => {
                let mut pat = pat.clone();
                let transform = pat.transform;

                Arc::make_mut(&mut pat).transform = pattern_transform(transform);

                let color_space = self
                    .resource_dictionary
                    .register_resource(Resource::Pattern(PatternResource::TilingPattern(
                        TilingPattern::new(pat.canvas.clone(), pat.transform, stroke.opacity),
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

    pub fn draw_blended(&mut self, blend_mode: BlendMode, byte_code: &ByteCode) {
        if let Ok(blend_mode) = blend_mode.try_into() {
            self.save_state();
            // These are the blend modes that are available natively in PDF. All other blend
            // modes have already been resolved during bytecode conversion
            self.set_pdf_blend_mode(blend_mode);
            self.serialize_bytecode(byte_code);
            self.restore_state();
        } else {
            self.serialize_bytecode(byte_code);
        }
    }

    pub fn draw_transformed(&mut self, transform: Transform, byte_code: &ByteCode) {
        self.save_state();
        self.transform(&transform);
        self.serialize_bytecode(byte_code);
        self.restore_state();
    }

    pub fn draw_clipped(&mut self, clip_paths: &[(PathWrapper, FillRule)], byte_code: &ByteCode) {
        self.save_state();
        for (clip_path, fill_rule) in clip_paths {
            self.set_clip_path(&clip_path.0, fill_rule);
        }
        self.serialize_bytecode(byte_code);
        self.restore_state();
    }

    pub fn draw_opacified(&mut self, opacity: NormalizedF32, byte_code: ByteCode) {
        let ext_state = ExtGState::new()
            .stroking_alpha(opacity)
            .non_stroking_alpha(opacity);
        let ext_state_name = self
            .resource_dictionary
            .register_resource(Resource::ExtGState(ext_state));

        let x_object = XObject::new(Arc::new(byte_code), true, false, None);
        let x_object_name = self
            .resource_dictionary
            .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
        self.save_state();
        self.content.set_parameters(ext_state_name.to_pdf_name());
        self.content.x_object(x_object_name.to_pdf_name());
        self.restore_state();
    }

    pub fn draw_shading(&mut self, shading: &ShadingFunction) {
        let sh = self
            .resource_dictionary
            .register_resource(Resource::Shading(shading.clone()));
        self.content.shading(sh.to_pdf_name());
    }

    pub fn draw_masked(&mut self, mask: Mask, byte_code: &ByteCode) {
        self.save_state();
        self.set_mask(mask);
        let x_object = XObject::new(Arc::new(byte_code.clone()), false, true, None);
        let name = self
            .resource_dictionary
            .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
        self.content.x_object(name.to_pdf_name());
        self.restore_state();
    }

    pub fn draw_isolated(&mut self, byte_code: Arc<ByteCode>) {
        let x_object = XObject::new(
            byte_code,
            true,
            self.graphics_states.cur().ext_g_state().has_mask(),
            None,
        );

        let name = self
            .resource_dictionary
            .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
        self.content.x_object(name.to_pdf_name());
    }

    pub fn draw_image(&mut self, image: Image, size: Size) {
        let image_name = self
            .resource_dictionary
            .register_resource(Resource::XObject(XObjectResource::Image(image)));
        // Apply user-supplied transform and scale the image from 1x1 to the actual dimensions.
        let transform =
            Transform::from_row(size.width(), 0.0, 0.0, -size.height(), 0.0, size.height());
        self.save_state();
        self.transform(&transform);
        self.content.x_object(image_name.to_pdf_name());
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
            serializer.transform(&Transform::from_row(
                1.0,
                0.0,
                0.0,
                -1.0,
                0.0,
                self.size.height(),
            ));
            serializer.serialize_bytecode(&self.byte_code);

            serializer.finish()
        };

        if serialize_settings.compress {
            let deflated = deflate(&content_stream);

            let mut stream = chunk.stream(content_ref, &deflated);
            stream.filter(Filter::FlateDecode);
            stream.finish();
        } else {
            chunk.stream(content_ref, &content_stream);
        }

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
    use crate::blend_mode::BlendMode;
    use crate::canvas::{Canvas, Surface};
    use crate::color::Color;
    use crate::object::image::Image;
    use crate::object::mask::{Mask, MaskType};
    use crate::paint::{
        LinearGradient, Paint, Pattern, RadialGradient, SpreadMethod, Stop, SweepGradient,
    };
    use crate::serialize::{PageSerialize, SerializeSettings};
    use crate::transform::TransformWrapper;
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

        let serialize_settings = SerializeSettings::default();

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

        let serialize_settings = SerializeSettings::default();

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

        let serialize_settings = SerializeSettings::default();

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

        let mut blended = canvas.blended(BlendMode::Difference);
        let mut transformed = blended.transformed(Transform::from_translate(100.0, 100.0));
        transformed.fill_path(
            dummy_path(100.0),
            Transform::from_translate(-25.0, -25.0),
            Fill {
                paint: Paint::Color(Color::new_rgb(255, 255, 0)),
                opacity: NormalizedF32::new(1.0).unwrap(),
                ..Fill::default()
            },
        );

        transformed.finish();
        blended.finish();

        let serialize_settings = SerializeSettings::default();

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

        let mut translated = canvas.transformed(Transform::from_translate(100.0, 100.0));
        let mut opacified = translated.opacified(NormalizedF32::new(0.5).unwrap());
        opacified.fill_path(
            dummy_path(100.0),
            Transform::identity(),
            Fill {
                paint: Paint::Color(Color::new_rgb(255, 255, 0)),
                opacity: NormalizedF32::new(0.5).unwrap(),
                ..Fill::default()
            },
        );

        opacified.finish();
        translated.finish();

        let serialize_settings = SerializeSettings::default();

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write("nested_opacity", &finished);
    }

    fn sweep_gradient(spread_method: SpreadMethod, name: &str) {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        canvas.fill_path(
            dummy_path(160.0),
            Transform::from_translate(0.0, 0.0).try_into().unwrap(),
            Fill {
                paint: Paint::SweepGradient(SweepGradient {
                    cx: FiniteF32::new(80.0).unwrap(),
                    cy: FiniteF32::new(80.0).unwrap(),
                    start_angle: FiniteF32::new(0.0).unwrap(),
                    end_angle: FiniteF32::new(90.0).unwrap(),
                    transform: TransformWrapper(
                        // Transform::from_scale(0.5, 0.5),
                        // ), // Transform::from_scale(0.5, 0.5),
                        Transform::from_scale(1.0, -1.0),
                    ),
                    spread_method,
                    stops: vec![
                        Stop {
                            offset: NormalizedF32::new(0.0).unwrap(),
                            color: Color::new_rgb(255, 0, 0),
                            opacity: NormalizedF32::new(0.7).unwrap(),
                        },
                        Stop {
                            offset: NormalizedF32::new(0.4).unwrap(),
                            color: Color::new_rgb(0, 255, 0),
                            opacity: NormalizedF32::ONE,
                        },
                        Stop {
                            offset: NormalizedF32::new(1.0).unwrap(),
                            color: Color::new_rgb(0, 0, 255),
                            opacity: NormalizedF32::new(0.5).unwrap(),
                        },
                    ],
                }),
                opacity: NormalizedF32::ONE,
                ..Fill::default()
            },
        );

        let serialize_settings = SerializeSettings::default();

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write(&format!("sweep_gradient_{}", name), &finished);
    }

    #[test]
    fn sweep_gradient_reflect() {
        sweep_gradient(SpreadMethod::Reflect, "reflect");
    }

    #[test]
    fn sweep_gradient_repeat() {
        sweep_gradient(SpreadMethod::Repeat, "repeat");
    }

    #[test]
    fn sweep_gradient_pad() {
        sweep_gradient(SpreadMethod::Pad, "pad");
    }

    fn linear_gradient(spread_method: SpreadMethod, name: &str) {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        canvas.fill_path(
            dummy_path(160.0),
            Transform::from_translate(0.0, 0.0).try_into().unwrap(),
            Fill {
                paint: Paint::LinearGradient(LinearGradient {
                    x1: FiniteF32::new(0.1 * 160.0).unwrap(),
                    y1: FiniteF32::new(0.6 * 160.0).unwrap(),
                    x2: FiniteF32::new(0.3 * 160.0).unwrap(),
                    y2: FiniteF32::new(0.6 * 160.0).unwrap(),
                    transform: TransformWrapper(
                        Transform::identity(), // Transform::from_scale(0.5, 0.5),
                                               // Transform::from_scale(0.5, 0.5).pre_concat(Transform::from_rotate(45.0)),
                    ), // Transform::from_scale(0.5, 0.5),
                    // Transform::identity()
                    spread_method,
                    stops: vec![
                        Stop {
                            offset: NormalizedF32::new(0.2).unwrap(),
                            color: Color::new_rgb(255, 0, 0),
                            opacity: NormalizedF32::ONE,
                        },
                        Stop {
                            offset: NormalizedF32::new(0.4).unwrap(),
                            color: Color::new_rgb(0, 255, 0),
                            opacity: NormalizedF32::new(0.5).unwrap(),
                        },
                        Stop {
                            offset: NormalizedF32::new(0.8).unwrap(),
                            color: Color::new_rgb(0, 0, 255),
                            opacity: NormalizedF32::ONE,
                        },
                    ],
                }),
                opacity: NormalizedF32::ONE,
                ..Fill::default()
            },
        );

        let serialize_settings = SerializeSettings::default();

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write(&format!("linear_gradient_{}", name), &finished);
    }

    #[test]
    fn linear_gradient_reflect() {
        linear_gradient(SpreadMethod::Reflect, "reflect");
    }

    #[test]
    fn linear_gradient_repeat() {
        linear_gradient(SpreadMethod::Repeat, "repeat");
    }

    #[test]
    fn linear_gradient_pad() {
        linear_gradient(SpreadMethod::Pad, "pad");
    }

    fn radial_gradient(spread_method: SpreadMethod, name: &str) {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        canvas.fill_path(
            dummy_path(160.0),
            Transform::from_translate(0.0, 0.0).try_into().unwrap(),
            Fill {
                paint: Paint::RadialGradient(RadialGradient {
                    cx: FiniteF32::new(80.0).unwrap(),
                    cy: FiniteF32::new(80.0).unwrap(),
                    cr: FiniteF32::new(80.0).unwrap(),
                    fx: FiniteF32::new(80.0).unwrap(),
                    fy: FiniteF32::new(80.0).unwrap(),
                    fr: FiniteF32::new(0.0).unwrap(),
                    transform: TransformWrapper(
                        // Transform::from_scale(0.5, 0.5).pre_concat(Transform::from_rotate(45.0)),
                        // Transform::from_scale(0.5, 0.5),
                        Transform::identity(),
                    ),
                    spread_method,
                    stops: vec![
                        Stop {
                            offset: NormalizedF32::new(0.2).unwrap(),
                            color: Color::new_rgb(255, 0, 0),
                            opacity: NormalizedF32::ONE,
                        },
                        Stop {
                            offset: NormalizedF32::new(0.7).unwrap(),
                            color: Color::new_rgb(0, 0, 255),
                            opacity: NormalizedF32::ONE,
                        },
                    ],
                }),
                opacity: NormalizedF32::ONE,
                ..Fill::default()
            },
        );

        let serialize_settings = SerializeSettings::default();

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write(&format!("radial_gradient_{}", name), &finished);
    }

    #[test]
    fn radial_gradient_reflect() {
        crate::canvas::tests::radial_gradient(SpreadMethod::Reflect, "reflect");
    }

    #[test]
    fn radial_gradient_repeat() {
        crate::canvas::tests::radial_gradient(SpreadMethod::Repeat, "repeat");
    }

    #[test]
    fn radial_gradient_pad() {
        crate::canvas::tests::radial_gradient(SpreadMethod::Pad, "pad");
    }

    #[test]
    fn clip_path() {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());

        let mut clipped = canvas.clipped(dummy_path(100.0), FillRule::NonZero);

        clipped.fill_path(
            dummy_path(200.0),
            Transform::from_scale(1.0, 1.0),
            Fill {
                paint: Paint::Color(Color::new_rgb(200, 0, 0)),
                opacity: NormalizedF32::new(0.25).unwrap(),
                ..Fill::default()
            },
        );

        clipped.finish();

        let serialize_settings = SerializeSettings::default();

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

        let serialize_settings = SerializeSettings::default();

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

        let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
        let mut masked = canvas.masked(Mask::new(
            Arc::new(mask_canvas.byte_code),
            MaskType::Luminosity,
        ));
        masked.fill_path(
            dummy_path(200.0),
            Transform::identity().try_into().unwrap(),
            Fill {
                paint: Paint::Color(Color::new_rgb(255, 0, 0)),
                opacity: NormalizedF32::ONE,
                ..Fill::default()
            },
        );

        masked.finish();

        let serialize_settings = SerializeSettings::default();

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
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

        let serialize_settings = SerializeSettings::default();

        let chunk = PageSerialize::serialize(canvas, serialize_settings);
        let finished = chunk.finish();

        write("png_image", &finished);
    }

    fn write(name: &str, data: &[u8]) {
        let _ = std::fs::write(format!("out/{name}.txt"), &data);
        let _ = std::fs::write(format!("out/{name}.pdf"), &data);
    }
}
