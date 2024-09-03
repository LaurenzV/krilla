//! Drawing onto a surface.
//!
//! This module contains most core part of krilla: The [`Surface`] struct. A surface
//! represents a drawing area on which you can define the contents of your page. This includes
//! operations such as applying linear transformations,
//! showing text or images and drawing paths.

use crate::content::ContentBuilder;
use crate::font::{Font, Glyph};
use crate::object::color::ColorSpace;
#[cfg(feature = "raster-images")]
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_function::ShadingFunction;
use crate::path::{Fill, FillRule, Stroke};
use crate::serialize::SerializerContext;
use crate::stream::{Stream, StreamBuilder};
#[cfg(feature = "svg")]
use crate::svg;
#[cfg(feature = "fontdb")]
use fontdb::{Database, ID};
use pdf_writer::types::BlendMode;
#[cfg(feature = "simple-text")]
use rustybuzz::ttf_parser::Tag;
#[cfg(feature = "simple-text")]
use rustybuzz::{Direction, Feature, UnicodeBuffer};
#[cfg(feature = "simple-text")]
use skrifa::GlyphId;
#[cfg(feature = "fontdb")]
use std::collections::HashMap;
use tiny_skia_path::{NormalizedF32, Rect};
#[cfg(feature = "raster-images")]
use tiny_skia_path::Size;
use tiny_skia_path::{Path, Point, Transform};
use crate::util::RectExt;

pub(crate) enum PushInstruction {
    Transform,
    Opacity(NormalizedF32),
    ClipPath,
    BlendMode,
    Mask(Mask),
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
    finish_fn: Box<dyn FnMut(Stream) + 'a>,
}

impl<'a> Surface<'a> {
    pub(crate) fn new(
        sc: &'a mut SerializerContext,
        root_builder: ContentBuilder,
        finish_fn: Box<dyn FnMut(Stream) + 'a>,
    ) -> Surface<'a> {
        Self {
            sc,
            root_builder,
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
    pub fn fill_path<T>(&mut self, path: &Path, fill: Fill<T>)
    where
        T: ColorSpace,
    {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_path(path, fill, self.sc);
    }

    /// Stroke a path.
    pub fn stroke_path<T>(&mut self, path: &Path, stroke: Stroke<T>) -> Option<()>
    where
        T: ColorSpace,
    {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .stroke_path(path, stroke, self.sc)
    }

    /// Draw a sequence of glyphs with a fill.
    ///
    /// This is a very low-level method, which gives you full control over how to place
    /// the glyphs that make up the text. This means that you must have your own text processing
    /// logic for dealing with bidirectional text, font fallback, text layouting, etc.
    pub fn fill_glyphs<T>(
        &mut self,
        start: Point,
        fill: Fill<T>,
        glyphs: &[Glyph],
        font: Font,
        text: &str,
        font_size: f32,
    ) where
        T: ColorSpace,
    {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_glyphs(start, self.sc, fill, glyphs, font, text, font_size);
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
    pub fn fill_text<T>(
        &mut self,
        start: Point,
        fill: Fill<T>,
        font: Font,
        font_size: f32,
        features: &[Feature],
        text: &str,
    ) where
        T: ColorSpace,
    {
        let glyphs = naive_shape(text, font.clone(), features, font_size);

        self.fill_glyphs(start, fill, &glyphs, font, text, font_size);
    }

    /// Draw a sequence of glyphs with a stroke.
    ///
    /// This is a very low-level method, which gives you full control over how to place
    /// the glyphs that make up the text. This means that you must have your own text processing
    /// you can use a text-layouting library like `cosmic-text` or `parley` to do so.
    pub fn stroke_glyphs<T>(
        &mut self,
        start: Point,
        stroke: Stroke<T>,
        glyphs: &[Glyph],
        font: Font,
        text: &str,
        font_size: f32
    ) where
        T: ColorSpace,
    {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .stroke_glyphs(start, self.sc, stroke, glyphs, font, text, font_size);
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
    pub fn stroke_text<T>(
        &mut self,
        start: Point,
        stroke: Stroke<T>,
        font: Font,
        font_size: f32,
        features: &[Feature],
        text: &str,
    ) where
        T: ColorSpace,
    {
        let glyphs = naive_shape(text, font.clone(), features, font_size);

        self.stroke_glyphs(start, stroke, &glyphs, font, text, font_size);
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
        self.push_instructions.push(PushInstruction::Mask(mask));
        self.sub_builders.push(ContentBuilder::new());
    }

    pub(crate) fn reset(&mut self) {
        self.push_instructions = vec![];
        self.sub_builders = vec![];
        self.root_builder = ContentBuilder::new();
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
            self.sub_builders.push(ContentBuilder::new());
        }
    }

    /// Push a new isolated layer.
    pub fn push_isolated(&mut self) {
        self.push_instructions.push(PushInstruction::Isolated);
        self.sub_builders.push(ContentBuilder::new());
    }

    /// Pop the last `push` instruction.
    pub fn pop(&mut self) {
        match self.push_instructions.pop().unwrap() {
            PushInstruction::Transform => {
                Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                    .restore_graphics_state()
            }
            PushInstruction::Opacity(o) => {
                if o != NormalizedF32::ONE {
                    let stream = self.sub_builders.pop().unwrap().finish();
                    Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                        .draw_opacified(o, stream);
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
                let stream = self.sub_builders.pop().unwrap().finish();
                Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                    .draw_masked(mask, stream)
            }
            PushInstruction::Isolated => {
                let stream = self.sub_builders.pop().unwrap().finish();
                Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
                    .draw_isolated(stream);
            }
        }
    }

    #[cfg(feature = "raster-images")]
    /// Draw a new bitmap image.
    pub fn draw_image(&mut self, image: Image, size: Size) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).draw_image(image, size);
    }

    #[cfg(feature = "svg")]
    /// Draw a new SVG image.
    pub fn draw_svg(&mut self, tree: &usvg::Tree, size: Size) -> Option<()> {
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
        svg::render_tree(tree, self.sc.serialize_settings.svg_settings, self);
        self.pop();
        self.pop();

        Some(())
    }

    pub(crate) fn draw_shading(&mut self, shading: &ShadingFunction) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).draw_shading(shading);
    }

    /// Convert a `fontdb` into `krilla` `Font` objects. This is a convenience method,
    /// which makes it easier to integrate `cosmic-text` with this library.
    #[cfg(feature = "fontdb")]
    pub fn convert_fontdb(&mut self, db: &mut Database, ids: Option<Vec<ID>>) -> Option<HashMap<ID, Font>> {
        self.sc.convert_fontdb(db, ids)
    }

    /// A convenience method for dropping the current surface.
    pub fn finish(self) {}

    pub(crate) fn draw_opacified_stream(&mut self, opacity: NormalizedF32, stream: Stream) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .draw_opacified(opacity, stream)
    }

    fn cur_builder<'b>(
        root_builder: &'b mut ContentBuilder,
        sub_builders: &'b mut [ContentBuilder],
    ) -> &'b mut ContentBuilder {
        sub_builders.last_mut().unwrap_or(root_builder)
    }

    pub(crate) fn fill_path_impl(
        &mut self,
        path: &Path,
        fill: Fill<impl ColorSpace>,
        no_fill: bool,
    ) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_path_impl(path, fill, self.sc, no_fill)
    }
}

impl Drop for Surface<'_> {
    fn drop(&mut self) {
        let root_builder = std::mem::replace(&mut self.root_builder, ContentBuilder::new());
        debug_assert!(self.sub_builders.is_empty());
        debug_assert!(self.push_instructions.is_empty());
        (self.finish_fn)(root_builder.finish())
    }
}

/// Shape some text with a single font.
#[cfg(feature = "simple-text")]
fn naive_shape(text: &str, font: Font, features: &[Feature], size: f32) -> Vec<Glyph> {
    let data = font.font_data();
    let mut rb_font = rustybuzz::Face::from_slice(data.as_ref().as_ref(), font.index()).unwrap();
    for (tag, val) in font.variations() {
        rb_font.set_variation(Tag::from_bytes_lossy(tag.as_bytes()), val);
    }

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();

    let dir = buffer.direction();

    let output = rustybuzz::shape(&rb_font, features, buffer);

    let positions = output.glyph_positions();
    let infos = output.glyph_infos();

    let mut glyphs = vec![];

    for i in 0..output.len() {
        let pos = positions[i];
        let start_info = infos[i];

        let start = start_info.cluster as usize;

        let end = if dir == Direction::LeftToRight {
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

        glyphs.push(Glyph::new(
            GlyphId::new(start_info.glyph_id),
            (pos.x_advance as f32 / font.units_per_em()) * size,
            (pos.x_offset as f32 / font.units_per_em()) * size,
            (pos.y_offset as f32 / font.units_per_em()) * size,
            start..end,
        ));
    }

    glyphs
}

#[cfg(test)]
mod tests {
    use crate::color::rgb::Rgb;
    use crate::font::Font;
    use crate::mask::MaskType;
    use crate::path::Fill;
    use crate::surface::Stroke;
    use crate::surface::Surface;
    use crate::tests::{
        basic_mask, blue_fill, cmyk_fill, gray_luma, green_fill, load_png_image, rect_to_path,
        red_fill, NOTO_SANS, NOTO_SANS_DEVANAGARI, SVGS_PATH,
    };
    use krilla_macros::{snapshot, visreg};
    use pdf_writer::types::BlendMode;
    use tiny_skia_path::{Point, Size, Transform};

    #[snapshot(stream)]
    fn stream_path_single_with_rgb(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let fill = red_fill(1.0);
        surface.fill_path(&path, fill);
    }

    #[snapshot(stream)]
    fn stream_path_single_with_luma(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let fill = gray_luma(1.0);
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
            Fill::<Rgb>::default(),
            Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap(),
            16.0,
            &[],
            "hi there",
        );
    }

    #[snapshot(stream)]
    fn stream_stroke_text(surface: &mut Surface) {
        surface.stroke_text(
            Point::from_xy(0.0, 50.0),
            Stroke::<Rgb>::default(),
            Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap(),
            16.0,
            &[],
            "hi there",
        );
    }

    #[snapshot(stream)]
    fn stream_complex_text(surface: &mut Surface) {
        surface.fill_text(
            Point::from_xy(0.0, 50.0),
            Fill::<Rgb>::default(),
            Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, vec![]).unwrap(),
            16.0,
            &[],
            "यह कुछ जटिल पाठ है.",
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

    fn sample_svg() -> usvg::Tree {
        let data = std::fs::read(SVGS_PATH.join("resvg_masking_mask_with_opacity_1.svg")).unwrap();
        usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap()
    }

    #[visreg(pdfium)]
    fn svg_simple(surface: &mut Surface) {
        let tree = sample_svg();
        surface.draw_svg(&tree, tree.size());
    }

    #[visreg(pdfium)]
    fn svg_resized(surface: &mut Surface) {
        surface.draw_svg(&sample_svg(), Size::from_wh(120.0, 80.0).unwrap());
    }

    #[visreg(pdfium)]
    fn svg_should_be_clipped(surface: &mut Surface) {
        let data = std::fs::read(SVGS_PATH.join("custom_paint_servers_pattern_patterns_2.svg")).unwrap();
        let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();

        surface.push_transform(&Transform::from_translate(100.0, 0.0));
        surface.draw_svg(&tree, tree.size());
        surface.pop();
    }
}
