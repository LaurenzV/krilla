use crate::font::{Font, FontIdentifier};
use crate::graphics_state::GraphicsStates;
use crate::object::cid_font::CIDFont;
use crate::object::color_space::{Color, ColorSpace};
use crate::object::ext_g_state::ExtGState;
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_function::{GradientProperties, GradientPropertiesExt, ShadingFunction};
use crate::object::shading_pattern::ShadingPattern;
use crate::object::tiling_pattern::TilingPattern;
use crate::object::type3_font::Type3Font;
use crate::object::xobject::XObject;
use crate::resource::{
    PatternResource, Resource, ResourceDictionary, ResourceDictionaryBuilder, XObjectResource,
};
use crate::serialize::{FontContainer, PDFGlyph, SerializerContext};
use crate::transform::TransformWrapper;
use crate::util::{calculate_stroke_bbox, LineCapExt, LineJoinExt, NameExt, RectExt, TransformExt};
use crate::{Fill, FillRule, LineCap, LineJoin, Paint, Stroke};
use float_cmp::approx_eq;
use pdf_writer::types::TextRenderingMode;
use pdf_writer::{Content, Finish, Name, Str, TextStr};
use skrifa::GlyphId;
use std::cell::RefCell;
use std::ops::Range;
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, NormalizedF32, Path, PathSegment, Rect, Size, Transform};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct Repr {
    content: Vec<u8>,
    bbox: Rect,
    resource_dictionary: ResourceDictionary,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Stream(Arc<Repr>);

impl Stream {
    pub(crate) fn new(
        content: Vec<u8>,
        bbox: Rect,
        resource_dictionary: ResourceDictionary,
    ) -> Self {
        Self(Arc::new(Repr {
            content,
            bbox,
            resource_dictionary,
        }))
    }

    pub(crate) fn content(&self) -> &[u8] {
        &self.0.content
    }

    pub(crate) fn bbox(&self) -> Rect {
        self.0.bbox
    }

    pub(crate) fn resource_dictionary(&self) -> &ResourceDictionary {
        &self.0.resource_dictionary
    }

    pub fn empty() -> Self {
        Self(Arc::new(Repr {
            content: vec![],
            bbox: Rect::from_xywh(0.0, 0.0, 0.0, 0.0).unwrap(),
            resource_dictionary: ResourceDictionaryBuilder::new().finish(),
        }))
    }
}

pub struct ContentBuilder {
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
    ) {
        if path.bounds().width() == 0.0 && path.bounds().height() == 0.0 {
            return;
        }

        let stroke_bbox = calculate_stroke_bbox(&stroke, path).unwrap();
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

        self.graphics_states.restore_state()
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

    pub fn fill_glyph_run<'a>(
        &mut self,
        x: f32,
        y: f32,
        sc: &mut SerializerContext,
        fill: Fill<impl ColorSpace>,
        glyphs: &[Glyph],
        font: Font,
        text: &str,
    ) {
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
        );

        self.graphics_states.restore_state();
    }

    pub fn stroke_glyph_run<'a>(
        &mut self,
        x: f32,
        y: f32,
        sc: &mut SerializerContext,
        stroke: Stroke<impl ColorSpace>,
        glyphs: &[Glyph],
        font: Font,
        text: &str,
    ) {
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
        );

        self.graphics_states.restore_state();
    }

    fn encode_consecutive_glyph_run(
        &mut self,
        cur_x: &mut f32,
        cur_y: f32,
        font_identifier: FontIdentifier,
        size: f32,
        // The y offset is already accounted for when splitting the glyph runs,
        // so we ignore it here.
        glyphs: &[InstanceGlyph],
    ) {
        let font_name = self
            .rd_builder
            .register_resource(Resource::Font(font_identifier));
        self.content.set_font(font_name.to_pdf_name(), size);
        self.content.set_text_matrix(
            Transform::from_row(1.0, 0.0, 0.0, -1.0, *cur_x, cur_y).to_pdf_transform(),
        );

        let mut positioned = self.content.show_positioned();
        let mut items = positioned.items();

        let mut adjustment = 0.0;
        let mut encoded = vec![];

        for glyph in glyphs {
            let actual_advance = glyph.x_advance / size * 1000.0;

            adjustment += glyph.x_offset / size * 1000.0;

            if !approx_eq!(f32, adjustment, 0.0, epsilon = 0.001) {
                if !encoded.is_empty() {
                    items.show(Str(&encoded));
                    encoded.clear();
                }

                items.adjust(-adjustment);
                adjustment = 0.0;
            }

            match &glyph.pdf_glyph {
                PDFGlyph::Type3(cg) => encoded.push(*cg),
                PDFGlyph::CID(cid) => {
                    encoded.push((cid >> 8) as u8);
                    encoded.push((cid & 0xff) as u8);
                }
            }

            if let Some(advance) = glyph.font_advance {
                adjustment += actual_advance - advance;
            }

            adjustment -= glyph.x_offset / size * 1000.0;
            *cur_x += glyph.x_advance;
        }

        if !encoded.is_empty() {
            items.show(Str(&encoded));
        }

        items.finish();
        positioned.finish();
    }

    fn fill_stroke_glyph_run(
        &mut self,
        x: f32,
        y: f32,
        sc: &mut SerializerContext,
        text_rendering_mode: TextRenderingMode,
        action: impl FnOnce(&mut ContentBuilder, &mut SerializerContext),
        glyphs: &[Glyph],
        font: Font,
        text: &str,
    ) {
        let mut cur_x = x;

        self.apply_isolated_op(|sb| {
            action(sb, sc);
            sb.content.begin_text();
            sb.content.set_text_rendering_mode(text_rendering_mode);

            let font_container = sc.create_or_get_font_container(font.clone());

            let spanned = TextSpanner::new(glyphs, text, font_container);

            for fragment in spanned {
                if let Some(text) = fragment.actual_text() {
                    let mut actual_text = sb
                        .content
                        .begin_marked_content_with_properties(Name(b"Span"));
                    actual_text.properties().actual_text(TextStr(text));
                }

                let segmented = GlyphGrouper::new(font_container, fragment.glyphs());

                for glyph_group in segmented {
                    sb.encode_consecutive_glyph_run(
                        &mut cur_x,
                        y - glyph_group.y_offset,
                        glyph_group.font_identifier,
                        glyph_group.size,
                        &glyph_group.glyphs,
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
        let pattern_transform = |transform: Transform| -> Transform {
            transform.post_concat(self.graphics_states.cur().transform())
        };

        let mut write_gradient = |gradient_props: GradientProperties,
                                  transform: TransformWrapper| {
            let shading_mask = Mask::new_from_shading(
                gradient_props.clone(),
                transform,
                bounds,
                serializer_context,
            );

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
                let color_space = self.rd_builder.register_resource(Resource::ColorSpace(
                    Into::<Color>::into(c)
                        .color_space(serializer_context.serialize_settings.no_device_cs)
                        .into(),
                ));
                set_solid_fn(&mut self.content, color_space, &c.into());
            }
            Paint::LinearGradient(lg) => {
                let (gradient_props, transform) = lg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, transform);
            }
            Paint::RadialGradient(rg) => {
                let (gradient_props, transform) = rg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, transform);
            }
            Paint::SweepGradient(sg) => {
                let (gradient_props, transform) = sg.clone().gradient_properties(bounds);
                write_gradient(gradient_props, transform);
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

#[derive(Debug, Clone)]
pub struct Glyph {
    pub glyph_id: GlyphId,
    pub range: Range<usize>,
    pub x_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    size: f32,
}

pub struct InstanceGlyph {
    pub pdf_glyph: PDFGlyph,
    pub x_advance: f32,
    pub font_advance: Option<f32>,
    pub x_offset: f32,
}

impl Glyph {
    pub fn new(
        glyph_id: GlyphId,
        x_advance: f32,
        x_offset: f32,
        y_offset: f32,
        range: Range<usize>,
        size: f32,
    ) -> Self {
        Self {
            glyph_id,
            x_advance,
            x_offset,
            y_offset,
            range,
            size,
        }
    }
}

pub trait PdfFont {
    fn identifier(&self) -> FontIdentifier;
    fn to_font_units(&self, val: f32) -> f32;
    fn advance_width(&self, pdf_glyph: PDFGlyph) -> Option<f32>;
    fn get_codepoints(&self, pdf_glyph: PDFGlyph) -> Option<&str>;
    fn set_codepoints(&mut self, pdf_glyph: PDFGlyph, text: String);
    fn get_gid(&self, gid: GlyphId) -> Option<PDFGlyph>;
}

impl PdfFont for Type3Font {
    fn identifier(&self) -> FontIdentifier {
        self.identifier()
    }

    fn to_font_units(&self, val: f32) -> f32 {
        Type3Font::to_pdf_font_units(self, val)
    }

    fn advance_width(&self, pdf_glyph: PDFGlyph) -> Option<f32> {
        match pdf_glyph {
            PDFGlyph::Type3(t3) => self.advance_width(t3),
            PDFGlyph::CID(_) => panic!("attempted to pass cid to type 3 font"),
        }
    }

    fn get_codepoints(&self, pdf_glyph: PDFGlyph) -> Option<&str> {
        match pdf_glyph {
            PDFGlyph::Type3(t3) => self.get_codepoints(t3),
            PDFGlyph::CID(_) => panic!("attempted to pass cid to type 3 font"),
        }
    }

    fn set_codepoints(&mut self, pdf_glyph: PDFGlyph, text: String) {
        match pdf_glyph {
            PDFGlyph::Type3(t3) => self.set_codepoints(t3, text),
            PDFGlyph::CID(_) => panic!("attempted to pass cid to type 3 font"),
        }
    }

    fn get_gid(&self, gid: GlyphId) -> Option<PDFGlyph> {
        self.get_gid(gid).map(|g| PDFGlyph::Type3(g))
    }
}

impl PdfFont for CIDFont {
    fn identifier(&self) -> FontIdentifier {
        self.identifier()
    }

    fn to_font_units(&self, val: f32) -> f32 {
        CIDFont::to_pdf_font_units(self, val)
    }

    fn advance_width(&self, pdf_glyph: PDFGlyph) -> Option<f32> {
        match pdf_glyph {
            PDFGlyph::Type3(_) => panic!("attempted to pass cid to type 3 font"),
            PDFGlyph::CID(cid) => self.advance_width(cid),
        }
    }

    fn get_codepoints(&self, pdf_glyph: PDFGlyph) -> Option<&str> {
        match pdf_glyph {
            PDFGlyph::Type3(_) => panic!("attempted to pass cid to type 3 font"),
            PDFGlyph::CID(cid) => self.get_codepoints(cid),
        }
    }

    fn set_codepoints(&mut self, pdf_glyph: PDFGlyph, text: String) {
        match pdf_glyph {
            PDFGlyph::Type3(_) => panic!("attempted to pass cid to type 3 font"),
            PDFGlyph::CID(cid) => self.set_codepoints(cid, text),
        }
    }

    fn get_gid(&self, gid: GlyphId) -> Option<PDFGlyph> {
        self.get_cid(gid).map(|g| PDFGlyph::CID(g))
    }
}

pub enum TextSpan<'a> {
    Unspanned(&'a [Glyph]),
    Spanned(&'a [Glyph], &'a str),
}

impl TextSpan<'_> {
    pub fn glyphs(&self) -> &[Glyph] {
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

pub struct TextSpanner<'a, 'b> {
    slice: &'a [Glyph],
    font_container: &'b RefCell<FontContainer>,
    text: &'a str,
}

impl<'a, 'b> TextSpanner<'a, 'b> {
    pub fn new(
        slice: &'a [Glyph],
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

impl<'a> Iterator for TextSpanner<'a, '_> {
    type Item = TextSpan<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let func = |g: &Glyph| {
            let mut font_container = self.font_container.borrow_mut();
            let pdf_glyph = font_container.add_glyph(g.glyph_id);
            let font_identifier = font_container.font_identifier(g.glyph_id).unwrap();
            let pdf_font = font_container
                .get_from_identifier_mut(font_identifier.clone())
                .unwrap();

            let range = g.range.clone();
            let text = &self.text[range.clone()];
            let codepoints = pdf_font.get_codepoints(pdf_glyph);
            let incompatible_codepoint = codepoints.is_some() && codepoints != Some(text);

            if !incompatible_codepoint {
                pdf_font.set_codepoints(pdf_glyph, text.to_string());
            }

            (range, incompatible_codepoint)
        };

        let mut use_span = None;
        let mut count = 1;

        let mut iter = self.slice.iter();
        let (first_range, first_incompatible) = (func)(iter.next()?);

        let mut last_range = first_range.clone();

        while let Some(next) = iter.next() {
            let (next_range, next_incompatible) = func(next);

            match use_span {
                None => {
                    if first_incompatible {
                        use_span = Some(true);
                        break;
                    }

                    use_span = Some(last_range == next_range);
                }
                Some(true) => {
                    if next_incompatible || last_range != next_range {
                        break;
                    }
                }
                Some(false) => {
                    if next_incompatible {
                        break;
                    }

                    if last_range == next_range {
                        count -= 1;
                        break;
                    }
                }
            }

            last_range = next.range.clone();
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

pub struct GlyphGroup {
    font_identifier: FontIdentifier,
    glyphs: Vec<InstanceGlyph>,
    size: f32,
    y_offset: f32,
}

impl GlyphGroup {
    pub fn new(
        font_identifier: FontIdentifier,
        glyphs: Vec<InstanceGlyph>,
        size: f32,
        y_offset: f32,
    ) -> Self {
        GlyphGroup {
            font_identifier,
            glyphs,
            size,
            y_offset,
        }
    }
}

pub struct GlyphGrouper<'a, 'b> {
    font_container: &'b RefCell<FontContainer>,
    slice: &'a [Glyph],
}

impl<'a, 'b> GlyphGrouper<'a, 'b> {
    pub fn new(font_container: &'b RefCell<FontContainer>, slice: &'a [Glyph]) -> Self {
        Self {
            font_container,
            slice,
        }
    }
}

impl<'a> Iterator for GlyphGrouper<'a, '_> {
    type Item = GlyphGroup;

    fn next(&mut self) -> Option<Self::Item> {
        let (head, tail, first) = {
            struct GlyphProps {
                font_identifier: FontIdentifier,
                size: f32,
                y_offset: f32,
            }

            let func = |g: &Glyph| {
                let font_container = self.font_container.borrow_mut();
                let font_identifier = font_container.font_identifier(g.glyph_id).unwrap();

                GlyphProps {
                    font_identifier,
                    size: g.size,
                    y_offset: g.y_offset,
                }
            };

            let mut count = 1;

            let mut iter = self.slice.iter();
            let first = (func)(iter.next()?);

            while let Some(next) = iter.next() {
                let temp_glyph = func(next);

                if first.font_identifier != temp_glyph.font_identifier
                    || first.y_offset != temp_glyph.y_offset
                    || first.size != temp_glyph.size
                {
                    break;
                }

                count += 1;
            }

            let (head, tail) = self.slice.split_at(count);
            (head, tail, first)
        };

        self.slice = tail;

        let glyphs = head
            .iter()
            .map(move |g| {
                let mut font_container = self.font_container.borrow_mut();
                let font_identifier = font_container.font_identifier(g.glyph_id).unwrap();
                let pdf_font = font_container
                    .get_from_identifier_mut(font_identifier.clone())
                    .unwrap();

                let pdf_glyph = pdf_font.get_gid(g.glyph_id).unwrap();

                InstanceGlyph {
                    pdf_glyph,
                    x_advance: g.x_advance,
                    font_advance: pdf_font.advance_width(pdf_glyph),
                    x_offset: g.x_offset,
                }
            })
            .collect::<Vec<_>>();

        let glyph_group =
            GlyphGroup::new(first.font_identifier, glyphs, first.size, first.y_offset);

        Some(glyph_group)
    }
}
