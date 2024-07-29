use crate::canvas::{Canvas, Surface};
use crate::serialize::{PageSerialize, SerializeSettings};
use crate::transform::TransformWrapper;
use crate::util::Prehashed;
use skrifa::instance::Location;
use skrifa::outline::OutlinePen;
use skrifa::prelude::{LocationRef, Size};
use skrifa::raw::{FontRead, Offset, TableProvider};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use std::ops::Deref;
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, Path, PathBuilder, Rect, Transform};

pub mod bitmap;
pub mod colr;
pub mod outline;
pub mod svg;

struct OutlineBuilder(PathBuilder);

impl OutlineBuilder {
    pub fn new() -> Self {
        Self(PathBuilder::new())
    }

    pub fn finish(self) -> Option<Path> {
        self.0.finish()
    }
}

impl OutlinePen for OutlineBuilder {
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

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct FontWrapper {
    data: Arc<Vec<u8>>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Repr {
    font_wrapper: FontWrapper,
    location: Location,
    units_per_em: u16,
    // Note that the bbox only applied to non-variable font settings
    global_bbox: Rect,
    has_color_table: bool,
    ascent: FiniteF32,
    descent: FiniteF32,
    cap_height: Option<FiniteF32>,
    is_monospaced: bool,
    italic_angle: FiniteF32,
    weight: FiniteF32,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Font(Arc<Prehashed<Repr>>);

impl Font {
    pub fn new(data: Arc<Vec<u8>>, location: Location) -> Option<Self> {
        let font_wrapper = FontWrapper { data };
        let font_ref = font_wrapper.tables();
        let metrics = font_ref.metrics(Size::unscaled(), &location);
        let ascent = FiniteF32::new(metrics.ascent).unwrap();
        let descent = FiniteF32::new(metrics.descent).unwrap();
        let is_monospaced = metrics.is_monospace;
        let cap_height = metrics.cap_height.map(|n| FiniteF32::new(n).unwrap());
        let italic_angle = FiniteF32::new(metrics.italic_angle).unwrap();
        let weight = FiniteF32::new(font_ref.attributes().weight.value()).unwrap();
        let units_per_em = metrics.units_per_em;
        let global_bbox = metrics
            .bounds
            .and_then(|b| Rect::from_xywh(b.x_min, b.y_min, b.x_max, b.y_max))
            .unwrap_or(Rect::from_xywh(
                0.0,
                0.0,
                units_per_em as f32,
                units_per_em as f32,
            )?);

        let has_color_table = font_ref.svg().is_ok()
            || font_ref.colr().is_ok()
            || font_ref.sbix().is_ok()
            || font_ref.cff2().is_ok();

        let font_wrapper = Self(Arc::new(Prehashed::new(Repr {
            font_wrapper,
            units_per_em,
            ascent,
            cap_height,
            descent,
            is_monospaced,
            weight,
            italic_angle,
            global_bbox,
            has_color_table,
            location,
        })));

        Some(font_wrapper)
    }

    pub fn cap_height(&self) -> Option<f32> {
        self.0.cap_height.map(|n| n.get())
    }

    pub fn ascent(&self) -> f32 {
        self.0.ascent.get()
    }

    pub fn weight(&self) -> f32 {
        self.0.weight.get()
    }

    pub fn descent(&self) -> f32 {
        self.0.descent.get()
    }

    pub fn is_monospaced(&self) -> bool {
        self.0.is_monospaced
    }

    pub fn italic_angle(&self) -> f32 {
        self.0.italic_angle.get()
    }

    pub fn glyph_advance(&self, glyph_id: GlyphId) -> Option<f32> {
        let location_ref = &self.0.location;
        let g_metrics = self
            .font_ref()
            .glyph_metrics(Size::unscaled(), location_ref);
        g_metrics.advance_width(glyph_id)
    }

    pub fn font_ref(&self) -> FontRef {
        FontRef::from_index(self.0.font_wrapper.data.as_slice(), 0).unwrap()
    }

    pub fn units_per_em(&self) -> u16 {
        self.0.units_per_em
    }

    pub fn bbox(&self) -> Rect {
        self.0.global_bbox
    }

    pub fn location_ref(&self) -> LocationRef {
        (&self.0.location).into()
    }

    pub fn has_color_table(&self) -> bool {
        self.0.has_color_table
    }
}

impl FontWrapper {
    pub fn tables(&self) -> FontRef {
        FontRef::from_index(&self.data, 0).unwrap()
    }
}

#[cfg(test)]
fn draw(
    font: &Font,
    glyphs: &[u32],
    name: &str,
    single_glyph: impl Fn(&Font, GlyphId) -> Option<Canvas>,
) {
    let metrics = font
        .font_ref()
        .metrics(Size::unscaled(), font.location_ref());
    let num_glyphs = glyphs.len();
    let width = 400;

    let size = 40u32;
    let num_cols = width / size;
    let height = (num_glyphs as f32 / num_cols as f32).ceil() as u32 * size;
    let units_per_em = metrics.units_per_em as f32;
    let mut cur_point = 0;

    let mut parent_canvas = Canvas::new(crate::Size::from_wh(width as f32, height as f32).unwrap());

    for i in glyphs.iter().copied() {
        fn get_transform(cur_point: u32, size: u32, num_cols: u32, units_per_em: f32) -> Transform {
            let el = cur_point / size;
            let col = el % num_cols;
            let row = el / num_cols;

            Transform::from_row(
                1.0,
                0.0,
                0.0,
                1.0,
                col as f32 * size as f32,
                row as f32 * size as f32,
            )
        }

        let mut transformed =
            parent_canvas.transformed(get_transform(cur_point, size, num_cols, units_per_em));
        transformed.draw_glyph(
            GlyphId::new(i),
            font.clone(),
            FiniteF32::new(size as f32).unwrap(),
            TransformWrapper(Transform::from_translate(0.0, size as f32)),
        );
        transformed.finish();

        cur_point += size;
    }

    let pdf = parent_canvas.serialize(SerializeSettings::default());
    let finished = pdf.finish();
    let _ = std::fs::write(format!("out/{}.pdf", name), &finished);
    let _ = std::fs::write(format!("out/{}.txt", name), &finished);
}
