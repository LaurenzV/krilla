use crate::font::Font;
use crate::object::annotation::Annotation;
use crate::object::color_space::ColorSpace;
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::page::{Page, PageLabel};
use crate::object::shading_function::ShadingFunction;
use crate::serialize::SerializerContext;
use crate::stream::{ContentBuilder, Glyph, Stream};
use crate::{Fill, FillRule, Stroke};
use fontdb::{Database, ID};
use pdf_writer::types::BlendMode;
use std::collections::HashMap;
use tiny_skia_path::{Path, Size, Transform};
use usvg::NormalizedF32;

pub enum PushInstruction {
    Transform,
    Opacity(NormalizedF32),
    ClipPath,
    BlendMode,
    Mask(Mask),
    Isolated,
}

pub struct Surface<'a> {
    sc: &'a mut SerializerContext,
    pub(crate) root_builder: ContentBuilder,
    sub_builders: Vec<ContentBuilder>,
    push_instructions: Vec<PushInstruction>,
    finish_fn: Box<dyn FnMut(Stream) + 'a>,
}

impl<'a> Surface<'a> {
    pub fn new(
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

    pub fn stream_builder(&mut self) -> StreamBuilder {
        StreamBuilder::new(&mut self.sc)
    }

    pub fn fill_path<T>(&mut self, path: &Path, fill: Fill<T>)
    where
        T: ColorSpace,
    {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_path(path, fill, self.sc);
    }

    pub fn stroke_path<T>(&mut self, path: &Path, stroke: Stroke<T>)
    where
        T: ColorSpace,
    {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .stroke_path(path, stroke, self.sc);
    }

    pub fn fill_glyphs<'b, T>(
        &mut self,
        x: f32,
        y: f32,
        fill: Fill<T>,
        glyphs: &[Glyph],
        font: Font,
        text: &str,
    ) where
        T: ColorSpace,
    {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_glyphs(x, y, self.sc, fill, glyphs, font, text);
    }

    pub fn stroke_glyphs<'b, T>(
        &mut self,
        x: f32,
        y: f32,
        stroke: Stroke<T>,
        glyphs: &[Glyph],
        font: Font,
        text: &str,
    ) where
        T: ColorSpace,
    {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .stroke_glyphs(x, y, self.sc, stroke, glyphs, font, text);
    }

    pub fn push_transform(&mut self, transform: &Transform) {
        self.push_instructions.push(PushInstruction::Transform);
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).save_graphics_state();
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .concat_transform(transform);
    }

    pub fn push_blend_mode(&mut self, blend_mode: BlendMode) {
        self.push_instructions.push(PushInstruction::BlendMode);
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).save_graphics_state();
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .set_blend_mode(blend_mode);
    }

    pub fn push_clip_path(&mut self, path: &Path, clip_rule: &FillRule) {
        self.push_instructions.push(PushInstruction::ClipPath);
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .push_clip_path(path, clip_rule);
    }

    pub fn push_mask(&mut self, mask: Mask) {
        self.push_instructions.push(PushInstruction::Mask(mask));
        self.sub_builders.push(ContentBuilder::new());
    }

    pub fn reset(&mut self) {
        self.push_instructions = vec![];
        self.sub_builders = vec![];
        self.root_builder = ContentBuilder::new();
    }

    pub fn push_opacified(&mut self, opacity: NormalizedF32) {
        self.push_instructions
            .push(PushInstruction::Opacity(opacity));

        if opacity != NormalizedF32::ONE {
            self.sub_builders.push(ContentBuilder::new());
        }
    }

    pub fn push_isolated(&mut self) {
        self.push_instructions.push(PushInstruction::Isolated);
        self.sub_builders.push(ContentBuilder::new());
    }

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

    pub fn draw_image(&mut self, image: Image, size: Size) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).draw_image(image, size);
    }

    pub fn draw_shading(&mut self, shading: &ShadingFunction) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).draw_shading(shading);
    }

    pub fn convert_fontdb(&mut self, db: &mut Database, ids: Option<Vec<ID>>) -> HashMap<ID, Font> {
        self.sc.convert_fontdb(db, ids)
    }

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
        // Replace with a dummy builder.
        let root_builder = std::mem::replace(&mut self.root_builder, ContentBuilder::new());
        debug_assert!(self.sub_builders.is_empty());
        debug_assert!(self.push_instructions.is_empty());
        (self.finish_fn)(root_builder.finish())
    }
}

pub struct PageBuilder<'a> {
    sc: &'a mut SerializerContext,
    size: Size,
    page_label: PageLabel,
    page_stream: Stream,
    annotations: Vec<Annotation>,
}

impl<'a> PageBuilder<'a> {
    pub(crate) fn new(sc: &'a mut SerializerContext, size: Size) -> Self {
        Self {
            sc,
            size,
            page_label: PageLabel::default(),
            page_stream: Stream::empty(),
            annotations: vec![],
        }
    }

    pub(crate) fn root_transform(&self) -> Transform {
        Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, self.size.height())
    }

    pub(crate) fn new_with(
        sc: &'a mut SerializerContext,
        size: Size,
        page_label: PageLabel,
    ) -> Self {
        Self {
            sc,
            size,
            page_label,
            page_stream: Stream::empty(),
            annotations: vec![],
        }
    }

    pub fn add_annotation(&mut self, annotation: Annotation) {
        self.annotations.push(annotation);
    }

    pub fn surface(&mut self) -> Surface {
        let mut root_builder = ContentBuilder::new();
        // Invert the y-axis.
        root_builder.concat_transform(&self.root_transform());

        let finish_fn = Box::new(|stream| self.page_stream = stream);

        Surface::new(&mut self.sc, root_builder, finish_fn)
    }

    pub fn finish(self) {}
}

impl Drop for PageBuilder<'_> {
    fn drop(&mut self) {
        let annotations = std::mem::take(&mut self.annotations);

        let stream = std::mem::replace(&mut self.page_stream, Stream::empty());
        let page = Page::new(self.size, stream, self.page_label.clone(), annotations);
        self.sc.add_page(page);
    }
}

pub struct StreamBuilder<'a> {
    sc: &'a mut SerializerContext,
    stream: Stream,
}

impl<'a> StreamBuilder<'a> {
    pub(crate) fn new(sc: &'a mut SerializerContext) -> Self {
        Self {
            sc,
            stream: Stream::empty(),
        }
    }

    pub fn surface(&mut self) -> Surface {
        let finish_fn = Box::new(|stream| {
            self.stream = stream;
        });

        Surface::new(&mut self.sc, ContentBuilder::new(), finish_fn)
    }

    pub fn finish(self) -> Stream {
        self.stream
    }
}
