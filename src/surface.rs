use crate::content::ContentBuilder;
use crate::font::Font;
use crate::object::color::ColorSpace;
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_function::ShadingFunction;
use crate::path::{Fill, FillRule, Stroke};
use crate::serialize::SerializerContext;
use crate::stream::{Glyph, Stream};
use crate::svg;
use fontdb::{Database, ID};
use pdf_writer::types::BlendMode;
use rustybuzz::{Direction, Feature, UnicodeBuffer};
use skrifa::GlyphId;
use std::collections::HashMap;
use tiny_skia_path::{Path, Point, Size, Transform};
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

    pub fn fill_glyphs<T>(
        &mut self,
        start: Point,
        fill: Fill<T>,
        glyphs: &[Glyph],
        font: Font,
        text: &str,
    ) where
        T: ColorSpace,
    {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_glyphs(start, self.sc, fill, glyphs, font, text);
    }

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

        self.fill_glyphs(start, fill, &glyphs, font, text);
    }

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

        self.stroke_glyphs(start, stroke, &glyphs, font, text);
    }

    pub fn stroke_glyphs<T>(
        &mut self,
        start: Point,
        stroke: Stroke<T>,
        glyphs: &[Glyph],
        font: Font,
        text: &str,
    ) where
        T: ColorSpace,
    {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .stroke_glyphs(start, self.sc, stroke, glyphs, font, text);
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

    pub fn draw_svg(&mut self, tree: &usvg::Tree, size: Size) {
        let transform = Transform::from_scale(
            tree.size().width() / size.width(),
            tree.size().height() / size.height(),
        );
        self.push_transform(&transform);
        svg::render_tree(tree, self.sc.serialize_settings.svg_settings, self);
        self.pop();
    }

    pub(crate) fn draw_shading(&mut self, shading: &ShadingFunction) {
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

fn naive_shape(text: &str, font: Font, features: &[Feature], size: f32) -> Vec<Glyph> {
    let data = font.font_data();
    let rb_font = rustybuzz::Face::from_slice(data.as_ref().as_ref(), font.index()).unwrap();

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
            loop {
                if let Some(index) = e {
                    if let Some(end_info) = infos.get(index) {
                        if end_info.cluster == start_info.cluster {
                            e = index.checked_sub(1);
                        } else {
                            break;
                        }
                    }
                } else {
                    break;
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
            size,
        ));
    }

    glyphs
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
