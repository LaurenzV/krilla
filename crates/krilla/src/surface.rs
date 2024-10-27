//! Drawing onto a surface.
//!
//! This module contains most core part of krilla: The [`Surface`] struct. A surface
//! represents a drawing area on which you can define the contents of your page. This includes
//! operations such as applying linear transformations,
//! showing text or images and drawing paths.

use crate::content::{unit_normalize, ContentBuilder};
use crate::font::{draw_glyph, Font, Glyph, GlyphUnits, KrillaGlyph, PaintMode};
#[cfg(feature = "raster-images")]
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_function::ShadingFunction;
use crate::path::{Fill, FillRule, Stroke};
use crate::serialize::SerializerContext;
use crate::stream::{Stream, StreamBuilder};
#[cfg(feature = "svg")]
use crate::svg;
use crate::tagging::{ContentTag, Identifier, PageTagIdentifier};
use crate::util::RectExt;
use crate::SvgSettings;
#[cfg(feature = "fontdb")]
use fontdb::{Database, ID};
#[cfg(feature = "simple-text")]
use rustybuzz::ttf_parser::Tag;
#[cfg(feature = "simple-text")]
use rustybuzz::{Direction, Feature, UnicodeBuffer};
#[cfg(feature = "simple-text")]
use skrifa::GlyphId;
#[cfg(feature = "fontdb")]
use std::collections::HashMap;
#[cfg(feature = "raster-images")]
use tiny_skia_path::Size;
use tiny_skia_path::{NormalizedF32, Rect};
use tiny_skia_path::{Path, Point, Transform};

pub use pdf_writer::types::BlendMode;

pub(crate) enum PushInstruction {
    Transform,
    Opacity(NormalizedF32),
    ClipPath,
    BlendMode,
    Mask(Box<Mask>),
    Isolated,
}

/// A surface.
///
/// Represents a drawing area for defining graphical content. The origin of the
/// coordinate axis is in the top-left corner.
///
/// You cannot directly create an instance of a [`Surface`] yourself.
/// Instead, there are two ways of getting access to a surface, which you can then use to draw on:
///
/// - The first way, and also the most common one you will use, is by creating a new document,
///   adding a page to it and then invoking the [`Page::surface`] method. The surface returned as part of
///   that represents the drawing area of the page.
/// - The second way is by calling the [`Surface::stream_builder`] method on the surface, to create a sub-drawing
///   context. See the documentation of the [`stream`] module for more information.
///
/// The surface uses a `push` and `pop` based mechanism for applying certain actions. For example,
/// there is a [`Surface::push_transform`] method which allows you to concatenate a new transform to the
/// current transform matrix. There is also a [`Surface::push_clip_path`] method, which allows you to
/// intersect the current drawing area with a clip path. Once you call such a `push` method,
/// the action that it invokes will be in place until you call the `pop` method, which will then
/// revert the last `push` operation. This allows you to, for example, define a clipping path area
/// or a mask to use only for certain objects.
///
/// It is important that, for each `push` method you invoke, there must be a corresponding `pop`
/// invocation that reverts it. If, at the end of the using the surface, the number of pushes/pops is
/// uneven, the program will panic.
///
/// [`stream`]: crate::stream
/// [`Page::surface`]: crate::page::Page::surface
pub struct Surface<'a> {
    sc: &'a mut SerializerContext,
    pub(crate) root_builder: ContentBuilder,
    sub_builders: Vec<ContentBuilder>,
    push_instructions: Vec<PushInstruction>,
    page_identifier: Option<PageTagIdentifier>,
    finish_fn: Box<dyn FnMut(Stream, i32) + 'a>,
}

impl<'a> Surface<'a> {
    pub(crate) fn new(
        sc: &'a mut SerializerContext,
        root_builder: ContentBuilder,
        page_identifier: Option<PageTagIdentifier>,
        finish_fn: Box<dyn FnMut(Stream, i32) + 'a>,
    ) -> Surface<'a> {
        Self {
            sc,
            root_builder,
            page_identifier,
            sub_builders: vec![],
            push_instructions: vec![],
            finish_fn,
        }
    }

    /// Return a `StreamBuilder` to allow drawing on a sub-context.
    pub fn stream_builder(&mut self) -> StreamBuilder {
        StreamBuilder::new(self.sc)
    }

    /// Fill a path.
    pub fn fill_path(&mut self, path: &Path, fill: Fill) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_path(path, fill, self.sc);
    }

    pub(crate) fn fill_path_impl(&mut self, path: &Path, fill: Fill, fill_props: bool) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_path_impl(path, fill, self.sc, fill_props)
    }

    /// Stroke a path.
    pub fn stroke_path(&mut self, path: &Path, stroke: Stroke) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .stroke_path(path, stroke, self.sc)
    }

    /// Start a new tagged content section.
    ///
    /// # Panics
    /// Panics if a tagged section has already been started.
    pub fn start_tagged(&mut self, tag: ContentTag) -> Identifier {
        if let Some(id) = &mut self.page_identifier {
            match tag {
                // An artifact is actually not really part of tagged PDF and doesn't have
                // a marked content identifier, so we need to return a dummy one here. It's just
                // the API of krilla that conflates artifacts with tagged content,
                // for the sake of simplicity. But the user of the library does not need to know
                // about this.
                ContentTag::Artifact(at) => {
                    if at.requires_properties() {
                        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                            .start_marked_content_with_properties(self.sc, None, tag);
                    } else {
                        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                            .start_marked_content(tag.name());
                    }

                    Identifier::dummy()
                }
                ContentTag::Span(_, _, _, _) | ContentTag::Other => {
                    Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                        .start_marked_content_with_properties(self.sc, Some(id.mcid), tag);
                    id.bump().into()
                }
            }
        } else {
            Identifier::dummy()
        }
    }

    /// End the current tagged section.
    ///
    /// # Panics
    /// Panics if no tagged section has been started.
    pub fn end_tagged(&mut self) {
        if self.page_identifier.is_some() {
            Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).end_marked_content();
        }
    }

    // It's very unfortunate that we have this method at the `Surface` level,
    // but it's only used in one place and should not be needed to be used anywhere
    // else.
    pub(crate) fn start_shape_glyph(
        &mut self,
        wx: f32,
        ll_x: f32,
        ll_y: f32,
        ur_x: f32,
        ur_y: f32,
    ) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .content_start_shape_glyph(wx, ll_x, ll_y, ur_x, ur_y);
    }

    fn outline_glyphs(
        &mut self,
        glyphs: &[impl Glyph],
        start: Point,
        font: Font,
        font_size: f32,
        glyph_units: GlyphUnits,
        paint_mode: PaintMode,
    ) {
        let normalize = |val| unit_normalize(glyph_units, font.units_per_em(), font_size, val);
        let (mut cur_x, y) = (start.x, start.y);

        for glyph in glyphs {
            let mut base_transform = Transform::from_translate(
                cur_x + normalize(glyph.x_offset()) * font_size,
                y - normalize(glyph.y_offset()) * font_size,
            );
            base_transform = base_transform.pre_concat(Transform::from_scale(
                font_size / font.units_per_em(),
                -font_size / font.units_per_em(),
            ));
            draw_glyph(
                font.clone(),
                SvgSettings::default(),
                glyph.glyph_id(),
                paint_mode,
                base_transform,
                self,
            );

            cur_x += normalize(glyph.x_advance()) * font_size;
        }
    }

    /// Draw a sequence of glyphs with a fill.
    ///
    /// This is a very low-level method, which gives you full control over how to place
    /// the glyphs that make up the text. This means that you must have your own text processing
    /// logic for dealing with bidirectional text, font fallback, text layouting, etc.
    #[allow(clippy::too_many_arguments)]
    pub fn fill_glyphs(
        &mut self,
        start: Point,
        fill: Fill,
        glyphs: &[impl Glyph],
        font: Font,
        text: &str,
        font_size: f32,
        glyph_units: GlyphUnits,
        outlined: bool,
    ) {
        if outlined {
            self.outline_glyphs(
                glyphs,
                start,
                font,
                font_size,
                glyph_units,
                PaintMode::Fill(&fill),
            );
        } else {
            Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).fill_glyphs(
                start,
                self.sc,
                fill,
                glyphs,
                font,
                text,
                font_size,
                glyph_units,
            );
        }
    }

    /// Draw some text with a fill.
    ///
    /// This is a high-level method which allows you to just provide some text, which will
    /// then be rendered into a single line. However, this approach has restrictions:
    ///
    /// - It will not perform BIDI resolution and only supports a single script, meaning that you
    ///   must ensure that your text does not contain multiple scripts.
    /// - It will only use the single font you provided to draw the text, no font fallback will
    ///   be performed.
    ///
    /// If you need more advanced control over how your text looks, but you don't want to
    /// implement your own text processing solution, so you can use the `fill_glyphs` method,
    /// you can use the `cosmic-text` integration to do so.
    #[cfg(feature = "simple-text")]
    #[allow(clippy::too_many_arguments)]
    pub fn fill_text(
        &mut self,
        start: Point,
        fill: Fill,
        font: Font,
        font_size: f32,
        features: &[Feature],
        text: &str,
        outlined: bool,
        direction: TextDirection,
    ) {
        let glyphs = naive_shape(text, font.clone(), features, font_size, direction);

        self.fill_glyphs(
            start,
            fill,
            &glyphs,
            font,
            text,
            font_size,
            GlyphUnits::UserSpace,
            outlined,
        );
    }

    /// Draw a sequence of glyphs with a stroke.
    ///
    /// This is a very low-level method, which gives you full control over how to place
    /// the glyphs that make up the text. This means that you must have your own text processing
    /// you can use a text-layouting library like `cosmic-text` or `parley` to do so.
    #[allow(clippy::too_many_arguments)]
    pub fn stroke_glyphs(
        &mut self,
        start: Point,
        stroke: Stroke,
        glyphs: &[impl Glyph],
        font: Font,
        text: &str,
        font_size: f32,
        glyph_units: GlyphUnits,
        outlined: bool,
    ) {
        if outlined {
            self.outline_glyphs(
                glyphs,
                start,
                font,
                font_size,
                glyph_units,
                PaintMode::Stroke(&stroke),
            );
        } else {
            Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).stroke_glyphs(
                start,
                self.sc,
                stroke,
                glyphs,
                font,
                text,
                font_size,
                glyph_units,
            );
        }
    }

    /// Draw some text with a stroke.
    ///
    /// This is a high-level method which allows you to just provide some text, which will
    /// then be rendered into a single line. However, this approach has restrictions:
    ///
    /// - It will not perform BIDI resolution and only supports a single script, meaning that you
    ///   must ensure that your text does not contain multiple scripts.
    /// - It will only use the single font you provided to draw the text, no font fallback will
    ///   be performed.
    ///
    /// If you need more advanced control over how your text looks, but you don't want to
    /// implement your own text processing solution, so you can use the `stroke_glyphs` method,
    /// you can use a text-layouting library like `cosmic-text` or `parley` to do so.
    #[cfg(feature = "simple-text")]
    #[allow(clippy::too_many_arguments)]
    pub fn stroke_text(
        &mut self,
        start: Point,
        stroke: Stroke,
        font: Font,
        font_size: f32,
        features: &[Feature],
        text: &str,
        outlined: bool,
        direction: TextDirection,
    ) {
        let glyphs = naive_shape(text, font.clone(), features, font_size, direction);

        self.stroke_glyphs(
            start,
            stroke,
            &glyphs,
            font,
            text,
            font_size,
            GlyphUnits::UserSpace,
            outlined,
        );
    }

    /// Concatenate a new transform to the current transformation matrix.
    pub fn push_transform(&mut self, transform: &Transform) {
        self.push_instructions.push(PushInstruction::Transform);
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).save_graphics_state();
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .concat_transform(transform);
    }

    /// Push a new blend mode.
    pub fn push_blend_mode(&mut self, blend_mode: BlendMode) {
        self.push_instructions.push(PushInstruction::BlendMode);
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).save_graphics_state();
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .set_blend_mode(blend_mode);
    }

    /// Push a new clip path.
    pub fn push_clip_path(&mut self, path: &Path, clip_rule: &FillRule) {
        self.push_instructions.push(PushInstruction::ClipPath);
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .push_clip_path(path, clip_rule);
    }

    /// Push a new mask.
    pub fn push_mask(&mut self, mask: Mask) {
        self.push_instructions
            .push(PushInstruction::Mask(Box::new(mask)));
        self.sub_builders
            .push(ContentBuilder::new(Transform::identity()));
    }

    /// Push a new opacity, meaning that each subsequent graphics object will be
    /// rendered with a base opacity.
    ///
    /// This stacks, meaning that if you do `push_opacity(0.5)` twice, the resulting
    /// base opacity will be 0.25.
    pub fn push_opacity(&mut self, opacity: NormalizedF32) {
        self.push_instructions
            .push(PushInstruction::Opacity(opacity));

        if opacity != NormalizedF32::ONE {
            self.sub_builders
                .push(ContentBuilder::new(Transform::identity()));
        }
    }

    /// Push a new isolated layer.
    pub fn push_isolated(&mut self) {
        self.push_instructions.push(PushInstruction::Isolated);
        self.sub_builders
            .push(ContentBuilder::new(Transform::identity()));
    }

    /// Pop the last `push` instruction.
    ///
    /// # Panics
    /// Panics if the there wasn't a corresponding `push` to the `pop`.
    pub fn pop(&mut self) {
        match self.push_instructions.pop().unwrap() {
            PushInstruction::Transform => {
                Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                    .restore_graphics_state()
            }
            PushInstruction::Opacity(o) => {
                if o != NormalizedF32::ONE {
                    let stream = self.sub_builders.pop().unwrap().finish(self.sc);
                    Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                        .draw_opacified(self.sc, o, stream);
                }
            }
            PushInstruction::ClipPath => {
                Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).pop_clip_path()
            }
            PushInstruction::BlendMode => {
                Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                    .restore_graphics_state()
            }
            PushInstruction::Mask(mask) => {
                let stream = self.sub_builders.pop().unwrap().finish(self.sc);
                Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                    .draw_masked(self.sc, *mask, stream)
            }
            PushInstruction::Isolated => {
                let stream = self.sub_builders.pop().unwrap().finish(self.sc);
                Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                    .draw_isolated(self.sc, stream);
            }
        }
    }

    #[cfg(feature = "raster-images")]
    /// Draw a new bitmap image.
    pub fn draw_image(&mut self, image: Image, size: Size) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .draw_image(image, size, self.sc);
    }

    #[cfg(feature = "svg")]
    /// Draw a new SVG image.
    pub fn draw_svg(
        &mut self,
        tree: &usvg::Tree,
        size: Size,
        svg_settings: SvgSettings,
    ) -> Option<()> {
        let transform = Transform::from_scale(
            size.width() / tree.size().width(),
            size.height() / tree.size().height(),
        );
        self.push_transform(&transform);
        self.push_clip_path(
            &Rect::from_xywh(0.0, 0.0, tree.size().width(), tree.size().height())
                .unwrap()
                .to_clip_path(),
            &FillRule::NonZero,
        );
        svg::render_tree(tree, svg_settings, self);
        self.pop();
        self.pop();

        Some(())
    }

    pub(crate) fn draw_shading(&mut self, shading: &ShadingFunction) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .draw_shading(shading, self.sc);
    }

    /// Convert a `fontdb` into `krilla` `Font` objects. This is a convenience method,
    /// which makes it easier to integrate `cosmic-text` with this library.
    #[cfg(feature = "fontdb")]
    pub fn convert_fontdb(&mut self, db: &mut Database, ids: Option<Vec<ID>>) -> HashMap<ID, Font> {
        self.sc.convert_fontdb(db, ids)
    }

    /// A convenience method for `std::mem::drop`.
    ///
    /// # Panics
    /// Panics if the push/pop difference is not 0.
    pub fn finish(self) {}

    pub(crate) fn draw_opacified_stream(&mut self, opacity: NormalizedF32, stream: Stream) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .draw_opacified(self.sc, opacity, stream)
    }

    fn cur_builder<'b>(
        root_builder: &'b mut ContentBuilder,
        sub_builders: &'b mut [ContentBuilder],
    ) -> &'b mut ContentBuilder {
        sub_builders.last_mut().unwrap_or(root_builder)
    }
}

impl Drop for Surface<'_> {
    fn drop(&mut self) {
        let root_builder = std::mem::replace(
            &mut self.root_builder,
            ContentBuilder::new(Transform::identity()),
        );
        let num_mcids = match self.page_identifier {
            Some(pi) => pi.mcid,
            None => 0,
        };
        assert!(self.sub_builders.is_empty());
        assert!(self.push_instructions.is_empty());
        assert!(!root_builder.active_marked_content);
        (self.finish_fn)(root_builder.finish(self.sc), num_mcids)
    }
}

#[cfg(feature = "simple-text")]
/// The direction of a text.
pub enum TextDirection {
    /// Determine the direction automatically.
    Auto,
    /// Left to right.
    LeftToRight,
    /// Right to left.
    RightToLeft,
    /// Top to bottom.
    TopToBottom,
    /// Bottom to top.
    BottomToTop,
}

/// Shape some text with a single font.
#[cfg(feature = "simple-text")]
fn naive_shape(
    text: &str,
    font: Font,
    features: &[Feature],
    size: f32,
    direction: TextDirection,
) -> Vec<KrillaGlyph> {
    let data = font.font_data();
    let mut rb_font = rustybuzz::Face::from_slice(data.as_ref().as_ref(), font.index()).unwrap();
    for (tag, val) in font.variations() {
        rb_font.set_variation(Tag::from_bytes_lossy(tag.as_bytes()), val);
    }

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();

    match direction {
        TextDirection::LeftToRight => buffer.set_direction(Direction::LeftToRight),
        TextDirection::RightToLeft => buffer.set_direction(Direction::RightToLeft),
        TextDirection::TopToBottom => buffer.set_direction(Direction::TopToBottom),
        TextDirection::BottomToTop => buffer.set_direction(Direction::BottomToTop),
        TextDirection::Auto => {}
    }

    let dir = buffer.direction();

    let output = rustybuzz::shape(&rb_font, features, buffer);

    let positions = output.glyph_positions();
    let infos = output.glyph_infos();

    let mut glyphs = vec![];

    for i in 0..output.len() {
        let pos = positions[i];
        let start_info = infos[i];

        let start = start_info.cluster as usize;

        let end = if dir == Direction::LeftToRight || dir == Direction::TopToBottom {
            let mut e = i.checked_add(1);
            loop {
                if let Some(index) = e {
                    if let Some(end_info) = infos.get(index) {
                        if end_info.cluster == start_info.cluster {
                            e = index.checked_add(1);
                            continue;
                        }
                    }
                }

                break;
            }

            e
        } else {
            let mut e = i.checked_sub(1);
            while let Some(index) = e {
                if let Some(end_info) = infos.get(index) {
                    if end_info.cluster == start_info.cluster {
                        e = index.checked_sub(1);
                    } else {
                        break;
                    }
                }
            }

            e
        }
        .and_then(|last| infos.get(last))
        .map_or(text.len(), |info| info.cluster as usize);

        glyphs.push(KrillaGlyph::new(
            GlyphId::new(start_info.glyph_id),
            (pos.x_advance as f32 / font.units_per_em()) * size,
            (pos.x_offset as f32 / font.units_per_em()) * size,
            (pos.y_offset as f32 / font.units_per_em()) * size,
            (pos.y_advance as f32 / font.units_per_em()) * size,
            start..end,
        ));
    }

    glyphs
}

#[cfg(test)]
mod tests {
    use crate::font::Font;
    use crate::mask::MaskType;
    use crate::paint::{LinearGradient, Paint, SpreadMethod};
    use crate::path::Fill;
    use crate::surface::Surface;
    use crate::surface::{Stroke, TextDirection};
    use crate::tests::{
        basic_mask, blue_fill, blue_stroke, cmyk_fill, gray_fill, green_fill, load_png_image,
        rect_to_path, red_fill, red_stroke, stops_with_3_solid_1, FONTDB, NOTO_COLOR_EMOJI_COLR,
        NOTO_SANS, NOTO_SANS_CJK, NOTO_SANS_DEVANAGARI, SVGS_PATH,
    };
    use crate::SvgSettings;
    use krilla_macros::{snapshot, visreg};
    use pdf_writer::types::BlendMode;
    use tiny_skia_path::{Point, Size, Transform};

    #[visreg]
    fn text_direction_ltr(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS_CJK.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            20.0,
            &[],
            "你好这是一段则是文字",
            false,
            TextDirection::LeftToRight,
        );
    }

    #[visreg]
    fn text_direction_rtl(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS_CJK.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            20.0,
            &[],
            "你好这是一段则是文字",
            false,
            TextDirection::RightToLeft,
        );
    }

    #[visreg]
    fn text_direction_ttb(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS_CJK.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(100.0, 0.0),
            Fill::default(),
            font,
            20.0,
            &[],
            "你好这是一段则是文字",
            false,
            TextDirection::TopToBottom,
        );
    }

    #[visreg]
    fn text_direction_btt(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS_CJK.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(100.0, 0.0),
            Fill::default(),
            font,
            20.0,
            &[],
            "你好这是一段则是文字",
            false,
            TextDirection::BottomToTop,
        );
    }

    #[snapshot(stream)]
    fn stream_path_single_with_rgb(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let fill = red_fill(1.0);
        surface.fill_path(&path, fill);
    }

    #[snapshot(stream)]
    fn stream_path_single_with_luma(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let fill = gray_fill(1.0);
        surface.fill_path(&path, fill);
    }

    #[snapshot(stream)]
    fn stream_path_single_with_rgb_and_opacity(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let fill = red_fill(0.5);
        surface.fill_path(&path, fill);
    }

    #[snapshot(stream)]
    fn stream_path_single_with_cmyk(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let fill = cmyk_fill(1.0);
        surface.fill_path(&path, fill);
    }

    #[snapshot(stream, settings_2)]
    fn stream_resource_cache(surface: &mut Surface) {
        let path1 = rect_to_path(0.0, 0.0, 100.0, 100.0);
        let path2 = rect_to_path(50.0, 50.0, 150.0, 150.0);
        let path3 = rect_to_path(100.0, 100.0, 200.0, 200.0);

        surface.fill_path(&path1, green_fill(1.0));
        surface.fill_path(&path2, red_fill(1.0));
        surface.fill_path(&path3, blue_fill(1.0));
    }

    #[snapshot(stream)]
    fn stream_nested_transforms(surface: &mut Surface) {
        let path1 = rect_to_path(0.0, 0.0, 100.0, 100.0);

        surface.push_transform(&Transform::from_translate(50.0, 50.0));
        surface.fill_path(&path1, green_fill(1.0));
        surface.push_transform(&Transform::from_translate(100.0, 100.0));
        surface.fill_path(&path1, red_fill(1.0));

        surface.pop();
        surface.pop();
    }

    #[snapshot(stream)]
    fn stream_reused_graphics_state(surface: &mut Surface) {
        let path1 = rect_to_path(0.0, 0.0, 100.0, 100.0);
        surface.fill_path(&path1, green_fill(0.5));
        surface.push_blend_mode(BlendMode::ColorBurn);
        surface.fill_path(&path1, green_fill(0.5));
        surface.pop();
        surface.fill_path(&path1, green_fill(0.5));
    }

    #[snapshot(stream)]
    fn stream_fill_text(surface: &mut Surface) {
        surface.fill_text(
            Point::from_xy(0.0, 50.0),
            Fill::default(),
            Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap(),
            16.0,
            &[],
            "hi there",
            false,
            TextDirection::Auto,
        );
    }

    #[snapshot(stream)]
    fn stream_stroke_text(surface: &mut Surface) {
        surface.stroke_text(
            Point::from_xy(0.0, 50.0),
            Stroke::default(),
            Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap(),
            16.0,
            &[],
            "hi there",
            false,
            TextDirection::Auto,
        );
    }

    #[snapshot(stream)]
    fn stream_complex_text(surface: &mut Surface) {
        surface.fill_text(
            Point::from_xy(0.0, 50.0),
            Fill::default(),
            Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, vec![]).unwrap(),
            16.0,
            &[],
            "यह कुछ जटिल पाठ है.",
            false,
            TextDirection::Auto,
        );
    }

    #[snapshot(stream)]
    fn stream_complex_text_2(surface: &mut Surface) {
        surface.fill_text(
            Point::from_xy(0.0, 50.0),
            Fill::default(),
            Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, vec![]).unwrap(),
            16.0,
            &[],
            "यु॒धा नर॑ ऋ॒ष्वा ",
            false,
            TextDirection::Auto,
        );
    }

    #[snapshot(stream)]
    fn stream_complex_text_3(surface: &mut Surface) {
        surface.fill_text(
            Point::from_xy(0.0, 50.0),
            Fill::default(),
            Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, vec![]).unwrap(),
            16.0,
            &[],
            "आ रु॒क्मैरा यु॒धा नर॑ ऋ॒ष्वा ऋ॒ष्टीर॑सृक्षत ।",
            false,
            TextDirection::Auto,
        );
    }

    #[snapshot(stream)]
    fn stream_complex_text_4(surface: &mut Surface) {
        surface.fill_text(
            Point::from_xy(0.0, 50.0),
            Fill::default(),
            Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, vec![]).unwrap(),
            16.0,
            &[],
            "अन्वे॑नाँ॒ अह॑ वि॒द्युतो॑ म॒रुतो॒ जज्झ॑तीरव भनर॑र्त॒ त्मना॑ दि॒वः ॥",
            false,
            TextDirection::Auto,
        );
    }

    #[snapshot(stream)]
    fn stream_image(surface: &mut Surface) {
        let image = load_png_image("rgb8.png");
        let size = image.size();
        surface.draw_image(image, size);
    }

    #[snapshot(stream)]
    fn stream_mask(surface: &mut Surface) {
        let mask = basic_mask(surface, MaskType::Alpha);
        surface.push_mask(mask);
        let path = rect_to_path(0.0, 0.0, 100.0, 100.0);
        surface.fill_path(&path, green_fill(0.5));
        surface.pop();
    }

    pub(crate) fn sample_svg() -> usvg::Tree {
        let data = std::fs::read(SVGS_PATH.join("resvg_masking_mask_with_opacity_1.svg")).unwrap();
        usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap()
    }

    #[visreg]
    fn svg_simple(surface: &mut Surface) {
        let tree = sample_svg();
        surface.draw_svg(&tree, tree.size(), SvgSettings::default());
    }

    #[visreg]
    fn svg_outlined_text(surface: &mut Surface) {
        let data = std::fs::read(SVGS_PATH.join("resvg_text_text_simple_case.svg")).unwrap();
        let tree = usvg::Tree::from_data(
            &data,
            &usvg::Options {
                fontdb: FONTDB.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let settings = SvgSettings {
            embed_text: false,
            ..Default::default()
        };
        surface.draw_svg(&tree, tree.size(), settings);
    }

    #[visreg]
    fn svg_resized(surface: &mut Surface) {
        surface.draw_svg(
            &sample_svg(),
            Size::from_wh(120.0, 80.0).unwrap(),
            SvgSettings::default(),
        );
    }

    #[visreg]
    fn svg_should_be_clipped(surface: &mut Surface) {
        let data =
            std::fs::read(SVGS_PATH.join("custom_paint_servers_pattern_patterns_2.svg")).unwrap();
        let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();

        surface.push_transform(&Transform::from_translate(100.0, 0.0));
        surface.draw_svg(&tree, tree.size(), SvgSettings::default());
        surface.pop();
    }

    fn text_gradient(spread_method: SpreadMethod) -> LinearGradient {
        LinearGradient {
            x1: 50.0,
            y1: 0.0,
            x2: 150.0,
            y2: 0.0,
            transform: Default::default(),
            spread_method,
            stops: stops_with_3_solid_1(),
        }
    }

    fn text_with_fill_impl(surface: &mut Surface, outlined: bool) {
        let font = Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 80.0),
            red_fill(0.5),
            font.clone(),
            20.0,
            &[],
            "red outlined text",
            outlined,
            TextDirection::Auto,
        );

        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            blue_fill(0.8),
            font.clone(),
            20.0,
            &[],
            "blue outlined text",
            outlined,
            TextDirection::Auto,
        );

        let grad_fill = Fill {
            paint: Paint::from(text_gradient(SpreadMethod::Pad)),
            ..Default::default()
        };

        surface.fill_text(
            Point::from_xy(0.0, 120.0),
            grad_fill,
            font.clone(),
            20.0,
            &[],
            "gradient text",
            outlined,
            TextDirection::Auto,
        );

        let noto_font = Font::new(NOTO_COLOR_EMOJI_COLR.clone(), 0, vec![]).unwrap();

        surface.fill_text(
            Point::from_xy(0.0, 140.0),
            blue_fill(0.8),
            noto_font.clone(),
            20.0,
            &[],
            "😄😁😆",
            outlined,
            TextDirection::Auto,
        );

        let grad_fill = Fill {
            paint: Paint::from(text_gradient(SpreadMethod::Reflect)),
            ..Default::default()
        };

        surface.fill_text(
            Point::from_xy(0.0, 160.0),
            grad_fill,
            font,
            20.0,
            &[],
            "longer gradient text with repeat",
            outlined,
            TextDirection::Auto,
        );
    }

    #[visreg]
    fn text_outlined_with_fill(surface: &mut Surface) {
        text_with_fill_impl(surface, true)
    }

    #[visreg(settings_4, all)]
    fn text_type3_with_fill(surface: &mut Surface) {
        text_with_fill_impl(surface, false)
    }

    fn text_with_stroke_impl(surface: &mut Surface, outlined: bool) {
        let font = Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap();
        surface.stroke_text(
            Point::from_xy(0.0, 80.0),
            red_stroke(0.5, 1.0),
            font.clone(),
            20.0,
            &[],
            "red outlined text",
            outlined,
            TextDirection::Auto,
        );

        surface.stroke_text(
            Point::from_xy(0.0, 100.0),
            blue_stroke(0.8),
            font.clone(),
            20.0,
            &[],
            "blue outlined text",
            outlined,
            TextDirection::Auto,
        );

        let grad_stroke = Stroke {
            paint: Paint::from(text_gradient(SpreadMethod::Pad)),
            ..Default::default()
        };

        surface.stroke_text(
            Point::from_xy(0.0, 120.0),
            grad_stroke,
            font,
            20.0,
            &[],
            "gradient text",
            outlined,
            TextDirection::Auto,
        );

        let font = Font::new(NOTO_COLOR_EMOJI_COLR.clone(), 0, vec![]).unwrap();

        surface.stroke_text(
            Point::from_xy(0.0, 140.0),
            blue_stroke(0.8),
            font,
            20.0,
            &[],
            "😄😁😆",
            outlined,
            TextDirection::Auto,
        );
    }

    #[visreg]
    fn text_outlined_with_stroke(surface: &mut Surface) {
        text_with_stroke_impl(surface, true);
    }

    // This test does not work correctly. Stroking is unfortunately
    // very tricky to get to work properly with Type3 fonts.
    #[visreg(all, settings_4)]
    fn text_type3_with_stroke(surface: &mut Surface) {
        text_with_stroke_impl(surface, false)
    }

    #[visreg]
    fn text_zalgo(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            32.0,
            &[],
            "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔̦͆̏̓͢",
            false,
            TextDirection::Auto,
        );
    }

    #[visreg]
    fn text_zalgo_outlined(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            32.0,
            &[],
            "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔̦͆̏̓͢",
            true,
            TextDirection::Auto,
        );
    }
}
