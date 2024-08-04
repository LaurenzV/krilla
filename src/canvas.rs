use crate::font::Glyph;
use crate::object::image::Image;
use crate::object::mask::Mask;
use crate::object::shading_function::ShadingFunction;
use crate::serialize::{PageSerialize, SerializeSettings, SerializerContext};
use crate::stream::{Stream, StreamBuilder};
use crate::util::{deflate, RectExt};
use crate::{Fill, FillRule, Stroke};
use fontdb::{Database, ID};
use pdf_writer::types::BlendMode;
use pdf_writer::{Chunk, Filter, Finish, Pdf};
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, Path, Size, Transform};
use usvg::NormalizedF32;

pub struct Page<'a> {
    pub size: Size,
    pub serializer_context: SerializerContext,
}

impl<'a> Page<'a> {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            serializer_context: SerializerContext::new(SerializeSettings::default()),
        }
    }

    pub fn builder(&'a mut self) -> CanvasBuilder<'a> {
        let size = self.size;
        CanvasBuilder::new_flipped(&mut self.serializer_context, size)
    }

    pub fn finish(self) -> SerializerContext {
        self.serializer_context
    }
}

impl PageSerialize for Stream {
    fn serialize(self, mut sc: SerializerContext, fontdb: &Database, size: Size) -> Pdf {
        let catalog_ref = sc.new_ref();
        let page_tree_ref = sc.new_ref();
        let page_ref = sc.new_ref();
        let content_ref = sc.new_ref();

        let mut chunk = Chunk::new();
        chunk.pages(page_tree_ref).count(1).kids([page_ref]);

        if sc.serialize_settings.compress {
            let deflated = deflate(self.content());

            let mut stream = chunk.stream(content_ref, &deflated);
            stream.filter(Filter::FlateDecode);
            stream.finish();
        } else {
            chunk.stream(content_ref, self.content());
        }

        let mut page = chunk.page(page_ref);
        self.resource_dictionary()
            .to_pdf_resources(&mut sc, &mut page.resources());

        page.media_box(size.to_rect(0.0, 0.0).unwrap().to_pdf_rect());
        page.parent(page_tree_ref);
        page.contents(content_ref);
        page.finish();
        let cached_chunk = sc.finish(fontdb);

        let mut pdf = Pdf::new();
        pdf.catalog(catalog_ref).pages(page_tree_ref);
        pdf.extend(&chunk);
        pdf.extend(&cached_chunk);

        pdf
    }
}

pub struct CanvasBuilder<'a> {
    sc: &'a mut SerializerContext,
    root_builder: StreamBuilder,
    sub_builders: Vec<StreamBuilder>,
    masks: Vec<Mask>,
    opacities: Vec<NormalizedF32>,
}

impl<'a> CanvasBuilder<'a> {
    pub fn new(sc: &'a mut SerializerContext) -> Self {
        Self {
            sc,
            root_builder: StreamBuilder::new(),
            sub_builders: Vec::new(),
            masks: Vec::new(),
            opacities: Vec::new(),
        }
    }

    pub fn new_flipped(sc: &'a mut SerializerContext, size: Size) -> Self {
        let mut root_builder = StreamBuilder::new();
        // Invert the y-axis.
        root_builder.concat_transform(&Transform::from_row(
            1.0,
            0.0,
            0.0,
            -1.0,
            0.0,
            size.height(),
        ));

        Self {
            sc,
            root_builder,
            sub_builders: Vec::new(),
            masks: Vec::new(),
            opacities: Vec::new(),
        }
    }

    pub fn sub_canvas(&mut self) -> CanvasBuilder {
        CanvasBuilder::new(&mut self.sc)
    }

    pub fn push_transform(&mut self, transform: &Transform) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).save_graphics_state();
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .concat_transform(transform);
    }

    pub fn pop_transform(&mut self) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).restore_graphics_state();
    }

    pub fn push_blend_mode(&mut self, blend_mode: BlendMode) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).save_graphics_state();
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .set_blend_mode(blend_mode);
    }

    pub fn pop_blend_mode(&mut self) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).restore_graphics_state();
    }

    pub fn fill_path<'b>(&'b mut self, path: &Path, fill: &Fill) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_path(path, fill, self.sc);
    }

    pub fn stroke_path<'b>(&'b mut self, path: &Path, stroke: &Stroke) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .stroke_path(path, stroke, self.sc);
    }

    pub fn push_clip_path(&mut self, path: &Path, clip_rule: &FillRule) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .push_clip_path(path, clip_rule);
    }

    pub fn pop_clip_path(&mut self) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).pop_clip_path();
    }

    pub(crate) fn fill_path_impl(&mut self, path: &Path, fill: &Fill, no_fill: bool) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_path_impl(path, fill, self.sc, no_fill)
    }

    pub fn invisible_glyph(
        &mut self,
        glyph: Glyph,
        font_id: ID,
        fontdb: &mut Database,
        size: FiniteF32,
        transform: &Transform,
    ) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .invisible_glyph(glyph, font_id, fontdb, size, transform, self.sc);
    }

    pub fn fill_glyph<'b>(
        &'b mut self,
        glyph: Glyph,
        font_id: ID,
        fontdb: &mut Database,
        size: FiniteF32,
        transform: &Transform,
        fill: &Fill,
    ) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .fill_glyph(glyph, font_id, fontdb, size, transform, fill, self.sc);
    }

    pub fn stroke_glyph<'b>(
        &'b mut self,
        glyph_id: Glyph,
        font_id: ID,
        fontdb: &mut Database,
        size: FiniteF32,
        transform: &Transform,
        stroke: &Stroke,
    ) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .stroke_glyph(glyph_id, font_id, fontdb, size, transform, stroke, self.sc);
    }

    fn cur_builder<'b>(
        root_builder: &'b mut StreamBuilder,
        sub_builders: &'b mut [StreamBuilder],
    ) -> &'b mut StreamBuilder {
        sub_builders.last_mut().unwrap_or(root_builder)
    }

    pub fn push_mask(&mut self, mask: Mask) {
        self.sub_builders.push(StreamBuilder::new());
        self.masks.push(mask);
    }

    pub fn pop_mask(&mut self) {
        let stream = self.sub_builders.pop().unwrap().finish();
        let mask = self.masks.pop().unwrap();
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).draw_masked(mask, stream);
    }

    pub fn draw_opacified_stream(&mut self, opacity: NormalizedF32, stream: Arc<Stream>) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .draw_opacified(opacity, stream)
    }

    pub fn push_opacified(&mut self, opacity: NormalizedF32) {
        self.sub_builders.push(StreamBuilder::new());
        self.opacities.push(opacity);
    }

    pub fn pop_opacified(&mut self) {
        let stream = self.sub_builders.pop().unwrap().finish();
        let opacity = self.opacities.pop().unwrap();
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders)
            .draw_opacified(opacity, Arc::new(stream));
    }

    pub fn push_isolated(&mut self) {
        self.sub_builders.push(StreamBuilder::new());
    }

    pub fn pop_isolated(&mut self) {
        let stream = self.sub_builders.pop().unwrap().finish();
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).draw_isolated(stream);
    }

    pub fn draw_image(&mut self, image: Image, size: Size) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).draw_image(image, size);
    }

    pub(crate) fn draw_shading(&mut self, shading: &ShadingFunction) {
        Self::cur_builder(&mut self.root_builder, &mut self.sub_builders).draw_shading(shading);
    }

    pub fn finish(self) -> Stream {
        self.root_builder.finish()
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
