//! A low-level abstraction over a single content stream.

use crate::color::{Color, ColorSpace, ColorSpaceType, DEVICE_CMYK, DEVICE_GRAY, DEVICE_RGB};
use crate::font::{Font, FontIdentifier, Glyph, GlyphUnits, KrillaGlyph};
use crate::graphics_state::GraphicsStates;
#[cfg(feature = "raster-images")]
use crate::image::Image;
use crate::mask::Mask;
use crate::object::cid_font::CIDFont;
use crate::object::ext_g_state::ExtGState;
use crate::object::shading_function::{GradientProperties, GradientPropertiesExt, ShadingFunction};
use crate::object::shading_pattern::ShadingPattern;
use crate::object::tiling_pattern::TilingPattern;
use crate::object::type3_font::Type3Font;
use crate::object::xobject::XObject;
use crate::paint::Paint;
use crate::path::{Fill, FillRule, LineCap, LineJoin, Stroke};
use crate::resource::{
    ColorSpaceResource, PatternResource, Resource, ResourceDictionaryBuilder, XObjectResource,
};
use crate::serialize::{FontContainer, PDFGlyph, SerializerContext};
use crate::stream::Stream;
use crate::util::{
    calculate_stroke_bbox, LineCapExt, LineJoinExt, NameExt, RectExt, TransformExt,
    TransformWrapper,
};
use float_cmp::approx_eq;
use pdf_writer::types::TextRenderingMode;
use pdf_writer::{Content, Finish, Name, Str, TextStr};
use skrifa::GlyphId;
use std::cell::{RefCell, RefMut};
use std::ops::Range;
#[cfg(feature = "raster-images")]
use tiny_skia_path::Size;
use tiny_skia_path::{FiniteF32, NormalizedF32, Path, PathSegment, Point, Rect, Transform};

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
        fill: Fill<impl ColorSpace>,
        serializer_context: &mut SerializerContext,
    ) {
        self.fill_path_impl(path, fill, serializer_context, false);
    }

    pub(crate) fn fill_path_impl(
        &mut self,
        path: &Path,
        fill: Fill<impl ColorSpace>,
        serializer_context: &mut SerializerContext,
        // This is only needed because when creating a Type3 glyph, we don't want to apply a
        // fill color for outline glyphs, so that they are taken from wherever the glyph is shown.
        no_fill: bool,
    ) {
        if path.bounds().width() == 0.0 || path.bounds().height() == 0.0 {
            return;
        }

        self.bbox
            .expand(&self.graphics_states.transform_bbox(path.bounds()));

        self.graphics_states.save_state();

        // PDF viewers don't show patterns with fill/stroke opacities consistently.
        // Because of this, the opacity is accounted for in the pattern itself.
        if !matches!(fill.paint, Paint::Pattern(_)) {
            self.set_fill_opacity(fill.opacity);
        }

        self.apply_isolated_op(|sb| {
            let fill_rule = fill.rule;
            if !no_fill {
                sb.content_set_fill_properties(path.bounds(), fill, serializer_context);
            }
            sb.content_draw_path(path.segments());

            match fill_rule {
                FillRule::NonZero => sb.content.fill_nonzero(),
                FillRule::EvenOdd => sb.content.fill_even_odd(),
            };
        });

        self.graphics_states.restore_state();
    }

    pub fn stroke_path(
        &mut self,
        path: &Path,
        stroke: Stroke<impl ColorSpace>,
        serializer_context: &mut SerializerContext,
    ) -> Option<()> {
        if path.bounds().width() == 0.0 && path.bounds().height() == 0.0 {
            return Some(());
        }

        // TODO: Revisit whether we shouldn't just use a dummy bbox instead.
        let stroke_bbox = calculate_stroke_bbox(&stroke, path)?;
        self.bbox
            .expand(&self.graphics_states.transform_bbox(stroke_bbox));

        self.graphics_states.save_state();

        // See comment in `set_fill_properties`
        if !matches!(stroke.paint, Paint::Pattern(_)) {
            self.set_stroke_opacity(stroke.opacity);
        }

        self.apply_isolated_op(|sb| {
            sb.content_set_stroke_properties(stroke_bbox, stroke, serializer_context);
            sb.content_draw_path(path.segments());
            sb.content.stroke();
        });

        self.graphics_states.restore_state();
        Some(())
    }

    pub fn push_clip_path(&mut self, path: &Path, clip_rule: &FillRule) {
        self.content_save_state();
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
        self.content_restore_state();
    }

    pub fn fill_glyphs(
        &mut self,
        start: Point,
        sc: &mut SerializerContext,
        fill: Fill<impl ColorSpace>,
        glyphs: &[impl Glyph],
        font: Font,
        text: &str,
        font_size: f32,
        glyph_units: GlyphUnits
    ) {
        let (x, y) = (start.x, start.y);
        self.graphics_states.save_state();

        // PDF viewers don't show patterns with fill/stroke opacities consistently.
        // Because of this, the opacity is accounted for in the pattern itself.
        if !matches!(&fill.paint, &Paint::Pattern(_)) {
            self.set_fill_opacity(fill.opacity);
        }

        self.fill_stroke_glyph_run(
            x,
            y,
            sc,
            TextRenderingMode::Fill,
            move |sb, sc| {
                sb.content_set_fill_properties(
                    Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
                    fill,
                    sc,
                )
            },
            glyphs,
            font,
            text,
            font_size,
            glyph_units
        );

        self.graphics_states.restore_state();
    }

    pub fn stroke_glyphs(
        &mut self,
        start: Point,
        sc: &mut SerializerContext,
        stroke: Stroke<impl ColorSpace>,
        glyphs: &[KrillaGlyph],
        font: Font,
        text: &str,
        font_size: f32,
        glyph_units: GlyphUnits
    ) {
        let (x, y) = (start.x, start.y);
        self.graphics_states.save_state();

        // PDF viewers don't show patterns with fill/stroke opacities consistently.
        // Because of this, the opacity is accounted for in the pattern itself.
        if !matches!(&stroke.paint, &Paint::Pattern(_)) {
            self.set_stroke_opacity(stroke.opacity);
        }

        self.fill_stroke_glyph_run(
            x,
            y,
            sc,
            TextRenderingMode::Stroke,
            |sb, sc| {
                sb.content_set_stroke_properties(
                    Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
                    stroke,
                    sc,
                )
            },
            glyphs,
            font,
            text,
            font_size,
            glyph_units
        );

        self.graphics_states.restore_state();
    }

    /// Encode a successive sequence of glyphs that share the same properties and
    /// can be encoded with one text showing operator.
    fn encode_consecutive_glyph_run(
        &mut self,
        cur_x: &mut f32,
        cur_y: f32,
        font_identifier: FontIdentifier,
        pdf_font: &dyn PdfFont,
        size: f32,
        glyphs: &[impl Glyph],
        glyph_units: GlyphUnits
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
            let pdf_glyph = pdf_font.get_gid(glyph.glyph_id()).unwrap();

            let normalize = |v| {
                unit_normalize(glyph_units, pdf_font, size, v)
            };

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
        y: f32,
        sc: &mut SerializerContext,
        text_rendering_mode: TextRenderingMode,
        action: impl FnOnce(&mut ContentBuilder, &mut SerializerContext),
        glyphs: &[impl Glyph],
        font: Font,
        text: &str,
        font_size: f32,
        glyph_units: GlyphUnits
    ) {
        let mut cur_x = x;

        self.apply_isolated_op(|sb| {
            action(sb, sc);
            sb.content.begin_text();
            sb.content.set_text_rendering_mode(text_rendering_mode);

            let font_container = sc.create_or_get_font_container(font.clone());

            // Separate into distinct glyph runs that either are encoded using actual text, or are
            // not.
            let spanned = TextSpanner::new(glyphs, text, font_container);

            for fragment in spanned {
                if let Some(text) = fragment.actual_text() {
                    let mut actual_text = sb
                        .content
                        .begin_marked_content_with_properties(Name(b"Span"));
                    actual_text.properties().actual_text(TextStr(text));
                }

                // Segment into glyph runs that can be encoded in one go using a PDF
                // text showing operator (i.e. no y shift, same Type3 font, etc.)
                let segmented = GlyphGrouper::new(font_container, fragment.glyphs());

                for glyph_group in segmented {
                    let borrowed = font_container.borrow();
                    let pdf_font = borrowed
                        .get_from_identifier(glyph_group.font_identifier.clone())
                        .unwrap();

                    sb.encode_consecutive_glyph_run(
                        &mut cur_x,
                        y - unit_normalize(glyph_units, pdf_font, font_size, glyph_group.y_offset) * font_size,
                        glyph_group.font_identifier,
                        pdf_font,
                        font_size,
                        &glyph_group.glyphs,
                        glyph_units
                    )
                }

                if fragment.actual_text().is_some() {
                    sb.content.end_marked_content();
                }
            }

            sb.content.end_text();
        })
    }

    pub(crate) fn draw_xobject(&mut self, x_object: XObject, state: &ExtGState) {
        self.graphics_states.save_state();
        self.graphics_states.combine(state);

        self.bbox.expand(&x_object.bbox());

        self.apply_isolated_op(move |sb| {
            let x_object_name = sb
                .rd_builder
                .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
            sb.content.x_object(x_object_name.to_pdf_name());
        });

        self.graphics_states.restore_state();
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
        self.save_graphics_state();
        // Scale the image from 1x1 to the actual dimensions.
        let transform =
            Transform::from_row(size.width(), 0.0, 0.0, -size.height(), 0.0, size.height());
        self.concat_transform(&transform);
        self.bbox.expand(
            &self
                .graphics_states
                .transform_bbox(Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap()),
        );

        self.apply_isolated_op(move |sb| {
            let image_name = sb
                .rd_builder
                .register_resource(Resource::XObject(XObjectResource::Image(image)));

            sb.content.x_object(image_name.to_pdf_name());
        });

        self.restore_graphics_state();
    }

    pub(crate) fn draw_shading(&mut self, shading: &ShadingFunction) {
        self.apply_isolated_op(move |sb| {
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

    fn apply_isolated_op(&mut self, op: impl FnOnce(&mut Self)) {
        self.save_graphics_state();
        self.content_save_state();
        self.content_set_transform();
        self.content_set_ext_state();

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

        if !state.empty() {
            let ext = self
                .rd_builder
                .register_resource(Resource::ExtGState(state));
            self.content.set_parameters(ext.to_pdf_name());
        }
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
        paint: Paint<impl ColorSpace>,
        opacity: NormalizedF32,
        serializer_context: &mut SerializerContext,
        mut set_pattern_fn: impl FnMut(&mut Content, String),
        mut set_solid_fn: impl FnMut(&mut Content, String, &Color),
    ) {
        let no_device_cs = serializer_context.serialize_settings.no_device_cs;

        let pattern_transform = |transform: Transform| -> Transform {
            transform.post_concat(self.graphics_states.cur().transform())
        };

        let color_to_string = |color: Color, content_builder: &mut ContentBuilder| match color
            .color_space(no_device_cs)
        {
            ColorSpaceType::Srgb(srgb) => content_builder
                .rd_builder
                .register_resource(Resource::ColorSpace(ColorSpaceResource::Srgb(srgb))),
            ColorSpaceType::SGray(sgray) => content_builder
                .rd_builder
                .register_resource(Resource::ColorSpace(ColorSpaceResource::SGray(sgray))),
            ColorSpaceType::DeviceRgb(_) => DEVICE_RGB.to_string(),
            ColorSpaceType::DeviceGray(_) => DEVICE_GRAY.to_string(),
            ColorSpaceType::DeviceCmyk(_) => DEVICE_CMYK.to_string(),
        };

        let mut write_gradient =
            |gradient_props: GradientProperties,
             transform: TransformWrapper,
             content_builder: &mut ContentBuilder| {
                if let Some((color, opacity)) = gradient_props.single_stop_color() {
                    // Write gradients with one stop as a solid color fill.
                    content_builder.set_fill_opacity(opacity);
                    let color_space = color_to_string(color, content_builder);
                    set_solid_fn(&mut content_builder.content, color_space, &color);
                } else {
                    let shading_mask = Mask::new_from_shading(
                        gradient_props.clone(),
                        transform,
                        bounds,
                        serializer_context,
                    );

                    let shading_pattern = ShadingPattern::new(
                        gradient_props,
                        TransformWrapper(
                            content_builder
                                .graphics_states
                                .cur()
                                .transform()
                                .pre_concat(transform.0),
                        ),
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

        match paint {
            Paint::Color(c) => {
                let color_space = color_to_string(c.into(), self);
                set_solid_fn(&mut self.content, color_space, &c.into());
            }
            Paint::LinearGradient(lg) => {
                let (gradient_props, transform) = lg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, transform, self);
            }
            Paint::RadialGradient(rg) => {
                let (gradient_props, transform) = rg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, transform, self);
            }
            Paint::SweepGradient(sg) => {
                let (gradient_props, transform) = sg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, transform, self);
            }
            Paint::Pattern(mut pat) => {
                let transform = pat.transform;

                pat.transform = pattern_transform(transform);

                let color_space = self.rd_builder.register_resource(Resource::Pattern(
                    PatternResource::TilingPattern(TilingPattern::new(
                        pat.stream,
                        TransformWrapper(pat.transform),
                        opacity,
                        FiniteF32::new(pat.width).unwrap(),
                        FiniteF32::new(pat.height).unwrap(),
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
        fill: Fill<impl ColorSpace>,
        serializer_context: &mut SerializerContext,
    ) {
        fn set_pattern_fn(content: &mut Content, color_space: String) {
            content.set_fill_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
            content.set_fill_pattern(None, color_space.to_pdf_name());
        }

        fn set_solid_fn(content: &mut Content, color_space: String, color: &Color) {
            content.set_fill_color_space(color_space.to_pdf_name());
            content.set_fill_color(color.to_pdf_color());
        }

        self.content_set_fill_stroke_properties(
            bounds,
            fill.paint,
            fill.opacity,
            serializer_context,
            set_pattern_fn,
            set_solid_fn,
        );
    }

    fn content_set_stroke_properties(
        &mut self,
        bounds: Rect,
        stroke: Stroke<impl ColorSpace>,
        serializer_context: &mut SerializerContext,
    ) {
        fn set_pattern_fn(content: &mut Content, color_space: String) {
            content.set_stroke_color_space(pdf_writer::types::ColorSpaceOperand::Pattern);
            content.set_stroke_pattern(None, color_space.to_pdf_name());
        }

        fn set_solid_fn(content: &mut Content, color_space: String, color: &Color) {
            content.set_stroke_color_space(color_space.to_pdf_name());
            content.set_stroke_color(color.to_pdf_color());
        }

        self.content_set_fill_stroke_properties(
            bounds,
            stroke.paint,
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

fn unit_normalize(glyph_units: GlyphUnits, pdf_font: &dyn PdfFont, size: f32, val: f32) -> f32 {
    match glyph_units {
        GlyphUnits::Normalized => val,
        GlyphUnits::UnitsPerEm => val / pdf_font.font().units_per_em(),
        GlyphUnits::UserSpace => val / size
    }
}

pub(crate) trait PdfFont {
    fn units_per_em(&self) -> f32;
    fn font(&self) -> Font;
    fn get_codepoints(&self, pdf_glyph: PDFGlyph) -> Option<&str>;
    fn set_codepoints(&mut self, pdf_glyph: PDFGlyph, text: String);
    fn get_gid(&self, gid: GlyphId) -> Option<PDFGlyph>;
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

    fn get_gid(&self, gid: GlyphId) -> Option<PDFGlyph> {
        self.get_gid(gid).map(PDFGlyph::Type3)
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

    fn get_gid(&self, gid: GlyphId) -> Option<PDFGlyph> {
        self.get_cid(gid).map(PDFGlyph::Cid)
    }
}

pub(crate) enum TextSpan<'a, T> where T: Glyph {
    Unspanned(&'a [T]),
    Spanned(&'a [T], &'a str),
}

impl<T> TextSpan<'_, T> where T: Glyph {
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

pub(crate) struct TextSpanner<'a, 'b, T> where T: Glyph {
    slice: &'a [T],
    font_container: &'b RefCell<FontContainer>,
    text: &'a str,
}

impl<'a, 'b, T> TextSpanner<'a, 'b, T> where T: Glyph {
    pub fn new(
        slice: &'a [T],
        text: &'a str,
        font_container: &'b RefCell<FontContainer>,
    ) -> Self {
        Self {
            slice,
            text,
            font_container,
        }
    }
}

impl<'a, T> Iterator for TextSpanner<'a, '_, T> where T: Glyph {
    type Item = TextSpan<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        fn func<U>(g: &U, mut font_container: RefMut<FontContainer>, text: &str) -> (Range<usize>, bool) where U: Glyph {
            let (identifier, pdf_glyph) = font_container.add_glyph(g.glyph_id());
            let pdf_font = font_container
                .get_from_identifier_mut(identifier.clone())
                .unwrap();

            let range = g.text_range().clone();
            let text = &text[range.clone()];
            let codepoints = pdf_font.get_codepoints(pdf_glyph);
            let incompatible_codepoint = codepoints.is_some() && codepoints != Some(text);

            if !incompatible_codepoint {
                pdf_font.set_codepoints(pdf_glyph, text.to_string());
            }

            (range, incompatible_codepoint)
        }

        let mut use_span = None;
        let mut count = 1;

        let mut iter = self.slice.iter();
        let (first_range, first_incompatible) = func(iter.next()?, self.font_container.borrow_mut(), self.text);

        let mut last_range = first_range.clone();

        for next in iter {
            let (next_range, next_incompatible) = func(next, self.font_container.borrow_mut(), self.text);

            match use_span {
                None => {
                    if first_incompatible {
                        use_span = Some(true);

                        if last_range != next_range {
                            break;
                        }
                    }

                    if next_incompatible && last_range != next_range {
                        break;
                    }

                    use_span = Some(last_range == next_range);
                }
                Some(true) => {
                    if last_range != next_range {
                        break;
                    }
                }
                Some(false) => {
                    if next_incompatible && last_range != next_range {
                        break;
                    }

                    if last_range == next_range {
                        count -= 1;
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

pub(crate) struct GlyphGroup<'a, T> where T: Glyph {
    font_identifier: FontIdentifier,
    glyphs: &'a [T],
    y_offset: f32,
}

impl<'a, T> GlyphGroup<'a, T> where T: Glyph {
    pub fn new(font_identifier: FontIdentifier, glyphs: &'a [T], y_offset: f32) -> Self {
        GlyphGroup {
            font_identifier,
            glyphs,
            y_offset,
        }
    }
}

pub(crate) struct GlyphGrouper<'a, 'b, T> where T: Glyph {
    font_container: &'b RefCell<FontContainer>,
    slice: &'a [T],
}

impl<'a, 'b, T> GlyphGrouper<'a, 'b, T> where T: Glyph {
    pub fn new(font_container: &'b RefCell<FontContainer>, slice: &'a [T]) -> Self {
        Self {
            font_container,
            slice,
        }
    }
}

impl<'a, T> Iterator for GlyphGrouper<'a, '_, T> where T: Glyph {
    type Item = GlyphGroup<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        // Guarantees: All glyphs in `head` have the font identifier that is given in
        // `props`, the same size and the same y offset.
        let (head, tail, props) = {
            struct GlyphProps {
                font_identifier: FontIdentifier,
                y_offset: f32,
            }

            fn func<U>(g: &U, font_container: RefMut<FontContainer>) -> GlyphProps where U: Glyph {
                // Safe because we've already added all glyphs in the text spanner.
                let font_identifier = font_container.font_identifier(g.glyph_id()).unwrap();

                GlyphProps {
                    font_identifier,
                    y_offset: g.y_offset(),
                }
            }

            let mut count = 1;

            let mut iter = self.slice.iter();
            let first = func(iter.next()?, self.font_container.borrow_mut());

            for next in iter {
                let temp_glyph = func(next, self.font_container.borrow_mut());

                if first.font_identifier != temp_glyph.font_identifier
                    || first.y_offset != temp_glyph.y_offset
                {
                    break;
                }

                count += 1;
            }

            let (head, tail) = self.slice.split_at(count);
            (head, tail, first)
        };

        self.slice = tail;

        let glyph_group = GlyphGroup::new(props.font_identifier, head, props.y_offset);

        Some(glyph_group)
    }
}
