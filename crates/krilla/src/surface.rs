//! Drawing onto a surface.
//!
//! This module contains most core part of krilla: The [`Surface`] struct. A surface
//! represents a drawing area on which you can define the contents of your page. This includes
//! operations such as applying linear transformations, showing text or images and drawing paths.

use crate::content::ContentBuilder;
use crate::geom::Path;
#[cfg(feature = "raster-images")]
use crate::geom::Size;
use crate::geom::{Point, Transform};
use crate::graphics::blend::BlendMode;
#[cfg(feature = "raster-images")]
use crate::graphics::image::Image;
use crate::graphics::mask::Mask;
use crate::graphics::paint::{Fill, FillRule, Stroke};
use crate::graphics::shading_function::ShadingFunction;
use crate::interchange::tagging::{ContentTag, Identifier, PageTagIdentifier};
use crate::num::NormalizedF32;
use crate::paint::InnerPaint;
use crate::serialize::SerializeContext;
use crate::stream::{Stream, StreamBuilder};
use crate::tagging::SpanTag;
use crate::text::{draw_glyph, Glyph};
#[cfg(feature = "simple-text")]
use crate::text::{shape::naive_shape, TextDirection};
use crate::text::{Font, PaintMode};

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
    fill: Option<Fill>,
    stroke: Option<Stroke>,
    bd: Builders,
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
            bd: Builders::new(root_builder),
            page_identifier,
            fill: None,
            stroke: None,
            push_instructions: vec![],
            finish_fn,
        }
    }

    /// Return a `StreamBuilder` to allow drawing on a sub-context.
    pub fn stream_builder(&mut self) -> StreamBuilder {
        StreamBuilder::new(self.sc)
    }

    /// Set the fill that should be used for the next drawing operation.
    ///
    /// You can set it to `None` if you want to disable filling (though
    /// if there is no active stroke, than krilla will choose to fill
    /// with black by default).
    pub fn set_fill(&mut self, fill: Option<Fill>) {
        self.fill = fill;
    }

    /// Get the currently active fill.
    pub fn get_fill(&mut self) -> Option<&Fill> {
        self.fill.as_ref()
    }

    /// Set the stroke that should be used for drawing operations.
    ///
    /// You can set it to `None` if you want to disable stroking.
    pub fn set_stroke(&mut self, stroke: Option<Stroke>) {
        self.stroke = stroke;
    }

    /// Get the currently active stroke.
    pub fn get_stroke(&mut self) -> Option<&Stroke> {
        self.stroke.as_ref()
    }

    /// Draw a path using the currently active fill and/or stroke.
    pub fn draw_path(&mut self, path: &Path) {
        if self.fill.is_some() || self.stroke.is_some() {
            if self.has_complex_fill_or_stroke() {
                self.bd
                    .get_mut()
                    .draw_path(&path.0, self.fill.as_ref(), None, self.sc);
                self.bd
                    .get_mut()
                    .draw_path(&path.0, None, self.stroke.as_ref(), self.sc);
            } else {
                self.bd.get_mut().draw_path(
                    &path.0,
                    self.fill.as_ref(),
                    self.stroke.as_ref(),
                    self.sc,
                );
            }
        } else {
            // Draw with black by default.
            self.bd
                .get_mut()
                .draw_path(&path.0, Some(&Fill::default()), None, self.sc);
        }
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
                        self.bd
                            .get_mut()
                            .start_marked_content_with_properties(self.sc, None, tag);
                    } else {
                        self.bd.get_mut().start_marked_content(tag.name());
                    }

                    Identifier::dummy()
                }
                ContentTag::Span(_) | ContentTag::Other => {
                    self.bd.get_mut().start_marked_content_with_properties(
                        self.sc,
                        Some(id.mcid),
                        tag,
                    );
                    id.bump().into()
                }
            }
        } else {
            Identifier::dummy()
        }
    }

    /// A temporary hacky method to support alt text in Typst. Will be removed in the future.
    #[doc(hidden)]
    pub fn start_alt_text(&mut self, text: &str) {
        let tag = ContentTag::Span(SpanTag {
            lang: None,
            alt_text: Some(text),
            expanded: None,
            actual_text: None,
        });

        self.bd
            .get_mut()
            .start_marked_content_with_properties(self.sc, None, tag);
    }

    /// A temporary hacky method to support alt text in Typst. Will be removed in the future.
    #[doc(hidden)]
    pub fn end_alt_text(&mut self) {
        self.bd.get_mut().end_marked_content();
    }

    /// End the current tagged section.
    ///
    /// # Panics
    /// Panics if no tagged section has been started.
    pub fn end_tagged(&mut self) {
        if self.page_identifier.is_some() {
            self.bd.get_mut().end_marked_content();
        }
    }

    fn outline_glyphs(
        &mut self,
        glyphs: &[impl Glyph],
        paint_mode: PaintMode,
        start: Point,
        font: Font,
        font_size: f32,
    ) {
        let (mut cur_x, y) = (start.x, start.y);

        for glyph in glyphs {
            let mut base_transform = tiny_skia_path::Transform::from_translate(
                cur_x + glyph.x_offset(font_size),
                y - glyph.y_offset(font_size),
            );
            base_transform = base_transform.pre_concat(tiny_skia_path::Transform::from_scale(
                font_size / font.units_per_em(),
                -font_size / font.units_per_em(),
            ));
            draw_glyph(
                font.clone(),
                paint_mode,
                glyph.glyph_id(),
                Transform::from_tsp(base_transform),
                self,
            );

            cur_x += glyph.x_advance(font_size);
        }
    }

    /// Draw a sequence of glyphs using the currently active fill and/or stroke.
    ///
    /// This is a very low-level method, which gives you full control over how to place
    /// the glyphs that make up the text. This means that you must have your own text processing
    /// logic for dealing with bidirectional text, font fallback, text layouting, etc.
    pub fn draw_glyphs(
        &mut self,
        start: Point,
        glyphs: &[impl Glyph],
        font: Font,
        text: &str,
        font_size: f32,
        outlined: bool,
    ) {
        if outlined {
            if let Some(paint_mode) = self.paint_mode() {
                self.outline_glyphs(
                    glyphs,
                    paint_mode.to_owned().as_ref(),
                    start,
                    font,
                    font_size,
                );
            } else {
                // Draw with black by default.
                self.outline_glyphs(
                    glyphs,
                    PaintMode::Fill(&Fill::default()),
                    start,
                    font,
                    font_size,
                );
            }
        } else {
            match (self.fill.as_ref(), self.stroke.as_ref()) {
                (Some(f), Some(s)) => {
                    if self.has_complex_fill_or_stroke() {
                        // We need to draw fill and stroke separately. In order to avoid embedding
                        // text twice, we fill the text as normal text and then stroke it as a path.
                        self.bd.get_mut().draw_glyphs(
                            start,
                            self.sc,
                            Some(f),
                            None,
                            glyphs,
                            font.clone(),
                            text,
                            font_size,
                        );

                        let stroke = s.clone();
                        self.outline_glyphs(
                            glyphs,
                            PaintMode::Stroke(&stroke),
                            start,
                            font,
                            font_size,
                        );
                    } else {
                        self.bd.get_mut().draw_glyphs(
                            start,
                            self.sc,
                            Some(f),
                            Some(s),
                            glyphs,
                            font,
                            text,
                            font_size,
                        );
                    }
                }
                (None, Some(s)) => {
                    self.bd.get_mut().draw_glyphs(
                        start,
                        self.sc,
                        None,
                        Some(s),
                        glyphs,
                        font,
                        text,
                        font_size,
                    );
                }
                (Some(f), None) => {
                    self.bd.get_mut().draw_glyphs(
                        start,
                        self.sc,
                        Some(f),
                        None,
                        glyphs,
                        font,
                        text,
                        font_size,
                    );
                }
                (None, None) => {
                    // Draw with black by default.
                    self.bd.get_mut().draw_glyphs(
                        start,
                        self.sc,
                        Some(&Fill::default()),
                        None,
                        glyphs,
                        font,
                        text,
                        font_size,
                    );
                }
            }
            if self.fill.is_some() || self.stroke.is_some() {
            } else {
            }
        }
    }

    /// Draw some text using the currently active fill and/or stroke.
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
    pub fn draw_text(
        &mut self,
        start: Point,
        font: Font,
        font_size: f32,
        text: &str,
        outlined: bool,
        direction: TextDirection,
    ) {
        let glyphs = naive_shape(text, font.clone(), direction);

        self.draw_glyphs(start, &glyphs, font, text, font_size, outlined);
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
        self.bd.get().cur_transform()
    }

    /// Concatenate a new transform to the current transformation matrix.
    pub fn push_transform(&mut self, transform: &Transform) {
        self.push_instructions.push(PushInstruction::Transform);
        self.bd.get_mut().save_graphics_state();
        self.bd.get_mut().concat_transform(transform);
    }

    /// Push a new blend mode.
    pub fn push_blend_mode(&mut self, blend_mode: BlendMode) {
        self.push_instructions.push(PushInstruction::BlendMode);
        self.bd.get_mut().save_graphics_state();
        self.bd.get_mut().set_blend_mode(blend_mode.to_pdf());
    }

    /// Push a new clip path.
    pub fn push_clip_path(&mut self, path: &Path, clip_rule: &FillRule) {
        self.push_instructions.push(PushInstruction::ClipPath);
        self.bd.get_mut().push_clip_path(&path.0, clip_rule);
    }

    /// Push a new mask.
    pub fn push_mask(&mut self, mask: Mask) {
        self.push_instructions
            .push(PushInstruction::Mask(Box::new(mask)));
        self.bd
            .sub_builders
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
            self.bd
                .sub_builders
                .push(ContentBuilder::new(Transform::identity()));
        }
    }

    /// Push a new isolated layer.
    pub fn push_isolated(&mut self) {
        self.push_instructions.push(PushInstruction::Isolated);
        self.bd
            .sub_builders
            .push(ContentBuilder::new(Transform::identity()));
    }

    /// Pop the last `push` instruction.
    ///
    /// # Panics
    /// Panics if the there wasn't a corresponding `push` to the `pop`.
    pub fn pop(&mut self) {
        match self.push_instructions.pop().unwrap() {
            PushInstruction::Transform => self.bd.get_mut().restore_graphics_state(),
            PushInstruction::Opacity(o) => {
                if o != NormalizedF32::ONE {
                    let stream = self.bd.sub_builders.pop().unwrap().finish(self.sc);
                    self.bd.get_mut().draw_opacified(self.sc, o, stream);
                }
            }
            PushInstruction::ClipPath => self.bd.get_mut().pop_clip_path(),
            PushInstruction::BlendMode => self.bd.get_mut().restore_graphics_state(),
            PushInstruction::Mask(mask) => {
                let stream = self.bd.sub_builders.pop().unwrap().finish(self.sc);
                self.bd.get_mut().draw_masked(self.sc, *mask, stream)
            }
            PushInstruction::Isolated => {
                let stream = self.bd.sub_builders.pop().unwrap().finish(self.sc);
                self.bd.get_mut().draw_isolated(self.sc, stream);
            }
        }
    }

    #[cfg(feature = "raster-images")]
    /// Draw a new bitmap image.
    pub fn draw_image(&mut self, image: Image, size: Size) {
        self.bd.get_mut().draw_image(image, size, self.sc);
    }

    pub(crate) fn draw_shading(&mut self, shading: &ShadingFunction) {
        self.bd.get_mut().draw_shading(shading, self.sc);
    }

    /// A convenience method for `std::mem::drop`.
    ///
    /// # Panics
    /// Panics if the push/pop difference is not 0.
    pub fn finish(self) {}

    pub(crate) fn draw_opacified_stream(&mut self, opacity: NormalizedF32, stream: Stream) {
        self.bd.get_mut().draw_opacified(self.sc, opacity, stream)
    }

    /// Return the current transformation matrix of the surface.
    pub fn cur_transform(&self) -> Transform {
        self.bd.get().cur_transform()
    }

    fn paint_mode(&self) -> Option<PaintMode> {
        match (&self.fill, &self.stroke) {
            (None, None) => None,
            (Some(f), None) => Some(PaintMode::Fill(f)),
            (None, Some(s)) => Some(PaintMode::Stroke(s)),
            (Some(f), Some(s)) => Some(PaintMode::FillStroke(f, s)),
        }
    }

    fn has_complex_fill_or_stroke(&self) -> bool {
        // Things can go wrong and yield inconsistent results in different PDF viewers
        // for some fill/stroke combinations. In the case of solid colors on strokes with opacity,
        // different PDF viewers draw them differently. In the case of complex paints on either fill
        // or stroke, if they have an opacity we need to apply a mask in the content stream,
        // in which case the mask will apply to the fill AND the stroke, which is not intended.
        // Because of this, in case the fill or stroke is complex, we fill and stroke
        // glyphs and path in two separate steps instead of in one go.

        let complex_stroke = self.stroke.as_ref().is_some_and(|s| match s.paint.0 {
            InnerPaint::Color(_) => s.opacity != NormalizedF32::ONE,
            _ => true,
        });

        let complex_fill = self.fill.as_ref().is_some_and(|f| match f.paint.0 {
            InnerPaint::Color(_) => false,
            _ => true,
        });

        complex_fill || complex_stroke
    }
}

impl Drop for Surface<'_> {
    fn drop(&mut self) {
        let root_builder = std::mem::replace(
            &mut self.bd.root_builder,
            ContentBuilder::new(Transform::identity()),
        );
        let num_mcids = match self.page_identifier {
            Some(pi) => pi.mcid,
            None => 0,
        };

        assert!(self.bd.sub_builders.is_empty());
        assert!(self.push_instructions.is_empty());
        assert!(!root_builder.active_marked_content);

        (self.finish_fn)(root_builder.finish(self.sc), num_mcids)
    }
}

/// Holds the different content streams we are currently building. In the usual case,
/// this only contains the current page stream as the root builder, but the sub builders
/// will be used if we are for example creating a mask/pattern, or an XObject.
struct Builders {
    pub(crate) root_builder: ContentBuilder,
    pub(crate) sub_builders: Vec<ContentBuilder>,
}

impl Builders {
    fn new(root_builder: ContentBuilder) -> Self {
        Self {
            root_builder,
            sub_builders: vec![],
        }
    }

    fn get_mut(&mut self) -> &mut ContentBuilder {
        self.sub_builders
            .last_mut()
            .unwrap_or(&mut self.root_builder)
    }

    fn get(&self) -> &ContentBuilder {
        self.sub_builders.last().unwrap_or(&self.root_builder)
    }
}

pub(crate) enum PushInstruction {
    Transform,
    Opacity(NormalizedF32),
    ClipPath,
    BlendMode,
    Mask(Box<Mask>),
    Isolated,
}
