use skrifa::color::{Brush, ColorPainter, CompositeMode};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::LocationRef;
use skrifa::raw::TableProvider;
use skrifa::raw::tables::colr::PaintTransform;
use skrifa::raw::types::BoundingBox;
use tiny_skia_path::{NormalizedF32, Path, PathBuilder, PathVerb, Size, Transform};
use crate::canvas::Canvas;
use crate::{Fill, FillRule};
use crate::color::Color;
use crate::paint::Paint;
use crate::transform::TransformWrapper;

struct GlyphBuilder(PathBuilder);

impl GlyphBuilder {
    pub fn finish(self) -> Option<Path> {
        self.0.finish()
    }
}

impl OutlinePen for GlyphBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
       self.0.quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.0.close()
    }
}

struct ColrCanvas<'a> {
    font: &'a FontRef<'a>,
    clips: Vec<Vec<Path>>,
    transforms: Vec<Transform>,
    canvas: Canvas
}

impl<'a> ColrCanvas<'a> {
    pub fn new(font_ref: &'a FontRef<'a>) -> Self {
        let mut canvas = Canvas::new(Size::from_wh(1000.0, 1000.0).unwrap());
        canvas.transform(Transform::from_row(
            1.0,
            0.0,
            0.0,
            -1.0,
            0.0,
            (font_ref.metrics(
                    skrifa::instance::Size::unscaled(), LocationRef::default()
                ).units_per_em as f32
        )));

        Self {
            font: font_ref,
            transforms: vec![Transform::identity()],
            clips: vec![vec![]],
            canvas
        }
    }
}

impl ColorPainter for ColrCanvas<'_> {
    fn push_transform(&mut self, transform: skrifa::color::Transform) {
        let new_transform = self.transforms.last().unwrap().pre_concat(Transform::from_row(transform.xx, transform.yx, transform.xy, transform.yy, transform.dx, transform.dy));
        self.transforms.push(new_transform);
    }

    fn pop_transform(&mut self) {
        self.transforms.pop();
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        let mut old = self.clips.last().unwrap().clone();

        let mut glyph_builder = GlyphBuilder(PathBuilder::new());
        let outline_glyphs = self.font.outline_glyphs();
        let outline_glyph = outline_glyphs.get(glyph_id).unwrap();
        outline_glyph.draw(DrawSettings::unhinted(skrifa::instance::Size::unscaled(), LocationRef::default()), &mut glyph_builder).unwrap();
        let path = glyph_builder.finish().unwrap()
            .transform(*self.transforms.last().unwrap()).unwrap();

        old.push(path);

        self.clips.push(old);
    }

    fn push_clip_box(&mut self, clip_box: BoundingBox<f32>) {
        let mut old = self.clips.last().unwrap().clone();

        let mut path_builder = PathBuilder::new();
        path_builder.move_to(clip_box.x_min, clip_box.y_min);
        path_builder.line_to(clip_box.x_min, clip_box.y_max);
        path_builder.line_to(clip_box.x_max, clip_box.y_max);
        path_builder.line_to(clip_box.x_max, clip_box.y_min);
        path_builder.close();

        let path = path_builder.finish().unwrap().transform(*self.transforms.last().unwrap()).unwrap();
        old.push(path);

        self.clips.push(old);
    }

    fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn fill(&mut self, brush: Brush<'_>) {
        self.canvas.push_layer();


        for clip_path in self.clips.last().unwrap() {
            self.canvas.set_clip_path(clip_path.clone(), FillRule::NonZero);
        }

        let mut path_builder = PathBuilder::new();
        path_builder.move_to(0.0, 0.0);
        path_builder.line_to(1000.0, 0.0);
        path_builder.line_to(1000.0, 1000.0);
        path_builder.line_to(0.0, 1000.0);
        path_builder.close();

        let fill = match brush {
            Brush::Solid { palette_index, alpha } => {
                let color = self.font.cpal().unwrap().color_records_array().unwrap().unwrap()[palette_index as usize];

                Fill {
                    paint: Paint::Color(Color::new_rgb(color.red,color.green, color.blue)),
                    opacity: NormalizedF32::ONE,
                    rule: Default::default(),
                }
            },
            Brush::LinearGradient { .. } => Fill::default(),
            Brush::RadialGradient { .. } => Fill::default(),
            Brush::SweepGradient { .. } => Fill::default(),
        };

        self.canvas.fill_path(
            path_builder.finish().unwrap(),
            Transform::identity(),
            fill
        );

        self.canvas.pop_layer();
    }

    fn push_layer(&mut self, composite_mode: CompositeMode) {
        // self.canvas.p
    }

    fn pop_layer(&mut self) {
        // todo!()
    }
}

#[cfg(test)]
mod tests {
    use skrifa::{FontRef, GlyphId, MetadataProvider};
    use skrifa::prelude::LocationRef;
    use crate::colr::ColrCanvas;
    use crate::serialize::{PageSerialize, SerializeSettings};

    #[test]
    fn try_it() {
        let font_data = std::fs::read("/Users/lstampfl/Programming/GitHub/krilla/test_glyphs-glyf_colr_1.ttf").unwrap();
        let font_ref = FontRef::from_index(&font_data, 0).unwrap();
        let mut colr_canvas = ColrCanvas::new(&font_ref);

        let colr_glyphs = font_ref.color_glyphs();
        let colr_glyph = colr_glyphs.get(GlyphId::new(120)).unwrap();
        colr_glyph.paint(LocationRef::default(), &mut colr_canvas);
        eprintln!("{:#?}", colr_canvas.canvas);

        let pdf = colr_canvas.canvas.serialize(SerializeSettings::default());
        std::fs::write("out.pdf", pdf.finish());

        assert!(false);
    }
}