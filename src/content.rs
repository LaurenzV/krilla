//! A low-level abstraction over a single content stream.

use crate::color::{Color, ColorSpace, DEVICE_CMYK, DEVICE_GRAY, DEVICE_RGB};
use crate::font::{Font, FontIdentifier, Glyph, GlyphUnits, PaintMode};
use crate::graphics_state::GraphicsStates;
#[cfg(feature = "raster-images")]
use crate::image::Image;
use crate::mask::Mask;
use crate::object::cid_font::CIDFont;
use crate::object::ext_g_state::ExtGState;
use crate::object::shading_function::{GradientProperties, GradientPropertiesExt, ShadingFunction};
use crate::object::shading_pattern::ShadingPattern;
use crate::object::tiling_pattern::TilingPattern;
use crate::object::type3_font::{CoveredGlyph, Type3Font};
use crate::object::xobject::XObject;
use crate::paint::{InnerPaint, Paint};
use crate::path::{Fill, FillRule, LineCap, LineJoin, Stroke};
use crate::resource::{
    ColorSpaceResource, PatternResource, Resource, ResourceDictionaryBuilder, XObjectResource,
};
use crate::serialize::{FontContainer, PDFGlyph, SerializerContext};
use crate::stream::Stream;
use crate::util::{calculate_stroke_bbox, LineCapExt, LineJoinExt, NameExt, RectExt, TransformExt};
use float_cmp::approx_eq;
use pdf_writer::types::TextRenderingMode;
use pdf_writer::{Content, Finish, Name, Str, TextStr};
use std::cell::{RefCell, RefMut};
use std::ops::Range;
#[cfg(feature = "raster-images")]
use tiny_skia_path::Size;
use tiny_skia_path::{NormalizedF32, Path, PathSegment, Point, Rect, Transform};

pub(crate) struct ContentBuilder {
    rd_builder: ResourceDictionaryBuilder,
    content: Content,
    graphics_states: GraphicsStates,
    bbox: Rect,
}

impl ContentBuilder {
    pub fn new() -> Self {
        Self {
            rd_builder: ResourceDictionaryBuilder::new(),
            content: Content::new(),
            graphics_states: GraphicsStates::new(),
            bbox: Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
        }
    }

    pub fn finish(self) -> Stream {
        Stream::new(self.content.finish(), self.bbox, self.rd_builder.finish())
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

    pub fn fill_path(
        &mut self,
        path: &Path,
        fill: Fill,
        serializer_context: &mut SerializerContext,
    ) {
        self.fill_path_impl(path, fill, serializer_context, true);
    }

    pub(crate) fn fill_path_impl(
        &mut self,
        path: &Path,
        fill: Fill,
        serializer_context: &mut SerializerContext,
        // This is only needed because when creating a Type3 glyph, we don't want to apply a
        // fill properties for outline glyphs, so that they are taken from wherever the glyph is shown.
        fill_props: bool,
    ) {
        if path.bounds().width() == 0.0 || path.bounds().height() == 0.0 {
            return;
        }

        self.apply_isolated_op(
            |sb| {
                sb.bbox
                    .expand(&sb.graphics_states.transform_bbox(path.bounds()));

                if fill_props {
                    // PDF viewers don't show patterns with fill/stroke opacities consistently.
                    // Because of this, the opacity is accounted for in the pattern itself.
                    if !matches!(fill.paint.0, InnerPaint::Pattern(_)) {
                        sb.set_fill_opacity(fill.opacity);
                    }
                }
            },
            |sb| {
                let fill_rule = fill.rule;
                if fill_props {
                    sb.content_set_fill_properties(path.bounds(), &fill, serializer_context);
                }
                sb.content_draw_path(path.segments());

                match fill_rule {
                    FillRule::NonZero => sb.content.fill_nonzero(),
                    FillRule::EvenOdd => sb.content.fill_even_odd(),
                };
            },
        );
    }

    pub fn stroke_path(
        &mut self,
        path: &Path,
        stroke: Stroke,
        serializer_context: &mut SerializerContext,
    ) -> Option<()> {
        if path.bounds().width() == 0.0 && path.bounds().height() == 0.0 {
            return Some(());
        }

        let stroke_bbox = calculate_stroke_bbox(&stroke, path).unwrap_or(path.bounds());

        self.apply_isolated_op(
            |sb| {
                sb.bbox
                    .expand(&sb.graphics_states.transform_bbox(stroke_bbox));

                // See comment in `set_fill_properties`
                if !matches!(stroke.paint.0, InnerPaint::Pattern(_)) {
                    sb.set_stroke_opacity(stroke.opacity);
                }
            },
            |sb| {
                sb.content_set_stroke_properties(stroke_bbox, &stroke, serializer_context);
                sb.content_draw_path(path.segments());
                sb.content.stroke();
            },
        );

        Some(())
    }

    pub fn push_clip_path(&mut self, path: &Path, clip_rule: &FillRule) {
        self.content.save_state();
        self.content_draw_path(
            path.clone()
                .transform(self.graphics_states.cur().transform())
                .unwrap()
                .segments(),
        );

        match clip_rule {
            FillRule::NonZero => self.content.clip_nonzero(),
            FillRule::EvenOdd => self.content.clip_even_odd(),
        };

        self.content.end_path();
    }

    pub fn pop_clip_path(&mut self) {
        self.content.restore_state();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fill_glyphs(
        &mut self,
        start: Point,
        sc: &mut SerializerContext,
        fill: Fill,
        glyphs: &[impl Glyph],
        font: Font,
        text: &str,
        font_size: f32,
        glyph_units: GlyphUnits,
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
                sb.content_set_fill_properties(
                    Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
                    &fill,
                    sc,
                )
            },
            glyphs,
            font,
            PaintMode::Fill(&fill),
            text,
            font_size,
            glyph_units,
        );

        self.graphics_states.restore_state();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stroke_glyphs(
        &mut self,
        start: Point,
        sc: &mut SerializerContext,
        stroke: Stroke,
        glyphs: &[impl Glyph],
        font: Font,
        text: &str,
        font_size: f32,
        glyph_units: GlyphUnits,
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
                sb.content_set_stroke_properties(
                    Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
                    &stroke,
                    sc,
                );

                // There is a very weird and inconsistent interaction between Type3
                // glyphs and stroking them. Each PDF viewer does something different.
                // Because of this, we simply set BOTH, fill and stroke when stroking
                // a run of glyphs.
                sb.content_set_fill_properties(
                    Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
                    &Fill {
                        paint: stroke.paint.clone(),
                        opacity: stroke.opacity,
                        rule: Default::default(),
                    },
                    sc,
                )
            },
            glyphs,
            font,
            PaintMode::Stroke(&stroke),
            text,
            font_size,
            glyph_units,
        );

        self.graphics_states.restore_state();
    }

    /// Encode a successive sequence of glyphs that share the same properties and
    /// can be encoded with one text showing operator.
    #[allow(clippy::too_many_arguments)]
    fn encode_consecutive_glyph_run(
        &mut self,
        cur_x: &mut f32,
        cur_y: f32,
        font_identifier: FontIdentifier,
        pdf_font: &dyn PdfFont,
        size: f32,
        paint_mode: PaintMode,
        glyphs: &[impl Glyph],
        glyph_units: GlyphUnits,
    ) {
        let font_name = self
            .rd_builder
            .register_resource(Resource::Font(font_identifier.clone()));
        self.content.set_font(font_name.to_pdf_name(), size);
        self.content.set_text_matrix(
            Transform::from_row(1.0, 0.0, 0.0, -1.0, *cur_x, cur_y).to_pdf_transform(),
        );

        let mut positioned = self.content.show_positioned();
        let mut items = positioned.items();

        let mut adjustment = 0.0;
        let mut encoded = vec![];

        for glyph in glyphs {
            let pdf_glyph = pdf_font
                .get_gid(CoveredGlyph::new(glyph.glyph_id(), paint_mode, size))
                .unwrap();

            let normalize =
                |v| unit_normalize(glyph_units, pdf_font.font().units_per_em(), size, v);

            let x_advance = normalize(glyph.x_advance()) * pdf_font.units_per_em();
            let font_advance = pdf_font
                .font()
                .advance_width(glyph.glyph_id())
                .map(|n| (n / pdf_font.font().units_per_em()) * pdf_font.units_per_em());
            let x_offset = normalize(glyph.x_offset()) * pdf_font.units_per_em();

            adjustment += x_offset;

            // Make sure we don't write miniscule adjustments
            if !approx_eq!(f32, adjustment, 0.0, epsilon = 0.001) {
                if !encoded.is_empty() {
                    items.show(Str(&encoded));
                    encoded.clear();
                }

                items.adjust(-adjustment);
                adjustment = 0.0;
            }

            pdf_glyph.encode_into(&mut encoded);

            if let Some(font_advance) = font_advance {
                adjustment += x_advance - font_advance;
            }

            adjustment -= x_offset;
            // cur_x/cur_y and glyph metrics are in user space units.
            *cur_x += normalize(glyph.x_advance()) * size;
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
        sc: &mut SerializerContext,
        fill_render_mode: TextRenderingMode,
        action: impl FnOnce(&mut ContentBuilder, &mut SerializerContext),
        glyphs: &[impl Glyph],
        font: Font,
        paint_mode: PaintMode,
        text: &str,
        font_size: f32,
        glyph_units: GlyphUnits,
    ) {
        self.apply_isolated_op(
            |_| {},
            |sb| {
                let mut cur_x = x;
                let mut cur_y = ys;

                action(sb, sc);
                sb.content.begin_text();

                let font_container = sc.create_or_get_font_container(font.clone());

                // Separate into distinct glyph runs that either are encoded using actual text, or are
                // not.
                let spanned = TextSpanner::new(glyphs, text, paint_mode, font_size, font_container);

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
                        GlyphGrouper::new(font_container, paint_mode, font_size, fragment.glyphs());

                    for glyph_group in segmented {
                        let borrowed = font_container.borrow();
                        let pdf_font = borrowed
                            .get_from_identifier(glyph_group.font_identifier.clone())
                            .unwrap();

                        let normalize = |v| {
                            unit_normalize(
                                glyph_units,
                                pdf_font.font().units_per_em(),
                                font_size,
                                v,
                            )
                        };

                        if fill_render_mode == TextRenderingMode::Fill || pdf_font.force_fill() {
                            sb.content.set_text_rendering_mode(TextRenderingMode::Fill);
                        } else {
                            sb.content
                                .set_text_rendering_mode(TextRenderingMode::Stroke);
                        }

                        sb.encode_consecutive_glyph_run(
                            &mut cur_x,
                            cur_y - normalize(glyph_group.y_offset) * font_size,
                            glyph_group.font_identifier,
                            pdf_font,
                            font_size,
                            paint_mode,
                            glyph_group.glyphs,
                            glyph_units,
                        );

                        cur_y -= normalize(glyph_group.y_advance) * font_size;
                    }

                    if fragment.actual_text().is_some() {
                        sb.content.end_marked_content();
                    }
                }

                sb.content.end_text();
            },
        )
    }

    pub(crate) fn draw_xobject(&mut self, x_object: XObject, state: &ExtGState) {
        let bbox = x_object.bbox();
        self.apply_isolated_op(
            |sb| {
                sb.graphics_states.combine(state);
                sb.bbox.expand(&bbox);
            },
            move |sb| {
                let x_object_name = sb
                    .rd_builder
                    .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
                sb.content.x_object(x_object_name.to_pdf_name());
            },
        );
    }

    pub fn draw_masked(&mut self, mask: Mask, stream: Stream) {
        let state = ExtGState::new().mask(mask);
        let x_object = XObject::new(stream, false, true, None);
        self.draw_xobject(x_object, &state);
    }

    pub fn draw_opacified(&mut self, opacity: NormalizedF32, stream: Stream) {
        let state = ExtGState::new()
            .stroking_alpha(opacity)
            .non_stroking_alpha(opacity);
        let x_object = XObject::new(stream, true, false, None);
        self.draw_xobject(x_object, &state);
    }

    pub fn draw_isolated(&mut self, stream: Stream) {
        let state = ExtGState::new();
        let x_object = XObject::new(stream, true, false, None);
        self.draw_xobject(x_object, &state);
    }

    #[cfg(feature = "raster-images")]
    pub fn draw_image(&mut self, image: Image, size: Size) {
        self.apply_isolated_op(
            |sb| {
                // Scale the image from 1x1 to the actual dimensions.
                let transform =
                    Transform::from_row(size.width(), 0.0, 0.0, -size.height(), 0.0, size.height());
                sb.concat_transform(&transform);
                sb.bbox.expand(
                    &sb.graphics_states
                        .transform_bbox(Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap()),
                );
            },
            move |sb| {
                let image_name = sb
                    .rd_builder
                    .register_resource(Resource::XObject(XObjectResource::Image(image)));

                sb.content.x_object(image_name.to_pdf_name());
            },
        );
    }

    pub(crate) fn draw_shading(&mut self, shading: &ShadingFunction) {
        self.apply_isolated_op(
            |_| {},
            move |sb| {
                let sh = sb
                    .rd_builder
                    .register_resource(Resource::Shading(shading.clone()));
                sb.content.shading(sh.to_pdf_name());
            },
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

    fn apply_isolated_op(&mut self, prep: impl FnOnce(&mut Self), op: impl FnOnce(&mut Self)) {
        self.save_graphics_state();
        self.content.save_state();

        prep(self);

        let transform = self.graphics_states.cur().transform();

        if transform != Transform::identity() {
            self.content.transform(transform.to_pdf_transform());
        }

        let state = self.graphics_states.cur().ext_g_state().clone();

        if !state.empty() {
            let ext = self
                .rd_builder
                .register_resource(Resource::ExtGState(state));
            self.content.set_parameters(ext.to_pdf_name());
        }

        op(self);

        self.content.restore_state();
        self.restore_graphics_state();
    }

    fn content_set_fill_stroke_properties(
        &mut self,
        bounds: Rect,
        paint: &Paint,
        opacity: NormalizedF32,
        serializer_context: &mut SerializerContext,
        mut set_pattern_fn: impl FnMut(&mut Content, String),
        mut set_solid_fn: impl FnMut(&mut Content, String, Color),
    ) {
        let no_device_cs = serializer_context.serialize_settings.no_device_cs;

        let pattern_transform = |transform: Transform| -> Transform {
            transform.post_concat(self.graphics_states.cur().transform())
        };

        let color_to_string =
            |color: Color, content_builder: &mut ContentBuilder, allow_gray: bool| match color
                .color_space(no_device_cs, allow_gray)
            {
                ColorSpace::Srgb => content_builder
                    .rd_builder
                    .register_resource(Resource::ColorSpace(ColorSpaceResource::Srgb)),
                ColorSpace::SGray => content_builder
                    .rd_builder
                    .register_resource(Resource::ColorSpace(ColorSpaceResource::SGray)),
                ColorSpace::DeviceRgb => DEVICE_RGB.to_string(),
                ColorSpace::DeviceGray => DEVICE_GRAY.to_string(),
                ColorSpace::DeviceCmyk => DEVICE_CMYK.to_string(),
            };

        let mut write_gradient =
            |gradient_props: GradientProperties,
             transform: Transform,
             content_builder: &mut ContentBuilder| {
                if let Some((color, opacity)) = gradient_props.single_stop_color() {
                    // Write gradients with one stop as a solid color fill.
                    // TODO: Does this leak the opacity?
                    content_builder.set_fill_opacity(opacity);
                    let color_space = color_to_string(color, content_builder, false);
                    set_solid_fn(&mut content_builder.content, color_space, color);
                } else {
                    let shading_mask = Mask::new_from_shading(
                        gradient_props.clone(),
                        transform,
                        bounds,
                        serializer_context,
                    );

                    let shading_pattern = ShadingPattern::new(
                        gradient_props,
                        content_builder
                            .graphics_states
                            .cur()
                            .transform()
                            .pre_concat(transform),
                    );
                    let color_space =
                        content_builder
                            .rd_builder
                            .register_resource(Resource::Pattern(PatternResource::ShadingPattern(
                                shading_pattern,
                            )));

                    if let Some(shading_mask) = shading_mask {
                        let state = ExtGState::new().mask(shading_mask);

                        let ext = content_builder
                            .rd_builder
                            .register_resource(Resource::ExtGState(state));
                        content_builder.content.set_parameters(ext.to_pdf_name());
                    }

                    set_pattern_fn(&mut content_builder.content, color_space);
                }
            };

        match &paint.0 {
            InnerPaint::Color(c) => {
                let color_space = color_to_string(*c, self, true);
                set_solid_fn(&mut self.content, color_space, *c);
            }
            InnerPaint::LinearGradient(lg) => {
                let (gradient_props, transform) = lg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, transform, self);
            }
            InnerPaint::RadialGradient(rg) => {
                let (gradient_props, transform) = rg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, transform, self);
            }
            InnerPaint::SweepGradient(sg) => {
                let (gradient_props, transform) = sg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, transform, self);
            }
            InnerPaint::Pattern(pat) => {
                let mut pat = pat.clone();
                let transform = pat.transform;

                pat.transform = pattern_transform(transform);

                let color_space = self.rd_builder.register_resource(Resource::Pattern(
                    PatternResource::TilingPattern(TilingPattern::new(
                        pat.stream,
                        pat.transform,
                        opacity,
                        pat.width,
                        pat.height,
                        serializer_context,
                    )),
                ));
                set_pattern_fn(&mut self.content, color_space);
            }
        }
    }

    fn content_set_fill_properties(
        &mut self,
        bounds: Rect,
        fill: &Fill,
        serializer_context: &mut SerializerContext,
    ) {
        fn set_pattern_fn(content: &mut Content, color_space: String) {
            content.set_fill_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
            content.set_fill_pattern(None, color_space.to_pdf_name());
        }

        fn set_solid_fn(content: &mut Content, color_space: String, color: Color) {
            content.set_fill_color_space(color_space.to_pdf_name());
            content.set_fill_color(color.to_pdf_color(true));
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
        serializer_context: &mut SerializerContext,
    ) {
        fn set_pattern_fn(content: &mut Content, color_space: String) {
            content.set_stroke_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
            content.set_stroke_pattern(None, color_space.to_pdf_name());
        }

        fn set_solid_fn(content: &mut Content, color_space: String, color: Color) {
            content.set_stroke_color_space(color_space.to_pdf_name());
            content.set_stroke_color(color.to_pdf_color(true));
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

pub(crate) fn unit_normalize(glyph_units: GlyphUnits, upem: f32, size: f32, val: f32) -> f32 {
    match glyph_units {
        GlyphUnits::Normalized => val,
        GlyphUnits::UnitsPerEm => val / upem,
        GlyphUnits::UserSpace => val / size,
    }
}

pub(crate) trait PdfFont {
    fn units_per_em(&self) -> f32;
    fn font(&self) -> Font;
    fn get_codepoints(&self, pdf_glyph: PDFGlyph) -> Option<&str>;
    fn set_codepoints(&mut self, pdf_glyph: PDFGlyph, text: String);
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

    fn set_codepoints(&mut self, pdf_glyph: PDFGlyph, text: String) {
        match pdf_glyph {
            PDFGlyph::Type3(t3) => self.set_codepoints(t3, text),
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
            PDFGlyph::Type3(_) => panic!("attempted to pass cid to type 3 font"),
            PDFGlyph::Cid(cid) => self.get_codepoints(cid),
        }
    }

    fn set_codepoints(&mut self, pdf_glyph: PDFGlyph, text: String) {
        match pdf_glyph {
            PDFGlyph::Type3(_) => panic!("attempted to pass cid to type 3 font"),
            PDFGlyph::Cid(cid) => self.set_codepoints(cid, text),
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
    pub fn glyphs(&self) -> &[T] {
        match self {
            TextSpan::Unspanned(glyphs) => glyphs,
            TextSpan::Spanned(glyphs, _) => glyphs,
        }
    }

    pub fn actual_text(&self) -> Option<&str> {
        match self {
            TextSpan::Unspanned(_) => None,
            TextSpan::Spanned(_, text) => Some(text),
        }
    }
}

/// In PDF, correspondences between glyphs and Unicode codepoints are expressed
/// via a CMAP. In a CMAP, you can assign a sequence of unicode codepoints to each
/// glyph. There are two issues with this approach:
/// - How to deal with the fact that the same glyph might be assigned two different codepoints
/// in different contexts (i.e. space and NZWJ).
/// - How to deal with complex shaping scenarios, where there is not a one-to-one or
/// one-to-many correspondence between glyphs and codepoints, but instead a many-to-one
/// or many-to-many mapping.
///
/// The answer to this is the `ActualText` feature of PDF, which allows to define some custom
/// actual text for a number of glyphs, which overrides anything else. Unfortunately, this
/// is seemingly only supported in Acrobat and Chrome, but it's the only proper way of addressing
/// this issue.
///
/// This is the task of the `TextSpanner`. Given a sequence of glyphs, it segments the
/// sequence into subruns of glyphs that either do need to be wrapped in an actual text
/// attribute, or not.
pub(crate) struct TextSpanner<'a, 'b, T>
where
    T: Glyph,
{
    slice: &'a [T],
    paint_mode: PaintMode<'a>,
    font_size: f32,
    font_container: &'b RefCell<FontContainer>,
    text: &'a str,
}

impl<'a, 'b, T> TextSpanner<'a, 'b, T>
where
    T: Glyph,
{
    pub fn new(
        slice: &'a [T],
        text: &'a str,
        paint_mode: PaintMode<'a>,
        font_size: f32,
        font_container: &'b RefCell<FontContainer>,
    ) -> Self {
        Self {
            slice,
            paint_mode,
            text,
            font_container,
            font_size,
        }
    }
}

impl<'a, T> Iterator for TextSpanner<'a, '_, T>
where
    T: Glyph,
{
    type Item = TextSpan<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        fn func<U>(
            g: &U,
            paint_mode: PaintMode,
            font_size: f32,
            mut font_container: RefMut<FontContainer>,
            text: &str,
        ) -> (Range<usize>, bool)
        where
            U: Glyph,
        {
            let (identifier, pdf_glyph) =
                font_container.add_glyph(CoveredGlyph::new(g.glyph_id(), paint_mode, font_size));
            let pdf_font = font_container
                .get_from_identifier_mut(identifier.clone())
                .unwrap();

            let range = g.text_range().clone();
            let text = &text[range.clone()];
            let codepoints = pdf_font.get_codepoints(pdf_glyph);
            // Check if the glyph has already been assigned codepoints that don't match the
            // one we are seeing right now.
            let incompatible_codepoint = codepoints.is_some() && codepoints != Some(text);

            // Only set the codepoint if there isn't a previous one.
            if !incompatible_codepoint {
                pdf_font.set_codepoints(pdf_glyph, text.to_string());
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
            self.font_size,
            self.font_container.borrow_mut(),
            self.text,
        );

        let mut last_range = first_range.clone();

        for next in iter {
            let (next_range, next_incompatible) = func(
                next,
                self.paint_mode,
                self.font_size,
                self.font_container.borrow_mut(),
                self.text,
            );

            match use_span {
                // In this case, we just started and we are looking at the first two glyphs.
                // This decides whether the current run will be spanned, or not.
                None => {
                    // The first glyph is incompatible, so we definitely need actual text.
                    if first_incompatible {
                        use_span = Some(true);

                        // If the range of the next one is the same, it means they are
                        // part of the same cluster, meaning that we need to include it
                        // in the actual text. If not, we abort and only wrap the first
                        // glyph in actual text.
                        if last_range != next_range {
                            break;
                        }
                    }

                    // If the next is incompatible but not part of the current cluster,
                    // then it will need a dedicated spanned range, and
                    // we can't include it in the current text span. So we abort and
                    // create a spanned element with just the first glyph.
                    if next_incompatible && last_range != next_range {
                        break;
                    }

                    // If they have the same range, they are part of the same cluster,
                    // and thus we started a spanned range with actual text.
                    //
                    // Otherwise, they are part of a different cluster, and we
                    // start a spanned range with no actual text (common case).
                    use_span = Some(last_range == next_range);
                }
                // We are currently building a spanned range, and all glyphs
                // are part of the same cluster.
                Some(true) => {
                    // If the next glyph is not part of the same cluster, terminate the current
                    // span and don't include the next one.
                    if last_range != next_range {
                        break;
                    }
                }
                // We are currently building an unspanned range, meaning the
                // glyphs are not part of the same cluster.
                Some(false) => {
                    // If the current and the last one are part of the same range
                    // this means that they are part of the same cluster. This means
                    // that the current AND the last one belong to a spanned segment,
                    // so we need to do count -= 1 as well before terminating.
                    if last_range == next_range {
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

            last_range = next.text_range().clone();
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
    y_offset: f32,
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
pub(crate) struct GlyphGrouper<'a, 'b, T>
where
    T: Glyph,
{
    font_container: &'b RefCell<FontContainer>,
    paint_mode: PaintMode<'a>,
    font_size: f32,
    slice: &'a [T],
}

impl<'a, 'b, T> GlyphGrouper<'a, 'b, T>
where
    T: Glyph,
{
    pub fn new(
        font_container: &'b RefCell<FontContainer>,
        paint_mode: PaintMode<'a>,
        font_size: f32,
        slice: &'a [T],
    ) -> Self {
        Self {
            font_container,
            paint_mode,
            font_size,
            slice,
        }
    }
}

impl<'a, T> Iterator for GlyphGrouper<'a, '_, T>
where
    T: Glyph,
{
    type Item = GlyphGroup<'a, T>;

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
                font_size: f32,
                font_container: RefMut<FontContainer>,
            ) -> GlyphProps
            where
                U: Glyph,
            {
                // Safe because we've already added all glyphs in the text spanner.
                let font_identifier = font_container
                    .font_identifier(CoveredGlyph::new(g.glyph_id(), paint_mode, font_size))
                    .unwrap();

                GlyphProps {
                    font_identifier,
                    y_offset: g.y_offset(),
                    y_advance: g.y_advance(),
                }
            }

            let mut count = 1;

            let mut iter = self.slice.iter();
            let first = func(
                iter.next()?,
                self.paint_mode,
                self.font_size,
                self.font_container.borrow_mut(),
            );

            for next in iter {
                let temp_glyph = func(
                    next,
                    self.paint_mode,
                    self.font_size,
                    self.font_container.borrow_mut(),
                );

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
