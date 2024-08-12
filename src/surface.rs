use crate::object::color_space::ColorSpace;
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::page::Page;
use crate::object::shading_function::ShadingFunction;
use crate::serialize::SerializerContext;
use crate::stream::{ContentBuilder, Stream, TestGlyph};
use crate::{Fill, FillRule, Stroke};
use fontdb::Database;
use pdf_writer::types::BlendMode;
use std::iter::Peekable;
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
    root_builder: ContentBuilder,
    sub_builders: Vec<ContentBuilder>,
    push_instructions: Vec<PushInstruction>,
    finish_fn: Box<dyn FnMut(Stream, &mut SerializerContext) + 'a>,
}

impl<'a> Surface<'a> {
    pub fn new(
        sc: &'a mut SerializerContext,
        root_builder: ContentBuilder,
        finish_fn: Box<dyn FnMut(Stream, &mut SerializerContext) + 'a>,
    ) -> Surface<'a> {
        Self {
            sc,
            root_builder,
            sub_builders: vec![],
            push_instructions: vec![],
            finish_fn,
        }
    }

    pub fn stream_surface(&mut self) -> StreamBuilder {
        StreamBuilder::new(&mut self.sc)
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

    pub fn fill_path(&mut self, path: &Path, fill: Fill<impl ColorSpace>) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_path(path, fill, self.sc);
    }

    pub fn stroke_path(&mut self, path: &Path, stroke: Stroke<impl ColorSpace>) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .stroke_path(path, stroke, self.sc);
    }

    pub fn push_clip_path(&mut self, path: &Path, clip_rule: &FillRule) {
        self.push_instructions.push(PushInstruction::ClipPath);
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .push_clip_path(path, clip_rule);
    }

    pub fn invisible_glyph_run(
        &mut self,
        x: f32,
        y: f32,
        fontdb: &mut Database,
        glyphs: Peekable<impl Iterator<Item = TestGlyph>>,
    ) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .invisible_glyph_run(x, y, fontdb, self.sc, glyphs)
    }

    pub fn fill_glyph_run(
        &mut self,
        x: f32,
        y: f32,
        fontdb: &mut Database,
        fill: Fill<impl ColorSpace>,
        glyphs: Peekable<impl Iterator<Item = TestGlyph>>,
    ) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_glyph_run(x, y, fontdb, self.sc, fill, glyphs);
    }

    pub fn stroke_glyph_run(
        &mut self,
        x: f32,
        y: f32,
        fontdb: &mut Database,
        stroke: Stroke<impl ColorSpace>,
        glyphs: Peekable<impl Iterator<Item = TestGlyph>>,
    ) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .stroke_glyph_run(x, y, fontdb, self.sc, stroke, glyphs);
    }

    pub fn push_mask(&mut self, mask: Mask) {
        self.push_instructions.push(PushInstruction::Mask(mask));
        self.sub_builders.push(ContentBuilder::new());
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

    pub fn finish(mut self) {
        (self.finish_fn)(self.root_builder.finish(), &mut self.sc)
    }

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

pub struct PageBuilder<'a> {
    sc: &'a mut SerializerContext,
    size: Size,
}

impl<'a> PageBuilder<'a> {
    pub(crate) fn new(sc: &'a mut SerializerContext, size: Size) -> Self {
        Self { sc, size }
    }

    pub fn surface(&mut self) -> Surface {
        let mut root_builder = ContentBuilder::new();
        // Invert the y-axis.
        root_builder.concat_transform(&Transform::from_row(
            1.0,
            0.0,
            0.0,
            -1.0,
            0.0,
            self.size.height(),
        ));

        let finish_fn = Box::new(|stream, sc: &mut SerializerContext| {
            let page = Page::new(self.size, stream);
            sc.add(page);
        });

        Surface::new(&mut self.sc, root_builder, finish_fn)
    }

    pub fn finish(self) {}
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
        let finish_fn = Box::new(|stream, _: &mut SerializerContext| {
            self.stream = stream;
        });

        Surface::new(&mut self.sc, ContentBuilder::new(), finish_fn)
    }

    pub fn finish(self) -> Stream {
        self.stream
    }
}

#[cfg(test)]
mod tests {
    // use tiny_skia_path::{Path, PathBuilder};

    // fn dummy_path(w: f32) -> Path {
    //     let mut builder = PathBuilder::new();
    //     builder.move_to(0.0, 0.0);
    //     builder.line_to(w, 0.0);
    //     builder.line_to(w, w);
    //     builder.line_to(0.0, w);
    //     builder.close();
    //
    //     builder.finish().unwrap()
    // }

    // #[test]
    // fn fill() {
    //     use crate::serialize::PageSerialize;
    //     let mut canvas = Canvas::new(Size::from_wh(100.0, 100.0).unwrap());
    //     canvas.fill_path(
    //         dummy_path(100.0),
    //         Transform::from_scale(2.0, 2.0),
    //         Fill {
    //             paint: Paint::Color(Color::new_rgb(200, 0, 0)),
    //             opacity: NormalizedF32::new(0.25).unwrap(),
    //             ..Fill::default()
    //         },
    //     );
    //
    //     let serialize_settings = SerializeSettings::default();
    //
    //     let chunk = PageSerialize::serialize(canvas, serialize_settings);
    //     let finished = chunk.finish();
    //
    //     write("fill", &finished);
    // }
    //
    // #[test]
    // fn blend() {
    //     use crate::serialize::PageSerialize;
    //     let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
    //     canvas.fill_path(
    //         dummy_path(100.0),
    //         Transform::from_translate(25.0, 25.0),
    //         Fill {
    //             paint: Paint::Color(Color::new_rgb(255, 0, 0)),
    //             opacity: NormalizedF32::new(0.25).unwrap(),
    //             ..Fill::default()
    //         },
    //     );
    //
    //     let mut blended = canvas.blended(BlendMode::Difference);
    //     let mut transformed = blended.transformed(Transform::from_translate(100.0, 100.0));
    //     transformed.fill_path(
    //         dummy_path(100.0),
    //         Transform::from_translate(-25.0, -25.0),
    //         Fill {
    //             paint: Paint::Color(Color::new_rgb(255, 255, 0)),
    //             opacity: NormalizedF32::new(1.0).unwrap(),
    //             ..Fill::default()
    //         },
    //     );
    //
    //     transformed.finish();
    //     blended.finish();
    //
    //     let serialize_settings = SerializeSettings::default();
    //
    //     let chunk = PageSerialize::serialize(canvas, serialize_settings);
    //     let finished = chunk.finish();
    //
    //     write("blend", &finished);
    // }
    //
    // #[test]
    // fn nested_opacity() {
    //     use crate::serialize::PageSerialize;
    //     let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
    //     canvas.fill_path(
    //         dummy_path(100.0),
    //         Transform::identity(),
    //         Fill {
    //             paint: Paint::Color(Color::new_rgb(255, 255, 0)),
    //             opacity: NormalizedF32::new(0.5).unwrap(),
    //             ..Fill::default()
    //         },
    //     );
    //
    //     let mut translated = canvas.transformed(Transform::from_translate(100.0, 100.0));
    //     let mut opacified = translated.opacified(NormalizedF32::new(0.5).unwrap());
    //     opacified.fill_path(
    //         dummy_path(100.0),
    //         Transform::identity(),
    //         Fill {
    //             paint: Paint::Color(Color::new_rgb(255, 255, 0)),
    //             opacity: NormalizedF32::new(0.5).unwrap(),
    //             ..Fill::default()
    //         },
    //     );
    //
    //     opacified.finish();
    //     translated.finish();
    //
    //     let serialize_settings = SerializeSettings::default();
    //
    //     let chunk = PageSerialize::serialize(canvas, serialize_settings);
    //     let finished = chunk.finish();
    //
    //     write("nested_opacity", &finished);
    // }
    //
    // fn sweep_gradient(spread_method: SpreadMethod, name: &str) {
    //     use crate::serialize::PageSerialize;
    //     let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
    //     canvas.fill_path(
    //         dummy_path(160.0),
    //         Transform::from_translate(0.0, 0.0).try_into().unwrap(),
    //         Fill {
    //             paint: Paint::SweepGradient(SweepGradient {
    //                 cx: FiniteF32::new(80.0).unwrap(),
    //                 cy: FiniteF32::new(80.0).unwrap(),
    //                 start_angle: FiniteF32::new(0.0).unwrap(),
    //                 end_angle: FiniteF32::new(90.0).unwrap(),
    //                 transform: TransformWrapper(
    //                     // Transform::from_scale(0.5, 0.5),
    //                     // ), // Transform::from_scale(0.5, 0.5),
    //                     Transform::from_scale(1.0, -1.0),
    //                 ),
    //                 spread_method,
    //                 stops: vec![
    //                     Stop {
    //                         offset: NormalizedF32::new(0.0).unwrap(),
    //                         color: Color::new_rgb(255, 0, 0),
    //                         opacity: NormalizedF32::new(0.7).unwrap(),
    //                     },
    //                     Stop {
    //                         offset: NormalizedF32::new(0.4).unwrap(),
    //                         color: Color::new_rgb(0, 255, 0),
    //                         opacity: NormalizedF32::ONE,
    //                     },
    //                     Stop {
    //                         offset: NormalizedF32::new(1.0).unwrap(),
    //                         color: Color::new_rgb(0, 0, 255),
    //                         opacity: NormalizedF32::new(0.5).unwrap(),
    //                     },
    //                 ],
    //             }),
    //             opacity: NormalizedF32::ONE,
    //             ..Fill::default()
    //         },
    //     );
    //
    //     let serialize_settings = SerializeSettings::default();
    //
    //     let chunk = PageSerialize::serialize(canvas, serialize_settings);
    //     let finished = chunk.finish();
    //
    //     write(&format!("sweep_gradient_{}", name), &finished);
    // }
    //
    // #[test]
    // fn sweep_gradient_reflect() {
    //     sweep_gradient(SpreadMethod::Reflect, "reflect");
    // }
    //
    // #[test]
    // fn sweep_gradient_repeat() {
    //     sweep_gradient(SpreadMethod::Repeat, "repeat");
    // }
    //
    // #[test]
    // fn sweep_gradient_pad() {
    //     sweep_gradient(SpreadMethod::Pad, "pad");
    // }
    //
    // fn linear_gradient(spread_method: SpreadMethod, name: &str) {
    //     use crate::serialize::PageSerialize;
    //     let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
    //     canvas.fill_path(
    //         dummy_path(160.0),
    //         Transform::from_translate(0.0, 0.0).try_into().unwrap(),
    //         Fill {
    //             paint: Paint::LinearGradient(LinearGradient {
    //                 x1: FiniteF32::new(0.1 * 160.0).unwrap(),
    //                 y1: FiniteF32::new(0.6 * 160.0).unwrap(),
    //                 x2: FiniteF32::new(0.3 * 160.0).unwrap(),
    //                 y2: FiniteF32::new(0.6 * 160.0).unwrap(),
    //                 transform: TransformWrapper(
    //                     Transform::identity(), // Transform::from_scale(0.5, 0.5),
    //                                            // Transform::from_scale(0.5, 0.5).pre_concat(Transform::from_rotate(45.0)),
    //                 ), // Transform::from_scale(0.5, 0.5),
    //                 // Transform::identity()
    //                 spread_method,
    //                 stops: vec![
    //                     Stop {
    //                         offset: NormalizedF32::new(0.2).unwrap(),
    //                         color: Color::new_rgb(255, 0, 0),
    //                         opacity: NormalizedF32::ONE,
    //                     },
    //                     Stop {
    //                         offset: NormalizedF32::new(0.4).unwrap(),
    //                         color: Color::new_rgb(0, 255, 0),
    //                         opacity: NormalizedF32::new(0.5).unwrap(),
    //                     },
    //                     Stop {
    //                         offset: NormalizedF32::new(0.8).unwrap(),
    //                         color: Color::new_rgb(0, 0, 255),
    //                         opacity: NormalizedF32::ONE,
    //                     },
    //                 ],
    //             }),
    //             opacity: NormalizedF32::ONE,
    //             ..Fill::default()
    //         },
    //     );
    //
    //     let serialize_settings = SerializeSettings::default();
    //
    //     let chunk = PageSerialize::serialize(canvas, serialize_settings);
    //     let finished = chunk.finish();
    //
    //     write(&format!("linear_gradient_{}", name), &finished);
    // }
    //
    // #[test]
    // fn linear_gradient_reflect() {
    //     linear_gradient(SpreadMethod::Reflect, "reflect");
    // }
    //
    // #[test]
    // fn linear_gradient_repeat() {
    //     linear_gradient(SpreadMethod::Repeat, "repeat");
    // }
    //
    // #[test]
    // fn linear_gradient_pad() {
    //     linear_gradient(SpreadMethod::Pad, "pad");
    // }
    //
    // fn radial_gradient(spread_method: SpreadMethod, name: &str) {
    //     use crate::serialize::PageSerialize;
    //     let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
    //     canvas.fill_path(
    //         dummy_path(160.0),
    //         Transform::from_translate(0.0, 0.0).try_into().unwrap(),
    //         Fill {
    //             paint: Paint::RadialGradient(RadialGradient {
    //                 cx: FiniteF32::new(80.0).unwrap(),
    //                 cy: FiniteF32::new(80.0).unwrap(),
    //                 cr: FiniteF32::new(80.0).unwrap(),
    //                 fx: FiniteF32::new(80.0).unwrap(),
    //                 fy: FiniteF32::new(80.0).unwrap(),
    //                 fr: FiniteF32::new(0.0).unwrap(),
    //                 transform: TransformWrapper(
    //                     // Transform::from_scale(0.5, 0.5).pre_concat(Transform::from_rotate(45.0)),
    //                     // Transform::from_scale(0.5, 0.5),
    //                     Transform::identity(),
    //                 ),
    //                 spread_method,
    //                 stops: vec![
    //                     Stop {
    //                         offset: NormalizedF32::new(0.2).unwrap(),
    //                         color: Color::new_rgb(255, 0, 0),
    //                         opacity: NormalizedF32::ONE,
    //                     },
    //                     Stop {
    //                         offset: NormalizedF32::new(0.7).unwrap(),
    //                         color: Color::new_rgb(0, 0, 255),
    //                         opacity: NormalizedF32::ONE,
    //                     },
    //                 ],
    //             }),
    //             opacity: NormalizedF32::ONE,
    //             ..Fill::default()
    //         },
    //     );
    //
    //     let serialize_settings = SerializeSettings::default();
    //
    //     let chunk = PageSerialize::serialize(canvas, serialize_settings);
    //     let finished = chunk.finish();
    //
    //     write(&format!("radial_gradient_{}", name), &finished);
    // }
    //
    // #[test]
    // fn radial_gradient_reflect() {
    //     crate::canvas::tests::radial_gradient(SpreadMethod::Reflect, "reflect");
    // }
    //
    // #[test]
    // fn radial_gradient_repeat() {
    //     crate::canvas::tests::radial_gradient(SpreadMethod::Repeat, "repeat");
    // }
    //
    // #[test]
    // fn radial_gradient_pad() {
    //     crate::canvas::tests::radial_gradient(SpreadMethod::Pad, "pad");
    // }
    //
    // #[test]
    // fn clip_path() {
    //     use crate::serialize::PageSerialize;
    //     let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
    //
    //     let mut clipped = canvas.clipped(dummy_path(100.0), FillRule::NonZero);
    //
    //     clipped.fill_path(
    //         dummy_path(200.0),
    //         Transform::from_scale(1.0, 1.0),
    //         Fill {
    //             paint: Paint::Color(Color::new_rgb(200, 0, 0)),
    //             opacity: NormalizedF32::new(0.25).unwrap(),
    //             ..Fill::default()
    //         },
    //     );
    //
    //     clipped.finish();
    //
    //     let serialize_settings = SerializeSettings::default();
    //
    //     let chunk = PageSerialize::serialize(canvas, serialize_settings);
    //     let finished = chunk.finish();
    //
    //     write("clip_path", &finished);
    // }
    //
    // #[test]
    // fn pattern() {
    //     use crate::serialize::PageSerialize;
    //
    //     let mut pattern_canvas = Canvas::new(Size::from_wh(10.0, 10.0).unwrap());
    //     pattern_canvas.fill_path(
    //         dummy_path(5.0),
    //         Transform::default(),
    //         Fill {
    //             paint: Paint::Color(Color::new_rgb(0, 255, 0)),
    //             ..Fill::default()
    //         },
    //     );
    //
    //     pattern_canvas.fill_path(
    //         dummy_path(5.0),
    //         Transform::from_translate(5.0, 5.0),
    //         Fill {
    //             paint: Paint::Color(Color::new_rgb(0, 0, 255)),
    //             ..Fill::default()
    //         },
    //     );
    //
    //     let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
    //     canvas.fill_path(
    //         dummy_path(200.0),
    //         Transform::from_scale(2.0, 2.0).try_into().unwrap(),
    //         Fill {
    //             paint: Paint::Pattern(Arc::new(Pattern {
    //                 canvas: Arc::new(pattern_canvas),
    //                 transform: TransformWrapper(Transform::from_rotate_at(45.0, 2.5, 2.5)),
    //             })),
    //             opacity: NormalizedF32::ONE,
    //             ..Fill::default()
    //         },
    //     );
    //
    //     let serialize_settings = SerializeSettings::default();
    //
    //     let chunk = PageSerialize::serialize(canvas, serialize_settings);
    //     let finished = chunk.finish();
    //
    //     write("pattern", &finished);
    // }
    //
    // #[test]
    // fn mask_luminance() {
    //     use crate::serialize::PageSerialize;
    //
    //     let mut mask_canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
    //     mask_canvas.fill_path(
    //         dummy_path(200.0),
    //         Transform::default(),
    //         Fill {
    //             paint: Paint::Color(Color::new_rgb(255, 0, 0)),
    //             opacity: NormalizedF32::new(1.0).unwrap(),
    //             ..Fill::default()
    //         },
    //     );
    //
    //     let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
    //     let mut masked = canvas.masked(Mask::new(
    //         Arc::new(mask_canvas.byte_code),
    //         MaskType::Luminosity,
    //     ));
    //     masked.fill_path(
    //         dummy_path(200.0),
    //         Transform::identity().try_into().unwrap(),
    //         Fill {
    //             paint: Paint::Color(Color::new_rgb(255, 0, 0)),
    //             opacity: NormalizedF32::ONE,
    //             ..Fill::default()
    //         },
    //     );
    //
    //     masked.finish();
    //
    //     let serialize_settings = SerializeSettings::default();
    //
    //     let chunk = PageSerialize::serialize(canvas, serialize_settings);
    //     let finished = chunk.finish();
    //
    //     write("mask_luminance", &finished);
    // }
    //
    // #[test]
    // fn png_image() {
    //     use crate::serialize::PageSerialize;
    //     let image_data = include_bytes!("../data/image.png");
    //     let dynamic_image = image::load_from_memory(image_data).unwrap();
    //     let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
    //     canvas.draw_image(
    //         Image::new(&dynamic_image),
    //         Size::from_wh(50.0, 50.0).unwrap(),
    //         Transform::from_translate(20.0, 20.0),
    //     );
    //
    //     let serialize_settings = SerializeSettings::default();
    //
    //     let chunk = PageSerialize::serialize(canvas, serialize_settings);
    //     let finished = chunk.finish();
    //
    //     write("png_image", &finished);
    // }
    //
    // #[test]
    // fn glyph() {
    //     use crate::serialize::PageSerialize;
    //     let mut canvas = Canvas::new(Size::from_wh(200.0, 200.0).unwrap());
    //     let font_data =
    //         std::fs::read("/Users/lstampfl/Programming/GitHub/krilla/NotoSans.ttf").unwrap();
    //     let font = Font::new(Arc::new(font_data), Location::default()).unwrap();
    //     canvas.fill_path(
    //         dummy_path(30.0),
    //         Transform::from_translate(30.0, 30.0),
    //         Fill {
    //             paint: Paint::Color(Color::new_rgb(255, 0, 0)),
    //             opacity: NormalizedF32::ONE,
    //             rule: FillRule::default(),
    //         },
    //     );
    //     canvas.fill_glyph(
    //         GlyphId::new(36),
    //         font,
    //         FiniteF32::new(20.0).unwrap(),
    //         TransformWrapper(Transform::from_translate(30.0, 30.0)),
    //     );
    //
    //     let serialize_settings = SerializeSettings {
    //         compress: false,
    //         ..SerializeSettings::default()
    //     };
    //
    //     let chunk = PageSerialize::serialize(canvas, serialize_settings);
    //     let finished = chunk.finish();
    //
    //     write("glyph", &finished);
    // }

    // fn write(name: &str, data: &[u8]) {
    //     let _ = std::fs::write(format!("out/{name}.txt"), &data);
    //     let _ = std::fs::write(format!("out/{name}.pdf"), &data);
    // }
}
