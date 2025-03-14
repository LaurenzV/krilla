//! A low-level abstraction over a single content stream.

use std::cell::{RefCell, RefMut};
use std::collections::HashSet;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use float_cmp::approx_eq;
use pdf_writer::types::TextRenderingMode;
use pdf_writer::{Content, Finish, Name, Str, TextStr};
use tiny_skia_path::{Path, PathSegment};

use crate::color::{Color, ColorSpace};
use crate::configure::ValidationError;
use crate::font::{Font, Glyph, GlyphId};
#[cfg(feature = "raster-images")]
use crate::geom::Size;
use crate::graphics_state::GraphicsStates;
#[cfg(feature = "raster-images")]
use crate::image::Image;
use crate::mask::Mask;
use crate::object::ext_g_state::ExtGState;
use crate::object::font::cid_font::CIDFont;
use crate::object::font::type3_font::{CoveredGlyph, Type3Font};
use crate::object::font::{FontContainer, FontIdentifier, PDFGlyph, PaintMode, PDF_UNITS_PER_EM};
use crate::object::shading_function::{GradientProperties, GradientPropertiesExt, ShadingFunction};
use crate::object::shading_pattern::ShadingPattern;
use crate::object::tiling_pattern::TilingPattern;
use crate::object::xobject::XObject;
use crate::paint::{InnerPaint, Paint};
use crate::path::{Fill, FillRule, LineCap, LineJoin, Stroke};
use crate::resource::{Resource, ResourceDictionaryBuilder};
use crate::serialize::{MaybeDeviceColorSpace, SerializeContext};
use crate::stream::Stream;
use crate::surface::Location;
use crate::tagging::ContentTag;
use crate::util::{calculate_stroke_bbox, LineCapExt, LineJoinExt, NameExt, RectExt, TransformExt};
use crate::{resource, NormalizedF32, Point, Rect, Transform};

pub(crate) struct ContentBuilder {
    rd_builder: ResourceDictionaryBuilder,
    content: Content,
    validation_errors: HashSet<ValidationError>,
    root_transform: Transform,
    graphics_states: GraphicsStates,
    bbox: Option<Rect>,
    pub(crate) active_marked_content: bool,
}

/// Stores either a device-specific color space,
/// or the name of a different colorspace (e.g. ICCBased) stored in
/// the current resource dictionary
enum ContentColorSpace {
    Device,
    Named(String),
}

impl ContentBuilder {
    pub(crate) fn new(root_transform: Transform) -> Self {
        Self {
            rd_builder: ResourceDictionaryBuilder::new(),
            validation_errors: HashSet::new(),
            content: Content::new(),
            root_transform,
            graphics_states: GraphicsStates::new(),
            bbox: None,
            active_marked_content: false,
        }
    }

    pub(crate) fn content_save_state(&mut self) {
        self.content.save_state();

        if self.content.state_nesting_depth() > 28 {
            self.validation_errors
                .insert(ValidationError::TooHighQNestingLevel);
        }
    }

    pub(crate) fn finish(self, sc: &mut SerializeContext) -> Stream {
        let buf = self.content.finish();
        sc.register_limits(buf.limits());

        Stream::new(
            buf.to_bytes(),
            self.bbox
                .unwrap_or(Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap()),
            self.validation_errors.into_iter().collect(),
            self.rd_builder.finish(),
        )
    }

    fn start_marked_content_prelude(&mut self) {
        if self.active_marked_content {
            panic!("can't start marked content twice");
        }

        self.active_marked_content = true;
    }

    pub(crate) fn start_marked_content(&mut self, name: Name) {
        self.start_marked_content_prelude();
        self.content.begin_marked_content(name);
    }

    pub(crate) fn start_marked_content_with_properties(
        &mut self,
        sc: &mut SerializeContext,
        mcid: Option<i32>,
        tag: ContentTag,
    ) {
        self.start_marked_content_prelude();

        let mut mc = self
            .content
            .begin_marked_content_with_properties(tag.name());
        let mut properties = mc.properties();

        if let Some(mcid) = mcid {
            properties.pairs([(Name(b"MCID"), mcid)]);
        }

        tag.write_properties(sc, properties);
    }

    pub(crate) fn end_marked_content(&mut self) {
        if !self.active_marked_content {
            panic!("can't end marked content when none has been started");
        }

        self.content.end_marked_content();
        self.active_marked_content = false;
    }

    pub(crate) fn concat_transform(&mut self, transform: &Transform) {
        self.graphics_states.transform(*transform);
    }

    fn cur_transform_with_root_transform(&self) -> Transform {
        self.root_transform.pre_concat(self.cur_transform())
    }

    pub(crate) fn cur_transform(&self) -> Transform {
        self.graphics_states.cur().transform()
    }

    pub(crate) fn save_graphics_state(&mut self) {
        self.graphics_states.save_state();
    }

    pub(crate) fn restore_graphics_state(&mut self) {
        self.graphics_states.restore_state();
    }

    pub(crate) fn set_blend_mode(&mut self, blend_mode: pdf_writer::types::BlendMode) {
        if blend_mode != pdf_writer::types::BlendMode::Normal {
            let state = ExtGState::new().blend_mode(blend_mode);
            self.graphics_states.combine(&state);
        }
    }

    pub(crate) fn expand_bbox(&mut self, new_bbox: Rect) {
        let new_bbox = self.graphics_states.transform_bbox(new_bbox);
        if let Some(bbox) = &mut self.bbox {
            bbox.expand(&new_bbox);
        } else {
            self.bbox = Some(new_bbox);
        }
    }

    pub(crate) fn fill_path(&mut self, path: &Path, fill: Fill, sc: &mut SerializeContext) {
        if path.bounds().width() == 0.0 || path.bounds().height() == 0.0 {
            return;
        }

        let has_pattern = matches!(fill.paint.0, InnerPaint::Pattern(_));
        let fill_opacity = fill.opacity;

        self.apply_isolated_op(
            |sb, _| {
                sb.expand_bbox(Rect::from_tsp(path.bounds()));

                // PDF viewers don't show patterns with fill/stroke opacities consistently.
                // Because of this, the opacity is accounted for in the pattern itself.
                if !has_pattern {
                    sb.set_fill_opacity(fill_opacity);
                }
            },
            |sb, sc| {
                let fill_rule = fill.rule;
                sb.content_set_fill_properties(Rect::from_tsp(path.bounds()), &fill, sc);
                sb.content_draw_path(path.segments());

                match fill_rule {
                    FillRule::NonZero => sb.content.fill_nonzero(),
                    FillRule::EvenOdd => sb.content.fill_even_odd(),
                };
            },
            sc,
        );
    }

    pub(crate) fn stroke_path(&mut self, path: &Path, stroke: Stroke, sc: &mut SerializeContext) {
        if path.bounds().width() == 0.0 && path.bounds().height() == 0.0 {
            return;
        }

        let stroke_bbox =
            calculate_stroke_bbox(&stroke, path).unwrap_or(Rect::from_tsp(path.bounds()));

        let is_pattern = matches!(stroke.paint.0, InnerPaint::Pattern(_));
        let stroke_opacity = stroke.opacity;

        self.apply_isolated_op(
            |sb, _| {
                sb.expand_bbox(stroke_bbox);

                // See comment in `set_fill_properties`
                if !is_pattern {
                    sb.set_stroke_opacity(stroke_opacity);
                }
            },
            |sb, sc| {
                sb.content_set_stroke_properties(stroke_bbox, stroke, sc);
                sb.content_draw_path(path.segments());
                sb.content.stroke();
            },
            sc,
        );
    }

    pub(crate) fn push_clip_path(&mut self, path: &Path, clip_rule: &FillRule) {
        self.content_save_state();
        self.content_draw_path(
            path.clone()
                .transform(self.cur_transform_with_root_transform().to_tsp())
                .unwrap()
                .segments(),
        );

        match clip_rule {
            FillRule::NonZero => self.content.clip_nonzero(),
            FillRule::EvenOdd => self.content.clip_even_odd(),
        };

        self.content.end_path();
    }

    pub(crate) fn pop_clip_path(&mut self) {
        self.content.restore_state();
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fill_glyphs(
        &mut self,
        start: Point,
        sc: &mut SerializeContext,
        fill: Fill,
        glyphs: &[impl Glyph],
        font: Font,
        text: &str,
        font_size: f32,
    ) {
        let (x, y) = (start.x, start.y);
        self.graphics_states.save_state();

        // PDF viewers don't show patterns with fill/stroke opacities consistently.
        // Because of this, the opacity is accounted for in the pattern itself.
        if !matches!(&fill.paint.0, &InnerPaint::Pattern(_)) {
            self.set_fill_opacity(fill.opacity);
        }

        self.fill_stroke_glyph_run(
            x,
            y,
            sc,
            TextRenderingMode::Fill,
            |sb, sc| {
                let bbox = get_glyphs_bbox(glyphs, x, y, font_size, font.clone());
                sb.expand_bbox(bbox);
                sb.content_set_fill_properties(bbox, &fill, sc)
            },
            glyphs,
            font.clone(),
            PaintMode::Fill(&fill),
            text,
            font_size,
        );

        self.graphics_states.restore_state();
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn stroke_glyphs(
        &mut self,
        start: Point,
        sc: &mut SerializeContext,
        stroke: Stroke,
        glyphs: &[impl Glyph],
        font: Font,
        text: &str,
        font_size: f32,
    ) {
        let (x, y) = (start.x, start.y);
        self.graphics_states.save_state();

        // PDF viewers don't show patterns with fill/stroke opacities consistently.
        // Because of this, the opacity is accounted for in the pattern itself.
        if !matches!(&stroke.paint.0, &InnerPaint::Pattern(_)) {
            self.set_stroke_opacity(stroke.opacity);

            // See the comment below regarding why we also set the fill opacity.
            self.set_fill_opacity(stroke.opacity);
        }

        self.fill_stroke_glyph_run(
            x,
            y,
            sc,
            TextRenderingMode::Stroke,
            |sb, sc| {
                // TODO: Bbox should also account for stroke.
                let bbox = get_glyphs_bbox(glyphs, x, y, font_size, font.clone());
                sb.expand_bbox(bbox);
                sb.content_set_stroke_properties(bbox, stroke.clone(), sc);

                // If we are rendering a Type3 glyph, then we actually need to set the fill
                // color instead of the stroke color, because the Type3 glyphs contains the
                // outlined stroke that needs to be filled. Because of this, we simply set both,
                // fill and stroke color, when stroking some text.
                sb.content_set_fill_properties(
                    bbox,
                    &Fill {
                        paint: stroke.paint.clone(),
                        opacity: stroke.opacity,
                        rule: Default::default(),
                    },
                    sc,
                )
            },
            glyphs,
            font.clone(),
            PaintMode::Stroke(&stroke),
            text,
            font_size,
        );

        self.graphics_states.restore_state();
    }

    /// Encode a successive sequence of glyphs that share the same properties and
    /// can be encoded with one text showing operator.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    fn encode_consecutive_glyph_run(
        &mut self,
        sc: &mut SerializeContext,
        cur_x: &mut f32,
        cur_y: f32,
        font_identifier: FontIdentifier,
        pdf_font: &dyn PdfFont,
        size: f32,
        paint_mode: PaintMode,
        glyphs: &[impl Glyph],
        text: &str,
    ) {
        let font_name = self
            .rd_builder
            .register_resource(sc.register_font_identifier(font_identifier));
        self.content.set_font(font_name.to_pdf_name(), size);
        self.content.set_text_matrix(
            Transform::from_row(1.0, 0.0, 0.0, -1.0, *cur_x, cur_y).to_pdf_transform(),
        );

        let mut positioned = self.content.show_positioned();
        let mut items = positioned.items();

        let mut adjustment = 0.0;
        let mut encoded = vec![];

        for glyph in glyphs {
            match glyph.location() {
                None => sc.reset_location(),
                Some(l) => sc.set_location(l),
            };

            if glyph.glyph_id() == GlyphId::new(0)
                || pdf_font.font().postscript_name() == Some("LastResort")
            {
                sc.register_validation_error(ValidationError::ContainsNotDefGlyph(
                    pdf_font.font(),
                    sc.location,
                    text[glyph.text_range()].to_string(),
                ));
            }

            let pdf_glyph = pdf_font
                .get_gid(CoveredGlyph::new(glyph.glyph_id(), paint_mode))
                .unwrap();

            let scale = |val| val * pdf_font.units_per_em();

            let x_advance = scale(glyph.x_advance(1.0));
            let font_advance = pdf_font
                .font()
                .advance_width(glyph.glyph_id())
                .map(|n| scale(n / pdf_font.font().units_per_em()));
            let x_offset = scale(glyph.x_offset(1.0));

            adjustment += x_offset;

            // Make sure we don't write miniscule adjustments
            if !approx_eq!(f32, adjustment, 0.0, epsilon = 0.001) {
                if !encoded.is_empty() {
                    items.show(Str(&encoded));
                    encoded.clear();
                }

                // Adjustment is always in 1000 units, even for Type3 fonts.
                items.adjust(-(adjustment / pdf_font.units_per_em() * PDF_UNITS_PER_EM));
                adjustment = 0.0;
            }

            pdf_glyph.encode_into(&mut encoded);

            if let Some(font_advance) = font_advance {
                adjustment += x_advance - font_advance;
            }

            adjustment -= x_offset;
            // cur_x/cur_y and glyph metrics are in user space units.
            *cur_x += glyph.x_advance(size);

            sc.reset_location();
        }

        if !encoded.is_empty() {
            items.show(Str(&encoded));
        }

        items.finish();
        positioned.finish();
    }

    #[allow(clippy::too_many_arguments)]
    fn fill_stroke_glyph_run(
        &mut self,
        x: f32,
        ys: f32,
        sc: &mut SerializeContext,
        fill_render_mode: TextRenderingMode,
        action: impl FnOnce(&mut ContentBuilder, &mut SerializeContext),
        glyphs: &[impl Glyph],
        font: Font,
        paint_mode: PaintMode,
        text: &str,
        font_size: f32,
    ) {
        self.apply_isolated_op(
            |_, _| {},
            |sb, sc| {
                let mut cur_x = x;
                let mut cur_y = ys;

                action(sb, sc);
                sb.content.begin_text();

                let font_container = sc.register_font_container(font.clone());

                // Separate into distinct glyph runs that either are encoded using actual text, or are
                // not.
                let spanned = TextSpanner::new(
                    glyphs,
                    text,
                    sc.serialize_settings()
                        .validator()
                        .requires_codepoint_mappings(),
                    paint_mode,
                    font_container.clone(),
                );

                for fragment in spanned {
                    if let Some(text) = fragment.actual_text() {
                        let mut actual_text = sb
                            .content
                            .begin_marked_content_with_properties(Name(b"Span"));
                        actual_text.properties().actual_text(TextStr(text));
                    }

                    // Segment into glyph runs that can be encoded in one go using a PDF
                    // text showing operator (i.e. no y shift, same Type3 font, etc.)
                    let segmented =
                        GlyphGrouper::new(font_container.clone(), paint_mode, fragment.glyphs());

                    for glyph_group in segmented {
                        let borrowed = font_container.borrow();
                        let pdf_font = borrowed
                            .get_from_identifier(glyph_group.font_identifier.clone())
                            .unwrap();

                        if fill_render_mode == TextRenderingMode::Fill || pdf_font.force_fill() {
                            sb.content.set_text_rendering_mode(TextRenderingMode::Fill);
                        } else {
                            sb.content
                                .set_text_rendering_mode(TextRenderingMode::Stroke);
                        }

                        sb.encode_consecutive_glyph_run(
                            sc,
                            &mut cur_x,
                            cur_y - glyph_group.y_offset * font_size,
                            glyph_group.font_identifier,
                            pdf_font,
                            font_size,
                            paint_mode,
                            glyph_group.glyphs,
                            text,
                        );

                        cur_y -= glyph_group.y_advance * font_size;
                    }

                    if fragment.actual_text().is_some() {
                        sb.content.end_marked_content();
                    }
                }

                sb.content.end_text();
            },
            sc,
        )
    }

    pub(crate) fn draw_xobject(
        &mut self,
        sc: &mut SerializeContext,
        x_object: XObject,
        state: &ExtGState,
    ) {
        let bbox = x_object.bbox();
        self.apply_isolated_op(
            |sb, _| {
                sb.graphics_states.combine(state);
                sb.expand_bbox(bbox);
            },
            move |sb, sc| {
                let x_object_name = sb
                    .rd_builder
                    .register_resource(sc.register_resourceable(x_object));
                sb.content.x_object(x_object_name.to_pdf_name());
            },
            sc,
        );
    }

    pub(crate) fn draw_masked(&mut self, sc: &mut SerializeContext, mask: Mask, stream: Stream) {
        let state = ExtGState::new().mask(mask, sc);
        let x_object = XObject::new(stream, false, true, None);
        self.draw_xobject(sc, x_object, &state);
    }

    pub(crate) fn draw_opacified(
        &mut self,
        sc: &mut SerializeContext,
        opacity: NormalizedF32,
        stream: Stream,
    ) {
        let state = ExtGState::new()
            .stroking_alpha(opacity)
            .non_stroking_alpha(opacity);
        let x_object = XObject::new(stream, true, false, None);
        self.draw_xobject(sc, x_object, &state);
    }

    pub(crate) fn draw_isolated(&mut self, sc: &mut SerializeContext, stream: Stream) {
        let state = ExtGState::new();
        let x_object = XObject::new(stream, true, false, None);
        self.draw_xobject(sc, x_object, &state);
    }

    #[cfg(feature = "raster-images")]
    pub(crate) fn draw_image(&mut self, image: Image, size: Size, sc: &mut SerializeContext) {
        self.apply_isolated_op(
            |sb, _| {
                // Scale the image from 1x1 to the actual dimensions.
                let transform =
                    Transform::from_row(size.width(), 0.0, 0.0, -size.height(), 0.0, size.height());
                sb.concat_transform(&transform);
                sb.expand_bbox(Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap());
            },
            move |sb, sc| {
                let image_name = sb
                    .rd_builder
                    .register_resource(resource::XObject::new(sc.register_image(image)));

                sb.content.x_object(image_name.to_pdf_name());
            },
            sc,
        );
    }

    pub(crate) fn draw_shading(&mut self, shading: &ShadingFunction, sc: &mut SerializeContext) {
        self.apply_isolated_op(
            |_, _| {},
            move |sb, sc| {
                let sh = sb
                    .rd_builder
                    .register_resource(sc.register_resourceable(shading.clone()));
                sb.content.shading(sh.to_pdf_name());
            },
            sc,
        )
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

    fn apply_isolated_op(
        &mut self,
        prep: impl FnOnce(&mut Self, &mut SerializeContext),
        op: impl FnOnce(&mut Self, &mut SerializeContext),
        sc: &mut SerializeContext,
    ) {
        self.save_graphics_state();
        self.content_save_state();

        prep(self, sc);

        let transform = self.cur_transform_with_root_transform();

        if transform != Transform::identity() {
            self.content.transform(transform.to_pdf_transform());
        }

        let state = self.graphics_states.cur().ext_g_state().clone();

        if !state.empty() {
            let ext = self
                .rd_builder
                .register_resource::<resource::ExtGState>(sc.register_resourceable(state));
            self.content.set_parameters(ext.to_pdf_name());
        }

        op(self, sc);

        self.content.restore_state();
        self.restore_graphics_state();
    }

    fn content_set_fill_stroke_properties(
        &mut self,
        bounds: Rect,
        paint: &Paint,
        opacity: NormalizedF32,
        sc: &mut SerializeContext,
        mut set_pattern_fn: impl FnMut(&mut Content, String),
        mut set_solid_fn: impl FnMut(&mut Content, ContentColorSpace, Color),
    ) {
        let pattern_transform = |transform: Transform| -> Transform {
            transform.post_concat(self.cur_transform_with_root_transform())
        };

        let mut write_gradient =
            |gradient_props: GradientProperties,
             sc: &mut SerializeContext,
             transform: Transform,
             content_builder: &mut ContentBuilder| {
                if let Some((color, opacity)) = gradient_props.single_stop_color() {
                    // Write gradients with one stop as a solid color fill.
                    content_builder.set_fill_opacity(opacity);
                    let cs = color.color_space(sc);
                    let color_space_resource = Self::cs_to_content_cs(content_builder, sc, cs);
                    set_solid_fn(&mut content_builder.content, color_space_resource, color);
                } else {
                    let shading_mask =
                        Mask::new_from_shading(gradient_props.clone(), transform, bounds, sc);

                    let shading_pattern = ShadingPattern::new(
                        gradient_props,
                        content_builder
                            .cur_transform_with_root_transform()
                            .pre_concat(transform),
                    );
                    let color_space = content_builder
                        .rd_builder
                        .register_resource::<resource::Pattern>(
                            sc.register_resourceable(shading_pattern),
                        );

                    if let Some(shading_mask) = shading_mask {
                        let state = ExtGState::new().mask(shading_mask, sc);

                        let ext = content_builder
                            .rd_builder
                            .register_resource::<resource::ExtGState>(
                                sc.register_resourceable(state),
                            );
                        content_builder.content.set_parameters(ext.to_pdf_name());
                    }

                    set_pattern_fn(&mut content_builder.content, color_space);
                }
            };

        match &paint.0 {
            InnerPaint::Color(c) => {
                let cs = c.color_space(sc);
                let color_space_resource = Self::cs_to_content_cs(self, sc, cs);
                set_solid_fn(&mut self.content, color_space_resource, *c);
            }
            InnerPaint::LinearGradient(lg) => {
                let (gradient_props, transform) = lg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, sc, transform, self);
            }
            InnerPaint::RadialGradient(rg) => {
                let (gradient_props, transform) = rg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, sc, transform, self);
            }
            InnerPaint::SweepGradient(sg) => {
                let (gradient_props, transform) = sg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, sc, transform, self);
            }
            InnerPaint::Pattern(pat) => {
                let mut pat = Arc::unwrap_or_clone(pat.clone());
                pat.transform = pattern_transform(pat.transform);

                let tiling_pattern = TilingPattern::new(
                    pat.stream,
                    pat.transform,
                    opacity,
                    pat.width,
                    pat.height,
                    sc,
                );

                let color_space = self.rd_builder.register_resource::<resource::Pattern>(
                    sc.register_resourceable(tiling_pattern),
                );
                set_pattern_fn(&mut self.content, color_space);
            }
        }
    }

    fn cs_to_content_cs(
        content_builder: &mut ContentBuilder,
        sc: &mut SerializeContext,
        cs: ColorSpace,
    ) -> ContentColorSpace {
        match sc.register_colorspace(cs) {
            MaybeDeviceColorSpace::ColorSpace(s) => {
                ContentColorSpace::Named(content_builder.rd_builder.register_resource(s))
            }
            MaybeDeviceColorSpace::DeviceGray => ContentColorSpace::Device,
            MaybeDeviceColorSpace::DeviceRgb => ContentColorSpace::Device,
            MaybeDeviceColorSpace::DeviceCMYK => ContentColorSpace::Device,
        }
    }

    fn content_set_fill_properties(
        &mut self,
        bounds: Rect,
        fill: &Fill,
        serializer_context: &mut SerializeContext,
    ) {
        fn set_pattern_fn(content: &mut Content, color_space: String) {
            content.set_fill_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
            content.set_fill_pattern(None, color_space.to_pdf_name());
        }

        fn set_solid_fn(content: &mut Content, color_space: ContentColorSpace, color: Color) {
            match color_space {
                ContentColorSpace::Device => match color {
                    Color::Rgb(r) => {
                        let comps = r.to_pdf_color();
                        content.set_fill_rgb(comps[0], comps[1], comps[2]);
                    }
                    Color::Luma(l) => {
                        content.set_fill_gray(l.to_pdf_color());
                    }
                    Color::Cmyk(c) => {
                        let comps = c.to_pdf_color();
                        content.set_fill_cmyk(comps[0], comps[1], comps[2], comps[3]);
                    }
                },
                ContentColorSpace::Named(n) => {
                    content.set_fill_color_space(n.to_pdf_name());
                    content.set_fill_color(color.to_pdf_color());
                }
            }
        }

        self.content_set_fill_stroke_properties(
            bounds,
            &fill.paint,
            fill.opacity,
            serializer_context,
            set_pattern_fn,
            set_solid_fn,
        );
    }

    fn content_set_stroke_properties(
        &mut self,
        bounds: Rect,
        stroke: Stroke,
        serializer_context: &mut SerializeContext,
    ) {
        fn set_pattern_fn(content: &mut Content, color_space: String) {
            content.set_stroke_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
            content.set_stroke_pattern(None, color_space.to_pdf_name());
        }

        fn set_solid_fn(content: &mut Content, color_space: ContentColorSpace, color: Color) {
            match color_space {
                ContentColorSpace::Device => match color {
                    Color::Rgb(r) => {
                        let comps = r.to_pdf_color();
                        content.set_stroke_rgb(comps[0], comps[1], comps[2]);
                    }
                    Color::Luma(l) => {
                        content.set_stroke_gray(l.to_pdf_color());
                    }
                    Color::Cmyk(c) => {
                        let comps = c.to_pdf_color();
                        content.set_stroke_cmyk(comps[0], comps[1], comps[2], comps[3]);
                    }
                },
                ContentColorSpace::Named(n) => {
                    content.set_stroke_color_space(n.to_pdf_name());
                    content.set_stroke_color(color.to_pdf_color());
                }
            }
        }

        self.content_set_fill_stroke_properties(
            bounds,
            &stroke.paint,
            stroke.opacity,
            serializer_context,
            set_pattern_fn,
            set_solid_fn,
        );

        // Only write if they don't correspond to the default values as defined in the
        // PDF specification.
        if stroke.width != 1.0 {
            self.content.set_line_width(stroke.width);
        }

        if stroke.miter_limit != 10.0 {
            self.content.set_miter_limit(stroke.miter_limit);
        }

        if stroke.line_cap != LineCap::Butt {
            self.content.set_line_cap(stroke.line_cap.to_pdf_line_cap());
        }

        if stroke.line_join != LineJoin::Miter {
            self.content
                .set_line_join(stroke.line_join.to_pdf_line_join());
        }

        if let Some(stroke_dash) = &stroke.dash {
            self.content
                .set_dash_pattern(stroke_dash.array.iter().copied(), stroke_dash.offset);
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

// Note that this isn't a 100% accurate calculation, it can overestimate (and in a few cases
// even underestimate), but it should be good enough for the majority of the cases.
// The bbox is mostly needed for automatic size detection and postscript gradient, so it's
// not too critical if it doesn't work in edge cases. What matters most is that the performance
// is good, since this code is on the hot path in text-intensive PDFs.
// TODO: Improve this so that `zalgo_text` test case shows up fully in the reference image.
fn get_glyphs_bbox(glyphs: &[impl Glyph], x: f32, y: f32, size: f32, font: Font) -> Rect {
    let font_bbox = font.bbox();
    let (mut bl, mut bt, mut br, mut bb) = font_bbox
        .transform(Transform::from_scale(
            size / font.units_per_em(),
            -size / font.units_per_em(),
        ))
        .and_then(|b| b.transform(Transform::from_translate(x, y)))
        .map(|b| (b.left(), b.top(), b.right(), b.bottom()))
        .unwrap_or((x, y, x + 1.0, y + 1.0));

    let mut x = x;
    let mut y = y;

    for glyph in glyphs {
        let xo = glyph.x_offset(size);
        let xa = glyph.x_advance(size);
        let yo = glyph.y_offset(size);
        let ya = glyph.y_advance(size);

        x += xa;
        y -= ya;

        bl = bl.min(x + xo);
        br = br.max(x + xo);
        bt = bt.min(y - yo);
        bb = bb.max(y - yo);
    }

    Rect::from_ltrb(bl, bt, br, bb).unwrap()
}

pub(crate) trait PdfFont {
    fn units_per_em(&self) -> f32;
    fn font(&self) -> Font;
    fn get_codepoints(&self, pdf_glyph: PDFGlyph) -> Option<&str>;
    fn set_codepoints(&mut self, pdf_glyph: PDFGlyph, text: String, location: Option<Location>);
    fn get_gid(&self, glyph: CoveredGlyph) -> Option<PDFGlyph>;
    fn force_fill(&self) -> bool;
}

impl PdfFont for Type3Font {
    fn units_per_em(&self) -> f32 {
        self.unit_per_em()
    }

    fn font(&self) -> Font {
        Type3Font::font(self)
    }

    fn get_codepoints(&self, pdf_glyph: PDFGlyph) -> Option<&str> {
        match pdf_glyph {
            PDFGlyph::Type3(t3) => self.get_codepoints(t3),
            PDFGlyph::Cid(_) => panic!("attempted to pass cid to type 3 font"),
        }
    }

    fn set_codepoints(&mut self, pdf_glyph: PDFGlyph, text: String, location: Option<Location>) {
        match pdf_glyph {
            PDFGlyph::Type3(t3) => self.set_codepoints(t3, text, location),
            PDFGlyph::Cid(_) => panic!("attempted to pass cid to type 3 font"),
        }
    }

    fn get_gid(&self, glyph: CoveredGlyph) -> Option<PDFGlyph> {
        self.get_gid(&glyph.to_owned()).map(PDFGlyph::Type3)
    }

    fn force_fill(&self) -> bool {
        true
    }
}

impl PdfFont for CIDFont {
    fn units_per_em(&self) -> f32 {
        self.units_per_em()
    }

    fn font(&self) -> Font {
        CIDFont::font(self)
    }

    fn get_codepoints(&self, pdf_glyph: PDFGlyph) -> Option<&str> {
        match pdf_glyph {
            PDFGlyph::Type3(_) => panic!("attempted to pass type 3 glyph to cid font"),
            PDFGlyph::Cid(cid) => self.get_codepoints(cid),
        }
    }

    fn set_codepoints(&mut self, pdf_glyph: PDFGlyph, text: String, location: Option<Location>) {
        match pdf_glyph {
            PDFGlyph::Type3(_) => panic!("attempted to pass type 3 glyph to cid font"),
            PDFGlyph::Cid(cid) => self.set_codepoints(cid, text, location),
        }
    }

    fn get_gid(&self, glyph: CoveredGlyph) -> Option<PDFGlyph> {
        self.get_cid(glyph.glyph_id).map(PDFGlyph::Cid)
    }

    fn force_fill(&self) -> bool {
        false
    }
}

pub(crate) enum TextSpan<'a, T>
where
    T: Glyph,
{
    Unspanned(&'a [T]),
    Spanned(&'a [T], &'a str),
}

impl<T> TextSpan<'_, T>
where
    T: Glyph,
{
    pub(crate) fn glyphs(&self) -> &[T] {
        match self {
            TextSpan::Unspanned(glyphs) => glyphs,
            TextSpan::Spanned(glyphs, _) => glyphs,
        }
    }

    pub(crate) fn actual_text(&self) -> Option<&str> {
        match self {
            TextSpan::Unspanned(_) => None,
            TextSpan::Spanned(_, text) => Some(text),
        }
    }
}

/// In PDF, correspondences between glyphs and Unicode codepoints are expressed
/// via a CMAP. In a CMAP, you can assign a sequence of Unicode codepoints to each
/// glyph. There are two issues with this approach:
/// - How to deal with the fact that the same glyph might be assigned two different codepoints
///   in different contexts (i.e. space and NZWJ).
/// - How to deal with complex shaping scenarios, where there is not a one-to-one or
///   one-to-many correspondence between glyphs and codepoints, but instead a many-to-one
///   or many-to-many mapping.
///
/// The answer to this is the `ActualText` feature of PDF, which allows to define some custom
/// actual text for a number of glyphs, which overrides anything else. Unfortunately, this
/// is seemingly only supported in Acrobat and Chrome, but it's the only proper way of addressing
/// this issue.
///
/// This is the task of the `TextSpanner`. Given a sequence of glyphs, it segments the
/// sequence into subruns of glyphs that either do need to be wrapped in an actual text
/// attribute, or not.
pub(crate) struct TextSpanner<'a, T>
where
    T: Glyph,
{
    slice: &'a [T],
    paint_mode: PaintMode<'a>,
    forbid_invalid_codepoints: bool,
    font_container: Rc<RefCell<FontContainer>>,
    text: &'a str,
}

impl<'a, T> TextSpanner<'a, T>
where
    T: Glyph,
{
    pub(crate) fn new(
        slice: &'a [T],
        text: &'a str,
        forbid_invalid_codepoints: bool,
        paint_mode: PaintMode<'a>,
        font_container: Rc<RefCell<FontContainer>>,
    ) -> Self {
        Self {
            slice,
            paint_mode,
            forbid_invalid_codepoints,
            text,
            font_container,
        }
    }
}

impl<'a, T> Iterator for TextSpanner<'a, T>
where
    T: Glyph,
{
    type Item = TextSpan<'a, T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        fn func<U>(
            g: &U,
            paint_mode: PaintMode,
            previous_range: Option<Range<usize>>,
            forbid_invalid_codepoints: bool,
            mut font_container: RefMut<FontContainer>,
            text: &str,
        ) -> (Range<usize>, bool)
        where
            U: Glyph,
        {
            let (identifier, pdf_glyph) =
                font_container.add_glyph(CoveredGlyph::new(g.glyph_id(), paint_mode));
            let pdf_font = font_container
                .get_from_identifier_mut(identifier.clone())
                .unwrap();

            let range = g.text_range().clone();
            let text = &text[range.clone()];
            let codepoints = pdf_font.get_codepoints(pdf_glyph);
            // Check if the glyph has already been assigned codepoints that don't match the
            // one we are seeing right now.
            let incompatible_codepoint = codepoints.is_some_and(|text| codepoints != Some(text));

            // Only set the codepoint if there isn't a previous, different mapping.
            //
            // If we could set it, we only want to insert a codepoint if we are not already
            // building a spanned run (which is the case if the previous range is the same).
            // If we are building a spanned run, it means that the glyphs are part of the same
            // cluster, in which case only the first glyph should be assigned the codepoint,
            // while all other glyphs in the same cluster should not be assigned anything.
            // Otherwise, when copying text from the PDF, we will get the same codepoint multiple
            // times in viewers that don't support `ActualText`.
            //
            // However, in case we are for example exporting to PDF/UA, every glyph is required
            // to have a valid codepoint mapping. So in this case, we still add the codepoints
            // to each glyph in the cluster, this will result in worse copy-pasting in viewers
            // that don't support `ActualText`.
            if !incompatible_codepoint
                && (previous_range != Some(range.clone()) || forbid_invalid_codepoints)
            {
                pdf_font.set_codepoints(pdf_glyph, text.to_string(), g.location());
            }

            (range, incompatible_codepoint)
        }

        let mut use_span = None;
        let mut count = 1;

        let mut iter = self.slice.iter();

        // Get the range of the first glyph, as well as whether it's
        // incompatible.
        let (first_range, first_incompatible) = func(
            iter.next()?,
            self.paint_mode,
            None,
            self.forbid_invalid_codepoints,
            self.font_container.borrow_mut(),
            self.text,
        );

        let mut prev_range = first_range.clone();

        for next in iter {
            let (next_range, next_incompatible) = func(
                next,
                self.paint_mode,
                Some(prev_range.clone()),
                self.forbid_invalid_codepoints,
                self.font_container.borrow_mut(),
                self.text,
            );

            match use_span {
                // In this case, we just started and we are looking at the first two glyphs.
                // This decides whether the current run will be spanned, or not.
                None => {
                    if prev_range == next_range {
                        // The two glyphs are in the same range, so we definitely want this run
                        // to be spanned, and also want to include both glyphs in that run.
                        use_span = Some(true);
                    } else {
                        // Else, whether we use a span depends on whether the first glyph
                        // is incompatible.
                        use_span = Some(first_incompatible);

                        // If either the first glyph or the second glyph are incompatible, they
                        // need to be in separate runs, since they are not part of the same cluster.
                        if first_incompatible || next_incompatible {
                            break;
                        }

                        // If none are incompatible, then `use_span` is false, and we can also
                        // include the next glyph in that unspanned run.
                    }
                }
                // We are currently building a spanned range, and all glyphs
                // are part of the same cluster.
                Some(true) => {
                    // If the next glyph is not part of the same cluster, terminate the current
                    // span and don't include the next one.
                    if prev_range != next_range {
                        break;
                    }
                }
                // We are currently building an unspanned range, meaning the
                // glyphs are not part of the same cluster.
                Some(false) => {
                    // If the previous and next glyph are part of the same range this means
                    // that they are part of the same cluster. This means that the previous
                    // AND the next glyph should be part of the upcoming spanned range, not
                    // the current one. To exclude the next glyph, we need to do
                    // `count -= 1` before terminating.
                    if prev_range == next_range {
                        count -= 1;
                        break;
                    }

                    // If the next one is incompatible, terminate the
                    // current run, since the next one needs to be spanned.
                    if next_incompatible {
                        break;
                    }
                }
            }

            prev_range = next.text_range().clone();
            count += 1;
        }

        let (head, tail) = self.slice.split_at(count);
        self.slice = tail;

        let fragment = match use_span.unwrap_or(false) {
            true => TextSpan::Spanned(head, &self.text[first_range]),
            false => TextSpan::Unspanned(head),
        };

        Some(fragment)
    }
}

pub(crate) struct GlyphGroup<'a, T>
where
    T: Glyph,
{
    font_identifier: FontIdentifier,
    glyphs: &'a [T],
    // This will be stored in normalized form.
    y_offset: f32,
    // This will be stored in normalized form.
    y_advance: f32,
}

impl<'a, T> GlyphGroup<'a, T>
where
    T: Glyph,
{
    pub fn new(
        font_identifier: FontIdentifier,
        glyphs: &'a [T],
        y_offset: f32,
        y_advance: f32,
    ) -> Self {
        GlyphGroup {
            font_identifier,
            glyphs,
            y_offset,
            y_advance,
        }
    }
}

// The GlyphGrouper further segments glyph runs (that already have been segmented
// by `TextSpanner` into subruns that can be encoded as one consecutive run in PDF.
// This is necessary because:
// - The user provides a font for the whole glyph run, but in PDF, the font might
// have to be switched if the glyph maps to a different Type3 font.
// - The glyph contains a y_offset/y_advance, which cannot be expressed as an adjustment
// and requires us to start a new run with a transformation matrix that takes this
// adjustment into account.
pub(crate) struct GlyphGrouper<'a, T>
where
    T: Glyph,
{
    font_container: Rc<RefCell<FontContainer>>,
    paint_mode: PaintMode<'a>,
    slice: &'a [T],
}

impl<'a, T> GlyphGrouper<'a, T>
where
    T: Glyph,
{
    pub fn new(
        font_container: Rc<RefCell<FontContainer>>,
        paint_mode: PaintMode<'a>,
        slice: &'a [T],
    ) -> Self {
        Self {
            font_container,
            paint_mode,
            slice,
        }
    }
}

impl<'a, T> Iterator for GlyphGrouper<'a, T>
where
    T: Glyph,
{
    type Item = GlyphGroup<'a, T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Guarantees: All glyphs in `head` have the font identifier that is given in
        // `props`, the same size and the same y offset.
        let (head, tail, props) = {
            struct GlyphProps {
                font_identifier: FontIdentifier,
                y_offset: f32,
                y_advance: f32,
            }

            fn func<U>(
                g: &U,
                paint_mode: PaintMode,
                font_container: RefMut<FontContainer>,
            ) -> GlyphProps
            where
                U: Glyph,
            {
                // Safe because we've already added all glyphs in the text spanner.
                let font_identifier = font_container
                    .font_identifier(CoveredGlyph::new(g.glyph_id(), paint_mode))
                    .unwrap();

                GlyphProps {
                    font_identifier,
                    y_offset: g.y_offset(1.0),
                    y_advance: g.y_advance(1.0),
                }
            }

            let mut count = 1;

            let mut iter = self.slice.iter();
            let first = func(
                iter.next()?,
                self.paint_mode,
                self.font_container.borrow_mut(),
            );

            for next in iter {
                let temp_glyph = func(next, self.paint_mode, self.font_container.borrow_mut());

                // If either of those is different, we need to start a new subrun.
                if first.font_identifier != temp_glyph.font_identifier
                    || first.y_offset != temp_glyph.y_offset
                    || first.y_advance != 0.0
                    || temp_glyph.y_advance != 0.0
                {
                    break;
                }

                count += 1;
            }

            let (head, tail) = self.slice.split_at(count);
            (head, tail, first)
        };

        self.slice = tail;

        let glyph_group =
            GlyphGroup::new(props.font_identifier, head, props.y_offset, props.y_advance);

        Some(glyph_group)
    }
}
