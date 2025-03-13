//! Drawing onto a surface.
//!
//! This module contains most core part of krilla: The [`Surface`] struct. A surface
//! represents a drawing area on which you can define the contents of your page. This includes
//! operations such as applying linear transformations,
//! showing text or images and drawing paths.

use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "simple-text")]
use rustybuzz::{Direction, UnicodeBuffer};

use crate::content::{unit_normalize, ContentBuilder};
#[cfg(feature = "simple-text")]
use crate::font::GlyphId;
#[cfg(feature = "simple-text")]
use crate::font::KrillaGlyph;
use crate::font::{draw_glyph, Font, FontInfo, Glyph, GlyphUnits};
use crate::object::font::PaintMode;
#[cfg(feature = "raster-images")]
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_function::ShadingFunction;
use crate::path::{Fill, FillRule, Path, Stroke};
use crate::serialize::SerializeContext;
use crate::stream::{Stream, StreamBuilder};
use crate::tagging::{ContentTag, Identifier, PageTagIdentifier};
#[cfg(feature = "raster-images")]
use crate::Size;
use crate::{NormalizedF32, Point, Transform};

pub(crate) enum PushInstruction {
    Transform,
    Opacity(NormalizedF32),
    ClipPath,
    BlendMode,
    Mask(Box<Mask>),
    Isolated,
}

/// Can be used to associate render operations with a unique identifier.
/// This is useful if you want to backtrack a validation error to a specific
/// identifier chosen by you.
pub type Location = u64;

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
    pub(crate) sc: &'a mut SerializeContext,
    pub(crate) root_builder: ContentBuilder,
    fill: Fill,
    stroke: Stroke,
    sub_builders: Vec<ContentBuilder>,
    push_instructions: Vec<PushInstruction>,
    page_identifier: Option<PageTagIdentifier>,
    finish_fn: Box<dyn FnMut(Stream, i32) + 'a>,
}

impl<'a> Surface<'a> {
    pub(crate) fn new(
        sc: &'a mut SerializeContext,
        root_builder: ContentBuilder,
        page_identifier: Option<PageTagIdentifier>,
        finish_fn: Box<dyn FnMut(Stream, i32) + 'a>,
    ) -> Surface<'a> {
        Self {
            sc,
            root_builder,
            page_identifier,
            fill: Fill::default(),
            stroke: Stroke::default(),
            sub_builders: vec![],
            push_instructions: vec![],
            finish_fn,
        }
    }

    /// Return a `StreamBuilder` to allow drawing on a sub-context.
    pub fn stream_builder(&mut self) -> StreamBuilder {
        StreamBuilder::new(self.sc)
    }

    /// Set the fill that should be used for filling operations.
    pub fn set_fill(&mut self, fill: Fill) {
        self.fill = fill;
    }

    /// Set the stroke that should be used for stroking operations.
    pub fn set_stroke(&mut self, stroke: Stroke) {
        self.stroke = stroke;
    }

    /// Fill a path.
    pub fn fill_path(&mut self, path: &Path) {
        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders).fill_path(
            &path.0,
            self.fill.clone(),
            self.sc,
        );
    }

    /// Stroke a path.
    pub fn stroke_path(&mut self, path: &Path) {
        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders).stroke_path(
            &path.0,
            self.stroke.clone(),
            self.sc,
        )
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
                        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
                            .start_marked_content_with_properties(self.sc, None, tag);
                    } else {
                        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
                            .start_marked_content(tag.name());
                    }

                    Identifier::dummy()
                }
                ContentTag::Span(_, _, _, _) | ContentTag::Other => {
                    Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
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
            Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
                .end_marked_content();
        }
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
            let mut base_transform = tiny_skia_path::Transform::from_translate(
                cur_x + normalize(glyph.x_offset()) * font_size,
                y - normalize(glyph.y_offset()) * font_size,
            );
            base_transform = base_transform.pre_concat(tiny_skia_path::Transform::from_scale(
                font_size / font.units_per_em(),
                -font_size / font.units_per_em(),
            ));
            draw_glyph(
                font.clone(),
                glyph.glyph_id(),
                paint_mode,
                Transform::from_tsp(base_transform),
                self,
            );

            cur_x += normalize(glyph.x_advance()) * font_size;
        }
    }

    /// Draw a sequence of glyphs using the current fill.
    ///
    /// This is a very low-level method, which gives you full control over how to place
    /// the glyphs that make up the text. This means that you must have your own text processing
    /// logic for dealing with bidirectional text, font fallback, text layouting, etc.
    pub fn fill_glyphs(
        &mut self,
        start: Point,
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
                PaintMode::Fill(&self.fill.clone()),
            );
        } else {
            Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders).fill_glyphs(
                start,
                self.sc,
                self.fill.clone(),
                glyphs,
                font,
                text,
                font_size,
                glyph_units,
            );
        }
    }

    /// Draw some text using the current fill.
    ///
    /// This is a high-level method which allows you to just provide some text, which will
    /// then be rendered into a single line. However, this approach has restrictions:
    ///
    /// - It will not perform BIDI resolution and only supports a single script, meaning that you
    ///   must ensure that your text does not contain multiple scripts.
    /// - It will only use the single font you provided to draw the text, no font fallback will
    ///   be performed.
    ///
    /// If you need more advanced control over how your text looks,
    /// you can use the `fill_glyphs` method.
    #[cfg(feature = "simple-text")]
    pub fn fill_text(
        &mut self,
        start: Point,
        font: Font,
        font_size: f32,
        text: &str,
        outlined: bool,
        direction: TextDirection,
    ) {
        let glyphs = naive_shape(text, font.clone(), font_size, direction);

        self.fill_glyphs(
            start,
            &glyphs,
            font,
            text,
            font_size,
            GlyphUnits::UserSpace,
            outlined,
        );
    }

    /// Draw a sequence of glyphs using the current stroke.
    #[allow(clippy::too_many_arguments)]
    pub fn stroke_glyphs(
        &mut self,
        start: Point,
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
                PaintMode::Stroke(&self.stroke.clone()),
            );
        } else {
            Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders).stroke_glyphs(
                start,
                self.sc,
                self.stroke.clone(),
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
    /// If you need more advanced control over how your text looks,
    /// you can use the `stroke_glyphs` method.
    #[cfg(feature = "simple-text")]
    pub fn stroke_text(
        &mut self,
        start: Point,
        font: Font,
        font_size: f32,
        text: &str,
        outlined: bool,
        direction: TextDirection,
    ) {
        let glyphs = naive_shape(text, font.clone(), font_size, direction);

        self.stroke_glyphs(
            start,
            &glyphs,
            font,
            text,
            font_size,
            GlyphUnits::UserSpace,
            outlined,
        );
    }

    /// Set the location that should be assumed for subsequent operations.
    pub fn set_location(&mut self, location: Location) {
        self.sc.set_location(location);
    }

    /// Reset the location that should be assumed for subsequent operations.
    pub fn reset_location(&mut self) {
        self.sc.reset_location();
    }

    /// Return the current transformation matrix.
    pub fn ctm(&self) -> Transform {
        Self::cur_builder(&self.root_builder, &self.sub_builders).cur_transform()
    }

    /// Concatenate a new transform to the current transformation matrix.
    pub fn push_transform(&mut self, transform: &Transform) {
        self.push_instructions.push(PushInstruction::Transform);
        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders).save_graphics_state();
        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
            .concat_transform(transform);
    }

    /// Push a new blend mode.
    pub fn push_blend_mode(&mut self, blend_mode: BlendMode) {
        self.push_instructions.push(PushInstruction::BlendMode);
        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders).save_graphics_state();
        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
            .set_blend_mode(blend_mode.to_pdf());
    }

    /// Push a new clip path.
    pub fn push_clip_path(&mut self, path: &Path, clip_rule: &FillRule) {
        self.push_instructions.push(PushInstruction::ClipPath);
        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
            .push_clip_path(&path.0, clip_rule);
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
                Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
                    .restore_graphics_state()
            }
            PushInstruction::Opacity(o) => {
                if o != NormalizedF32::ONE {
                    let stream = self.sub_builders.pop().unwrap().finish(self.sc);
                    Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
                        .draw_opacified(self.sc, o, stream);
                }
            }
            PushInstruction::ClipPath => {
                Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
                    .pop_clip_path()
            }
            PushInstruction::BlendMode => {
                Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
                    .restore_graphics_state()
            }
            PushInstruction::Mask(mask) => {
                let stream = self.sub_builders.pop().unwrap().finish(self.sc);
                Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
                    .draw_masked(self.sc, *mask, stream)
            }
            PushInstruction::Isolated => {
                let stream = self.sub_builders.pop().unwrap().finish(self.sc);
                Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
                    .draw_isolated(self.sc, stream);
            }
        }
    }

    #[cfg(feature = "raster-images")]
    /// Draw a new bitmap image.
    pub fn draw_image(&mut self, image: Image, size: Size) {
        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
            .draw_image(image, size, self.sc);
    }

    pub(crate) fn draw_shading(&mut self, shading: &ShadingFunction) {
        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
            .draw_shading(shading, self.sc);
    }

    /// THIS IS AN INTERNAL FUNCTION, DO NOT USE DIRECTLY!
    ///
    /// Returns the font cache of the surface.
    #[doc(hidden)]
    pub fn font_cache(&self) -> &HashMap<Arc<FontInfo>, Font> {
        &self.sc.font_cache
    }

    /// A convenience method for `std::mem::drop`.
    ///
    /// # Panics
    /// Panics if the push/pop difference is not 0.
    pub fn finish(self) {}

    pub(crate) fn draw_opacified_stream(&mut self, opacity: NormalizedF32, stream: Stream) {
        Self::cur_builder_mut(&mut self.root_builder, &mut self.sub_builders)
            .draw_opacified(self.sc, opacity, stream)
    }

    /// Return the current transformation matrix of the surface.
    pub fn cur_transform(&self) -> Transform {
        Self::cur_builder(&self.root_builder, &self.sub_builders).cur_transform()
    }

    fn cur_builder_mut<'b>(
        root_builder: &'b mut ContentBuilder,
        sub_builders: &'b mut [ContentBuilder],
    ) -> &'b mut ContentBuilder {
        sub_builders.last_mut().unwrap_or(root_builder)
    }

    fn cur_builder<'b>(
        root_builder: &'b ContentBuilder,
        sub_builders: &'b [ContentBuilder],
    ) -> &'b ContentBuilder {
        sub_builders.last().unwrap_or(root_builder)
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

/// How to blend source and backdrop.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[allow(missing_docs)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl BlendMode {
    fn to_pdf(self) -> pdf_writer::types::BlendMode {
        match self {
            BlendMode::Normal => pdf_writer::types::BlendMode::Normal,
            BlendMode::Multiply => pdf_writer::types::BlendMode::Multiply,
            BlendMode::Screen => pdf_writer::types::BlendMode::Screen,
            BlendMode::Overlay => pdf_writer::types::BlendMode::Overlay,
            BlendMode::Darken => pdf_writer::types::BlendMode::Darken,
            BlendMode::Lighten => pdf_writer::types::BlendMode::Lighten,
            BlendMode::ColorDodge => pdf_writer::types::BlendMode::ColorDodge,
            BlendMode::ColorBurn => pdf_writer::types::BlendMode::ColorBurn,
            BlendMode::HardLight => pdf_writer::types::BlendMode::HardLight,
            BlendMode::SoftLight => pdf_writer::types::BlendMode::SoftLight,
            BlendMode::Difference => pdf_writer::types::BlendMode::Difference,
            BlendMode::Exclusion => pdf_writer::types::BlendMode::Exclusion,
            BlendMode::Hue => pdf_writer::types::BlendMode::Hue,
            BlendMode::Saturation => pdf_writer::types::BlendMode::Saturation,
            BlendMode::Color => pdf_writer::types::BlendMode::Color,
            BlendMode::Luminosity => pdf_writer::types::BlendMode::Luminosity,
        }
    }
}

/// Shape some text with a single font.
#[cfg(feature = "simple-text")]
fn naive_shape(text: &str, font: Font, size: f32, direction: TextDirection) -> Vec<KrillaGlyph> {
    let data = font.font_data();
    let rb_font = rustybuzz::Face::from_slice(data.as_ref(), font.index()).unwrap();

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

    let output = rustybuzz::shape(&rb_font, &[], buffer);

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
            None,
        ));
    }

    glyphs
}
