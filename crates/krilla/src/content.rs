//! A low-level abstraction over a single content stream.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use float_cmp::approx_eq;
use pdf_writer::types::TextRenderingMode;
use pdf_writer::{Content, Finish, Name, Str, TextStr};
use tiny_skia_path::{Path, PathSegment, PathVerb};

use crate::color::rgb;
use crate::configure::ValidationError;
#[cfg(feature = "raster-images")]
use crate::geom::Size;
use crate::geom::{Point, Rect, Transform};
use crate::graphics::color::{Color, ColorSpace};
use crate::graphics::graphics_state::{ExtGState, GraphicsStates};
#[cfg(feature = "raster-images")]
use crate::graphics::image::Image;
use crate::graphics::mask::Mask;
use crate::graphics::paint::{Fill, FillRule, InnerPaint, LineCap, LineJoin, Paint, Stroke};
use crate::graphics::shading_function::{
    GradientProperties, GradientPropertiesExt, ShadingFunction,
};
use crate::graphics::shading_pattern::ShadingPattern;
use crate::graphics::tiling_pattern::TilingPattern;
use crate::graphics::xobject::XObject;
use crate::interchange::tagging::ContentTag;
use crate::num::NormalizedF32;
use crate::resource;
use crate::resource::{Resource, ResourceDictionaryBuilder};
use crate::serialize::{MaybeDeviceColorSpace, SerializeContext};
use crate::stream::Stream;
use crate::text::group::{use_text_spanner, GlyphGroup, GlyphGrouper, GlyphSpan, GlyphSpanner};
use crate::text::type3::ColoredGlyph;
use crate::text::{Font, FontContainer, FontIdentifier, PdfFont, PDF_UNITS_PER_EM};
use crate::text::{Glyph, GlyphId};
use crate::util::{calculate_stroke_bbox, NameExt};

pub(crate) struct ContentBuilder {
    rd_builder: ResourceDictionaryBuilder,
    content: Content,
    validation_errors: HashSet<ValidationError>,
    root_transform: Transform,
    graphics_states: GraphicsStates,
    bbox: Option<Rect>,
    // Calculating the bbox of text is expensive, so we should avoid doing it if not needed.
    // The only time we really need it is if we are currently inside of an XObject, where we
    // need to provide a bbox of all its contents. If we are on the main page stream, we only
    // need it if automatic size detection is enabled.
    bbox_important: bool,
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
    pub(crate) fn new(root_transform: Transform, bbox_important: bool) -> Self {
        Self {
            rd_builder: ResourceDictionaryBuilder::new(),
            validation_errors: HashSet::new(),
            content: Content::new(),
            root_transform,
            bbox_important,
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
            buf.to_vec(),
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

    #[track_caller]
    pub(crate) fn start_marked_content(&mut self, name: Name) {
        self.start_marked_content_prelude();
        self.content.begin_marked_content(name);
    }

    #[track_caller]
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

    pub(crate) fn draw_path(
        &mut self,
        path: &Path,
        mut fill: Option<&Fill>,
        stroke: Option<&Stroke>,
        sc: &mut SerializeContext,
    ) {
        if fill.is_none() && stroke.is_none() {
            return;
        }

        // Zero-size geometry, don't draw.
        if path.bounds().width() == 0.0 && path.bounds().height() == 0.0 {
            return;
        }

        // See issue 199.
        let is_line = path.verbs().len() == 2
            && path.verbs()[0] == PathVerb::Move
            && path.verbs()[1] == PathVerb::Line;
        let dont_fill = path.bounds().width() == 0.0 || path.bounds().height() == 0.0 || is_line;

        if dont_fill {
            match stroke.is_some() {
                // Some PDF viewers have bugs where they slightly fill zero-sized lines, so
                // don't draw them in the first place.
                false => return,
                // Don't fill, but we still want to draw the stroke, so don't return.
                true => {
                    fill = None;
                }
            }
        }

        let bbox_important = self.bbox_important;
        let calculate_bbox = |is_solid: bool| bbox_important || !is_solid;

        let stroke_bbox = |stroke: &Stroke| {
            if calculate_bbox(matches!(&stroke.paint.0, InnerPaint::Color(_))) {
                calculate_stroke_bbox(stroke, path).unwrap_or(Rect::from_tsp(path.bounds()))
            } else {
                Rect::from_tsp(path.bounds())
            }
        };

        let fill_prep = |sb: &mut ContentBuilder, fill: &Fill| {
            let has_pattern = matches!(fill.paint.0, InnerPaint::Pattern(_));
            let fill_opacity = fill.opacity;
            sb.expand_bbox(Rect::from_tsp(path.bounds()));

            // PDF viewers don't show patterns with fill/stroke opacities consistently.
            // Because of this, the opacity is accounted for in the pattern itself.
            if !has_pattern {
                sb.set_fill_opacity(fill_opacity);
            }
        };

        let stroke_prep = |sb: &mut ContentBuilder, stroke: &Stroke, stroke_bbox: Rect| {
            let is_pattern = matches!(stroke.paint.0, InnerPaint::Pattern(_));
            let stroke_opacity = stroke.opacity;
            sb.expand_bbox(stroke_bbox);

            // See comment in `set_fill_properties`
            if !is_pattern {
                sb.set_stroke_opacity(stroke_opacity);
            }
        };

        let fill_op = |sb: &mut ContentBuilder, sc: &mut SerializeContext, fill: &Fill| {
            let fill_rule = fill.rule;
            sb.content_set_fill_properties(Rect::from_tsp(path.bounds()), fill, sc);
            sb.content_draw_path(path.segments());

            match fill_rule {
                FillRule::NonZero => sb.content.fill_nonzero(),
                FillRule::EvenOdd => sb.content.fill_even_odd(),
            };
        };

        let stroke_op = |sb: &mut ContentBuilder,
                         sc: &mut SerializeContext,
                         stroke: &Stroke,
                         stroke_bbox: Rect| {
            sb.content_set_stroke_properties(stroke_bbox, stroke, sc);
            sb.content_draw_path(path.segments());
            sb.content.stroke();
        };

        let fill_stroke_op = |sb: &mut ContentBuilder,
                              sc: &mut SerializeContext,
                              fill: &Fill,
                              stroke: &Stroke,
                              stroke_bbox: Rect| {
            let fill_rule = fill.rule;
            sb.content_set_fill_properties(Rect::from_tsp(path.bounds()), fill, sc);
            sb.content_set_stroke_properties(stroke_bbox, stroke, sc);
            sb.content_draw_path(path.segments());

            match fill_rule {
                FillRule::NonZero => sb.content.fill_nonzero_and_stroke(),
                FillRule::EvenOdd => sb.content.fill_even_odd_and_stroke(),
            };
        };

        match (fill, stroke) {
            (Some(fill), None) => {
                self.apply_isolated_op(
                    |sb, _| {
                        fill_prep(sb, fill);
                    },
                    |sb, sc| {
                        fill_op(sb, sc, fill);
                    },
                    sc,
                );
            }
            (None, Some(stroke)) => {
                let stroke_bbox = stroke_bbox(stroke);

                self.apply_isolated_op(
                    |sb, _| {
                        stroke_prep(sb, stroke, stroke_bbox);
                    },
                    |sb, sc| {
                        stroke_op(sb, sc, stroke, stroke_bbox);
                    },
                    sc,
                );
            }
            (Some(fill), Some(stroke)) => {
                let stroke_bbox = stroke_bbox(stroke);

                self.apply_isolated_op(
                    |sb, _| {
                        fill_prep(sb, fill);
                        stroke_prep(sb, stroke, stroke_bbox);
                    },
                    |sb, sc| {
                        fill_stroke_op(sb, sc, fill, stroke, stroke_bbox);
                    },
                    sc,
                );
            }
            (None, None) => unreachable!(),
        }
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
    pub(crate) fn draw_glyphs(
        &mut self,
        start: Point,
        sc: &mut SerializeContext,
        fill: Option<&Fill>,
        stroke: Option<&Stroke>,
        context_color: rgb::Color,
        glyphs: &[impl Glyph],
        font: Font,
        text: &str,
        font_size: f32,
    ) {
        if fill.is_none() && stroke.is_none() {
            return;
        }

        let (x, y) = (start.x, start.y);
        self.graphics_states.save_state();

        // Calculating the glyphs bbox is very expensive but not always necessary, so omit
        // if not needed.
        let bbox_important = self.bbox_important;
        let calculate_bbox = |is_solid: bool| bbox_important || !is_solid;

        let fill_action = |sb: &mut ContentBuilder, sc: &mut SerializeContext, fill: &Fill| {
            let bbox = if calculate_bbox(matches!(&fill.paint.0, InnerPaint::Color(_))) {
                let bbox = get_glyphs_bbox(glyphs, x, y, font_size, font.clone());
                sb.expand_bbox(bbox);
                bbox
            } else {
                Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap()
            };

            sb.content_set_fill_properties(bbox, fill, sc)
        };

        let stroke_action =
            |sb: &mut ContentBuilder, sc: &mut SerializeContext, stroke: &Stroke| {
                let bbox = if calculate_bbox(matches!(&stroke.paint.0, InnerPaint::Color(_))) {
                    // TODO: Bbox should also account for stroke.
                    let bbox = get_glyphs_bbox(glyphs, x, y, font_size, font.clone());
                    sb.expand_bbox(bbox);
                    bbox
                } else {
                    Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap()
                };

                sb.content_set_stroke_properties(bbox, stroke, sc);
            };

        let set_fill_opacity = |sb: &mut ContentBuilder, fill: &Fill| {
            // PDF viewers don't show patterns with fill/stroke opacities consistently.
            // Because of this, the opacity is accounted for in the pattern itself.
            if !matches!(&fill.paint.0, &InnerPaint::Pattern(_)) {
                sb.set_fill_opacity(fill.opacity);
            }
        };

        let set_stroke_opacity = |sb: &mut ContentBuilder, stroke: &Stroke| {
            // PDF viewers don't show patterns with fill/stroke opacities consistently.
            // Because of this, the opacity is accounted for in the pattern itself.
            if !matches!(&stroke.paint.0, &InnerPaint::Pattern(_)) {
                // PDF viewers don't show patterns with fill/stroke opacities consistently.
                // Because of this, the opacity is accounted for in the pattern itself.
                sb.set_stroke_opacity(stroke.opacity);

                // See the comment in `stroke-action` for why we also set the fill opacity.
                sb.set_fill_opacity(stroke.opacity);
            }
        };

        match (fill, stroke) {
            (Some(f), Some(s)) => {
                set_fill_opacity(self, f);
                set_stroke_opacity(self, s);

                self.fill_stroke_glyph_run(
                    x,
                    y,
                    sc,
                    TextRenderingMode::FillStroke,
                    |sb, sc| {
                        fill_action(sb, sc, f);
                        stroke_action(sb, sc, s);
                    },
                    glyphs,
                    font.clone(),
                    context_color,
                    text,
                    font_size,
                );
            }
            (Some(f), None) => {
                set_fill_opacity(self, f);

                self.fill_stroke_glyph_run(
                    x,
                    y,
                    sc,
                    TextRenderingMode::Fill,
                    |sb, sc| {
                        fill_action(sb, sc, f);
                    },
                    glyphs,
                    font.clone(),
                    context_color,
                    text,
                    font_size,
                );
            }
            (None, Some(s)) => {
                set_stroke_opacity(self, s);

                self.fill_stroke_glyph_run(
                    x,
                    y,
                    sc,
                    TextRenderingMode::Stroke,
                    |sb, sc| {
                        stroke_action(sb, sc, s);
                    },
                    glyphs,
                    font.clone(),
                    context_color,
                    text,
                    font_size,
                );
            }
            (None, None) => unreachable!(),
        }

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
        context_color: rgb::Color,
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
                .get_gid(ColoredGlyph::new(glyph.glyph_id(), context_color))
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
        context_color: rgb::Color,
        text: &str,
        font_size: f32,
    ) {
        if glyphs.is_empty() {
            return;
        }

        self.apply_isolated_op(
            |_, _| {},
            |sb, sc| {
                let mut cur_x = x;
                let mut cur_y = ys;

                action(sb, sc);
                sb.content.begin_text();

                let font_container = sc.register_font_container(font.clone());
                let do_text_span = use_text_spanner(
                    glyphs,
                    text,
                    context_color,
                    &mut font_container.borrow_mut(),
                );

                if do_text_span {
                    // Separate into distinct glyph runs that either are encoded using actual text, or are
                    // not.
                    let spanned = GlyphSpanner::new(
                        glyphs,
                        text,
                        sc.serialize_settings()
                            .validator()
                            .requires_codepoint_mappings(),
                        context_color,
                        font_container.clone(),
                    );

                    for fragment in spanned {
                        sb.fill_stroke_glyph_span(
                            &mut cur_x,
                            &mut cur_y,
                            fragment,
                            sc,
                            fill_render_mode,
                            font_container.clone(),
                            context_color,
                            text,
                            font_size,
                        )
                    }
                } else {
                    let glyph_span = GlyphSpan::Unspanned(glyphs);

                    sb.fill_stroke_glyph_span(
                        &mut cur_x,
                        &mut cur_y,
                        glyph_span,
                        sc,
                        fill_render_mode,
                        font_container.clone(),
                        context_color,
                        text,
                        font_size,
                    )
                }

                sb.content.end_text();
            },
            sc,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn fill_stroke_glyph_span(
        &mut self,
        cur_x: &mut f32,
        cur_y: &mut f32,
        fragment: GlyphSpan<'_, impl Glyph>,
        sc: &mut SerializeContext,
        fill_render_mode: TextRenderingMode,
        font_container: Rc<RefCell<FontContainer>>,
        context_color: rgb::Color,
        text: &str,
        font_size: f32,
    ) {
        if let Some(text) = fragment.actual_text() {
            let mut actual_text = self
                .content
                .begin_marked_content_with_properties(Name(b"Span"));
            actual_text.properties().actual_text(TextStr(text));
        }

        // Segment into glyph runs that can be encoded in one go using a PDF
        // text showing operator (i.e. no y shift, same Type3 font, etc.)
        let segmented = GlyphGrouper::new(font_container.clone(), context_color, fragment.glyphs());

        for glyph_group in segmented {
            self.fill_stroke_glyph_group(
                cur_x,
                cur_y,
                glyph_group,
                sc,
                fill_render_mode,
                font_container.clone(),
                context_color,
                text,
                font_size,
            )
        }

        if fragment.actual_text().is_some() {
            self.content.end_marked_content();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn fill_stroke_glyph_group(
        &mut self,
        cur_x: &mut f32,
        cur_y: &mut f32,
        glyph_group: GlyphGroup<'_, impl Glyph>,
        sc: &mut SerializeContext,
        fill_render_mode: TextRenderingMode,
        font_container: Rc<RefCell<FontContainer>>,
        context_color: rgb::Color,
        text: &str,
        font_size: f32,
    ) {
        let borrowed = font_container.borrow();
        let pdf_font = borrowed
            .get_from_identifier(glyph_group.font_identifier.clone())
            .unwrap();

        if fill_render_mode == TextRenderingMode::Fill || pdf_font.force_fill() {
            self.content
                .set_text_rendering_mode(TextRenderingMode::Fill);
        } else if fill_render_mode == TextRenderingMode::FillStroke {
            self.content
                .set_text_rendering_mode(TextRenderingMode::FillStroke);
        } else {
            self.content
                .set_text_rendering_mode(TextRenderingMode::Stroke);
        }

        self.encode_consecutive_glyph_run(
            sc,
            cur_x,
            *cur_y - glyph_group.y_offset * font_size,
            glyph_group.font_identifier,
            pdf_font,
            font_size,
            context_color,
            glyph_group.glyphs,
            text,
        );

        *cur_y -= glyph_group.y_advance * font_size;
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
        stroke: &Stroke,
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
