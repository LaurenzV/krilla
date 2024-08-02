use crate::util::Prehashed;
use skrifa::instance::Location;
use skrifa::outline::OutlinePen;
use skrifa::prelude::{LocationRef, Size};
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use std::rc::Rc;
use tiny_skia_path::{FiniteF32, Path, PathBuilder, Rect};

pub mod bitmap;
pub mod colr;
pub mod outline;
mod parley;
pub mod svg;

pub struct Glyph {
    pub glyph_id: GlyphId,
    pub string: String,
}

impl Glyph {
    pub fn new(glyph_id: GlyphId, string: String) -> Self {
        Self { glyph_id, string }
    }
}

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
    data: Rc<Vec<u8>>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Repr {
    font_wrapper: FontWrapper,
    location: Location,
    units_per_em: u16,
    // Note that the bbox only applied to non-variable font settings
    global_bbox: Rect,
    is_type3_font: bool,
    ascent: FiniteF32,
    descent: FiniteF32,
    cap_height: Option<FiniteF32>,
    is_monospaced: bool,
    italic_angle: FiniteF32,
    weight: FiniteF32,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Font(Rc<Prehashed<Repr>>);

impl Font {
    pub fn new(data: Rc<Vec<u8>>, location: Location) -> Option<Self> {
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

        // Right now, we decide whether to embed a font as a Type3 font solely based on whether one of these
        // tables exist. This is not the most "efficient" method, because it is possible a font has a `COLR` table,
        // but there are still some glyphs which are not in COLR but still in `glyf` or `CFF`. In this case,
        // we would still choose a Type3 font for the outlines, even though they could be embedded as a CID font.
        // For now, we make the simplifying assumption that a font is either mapped to a series of Type3 fonts
        // or to a single CID font, but not a mix of both.
        let is_type3_font = font_ref.svg().is_ok()
            || font_ref.colr().is_ok()
            || font_ref.sbix().is_ok()
            || font_ref.cff2().is_ok();

        let font_wrapper = Self(Rc::new(Prehashed::new(Repr {
            font_wrapper,
            units_per_em,
            ascent,
            cap_height,
            descent,
            is_monospaced,
            weight,
            italic_angle,
            global_bbox,
            is_type3_font,
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

    pub fn is_type3_font(&self) -> bool {
        self.0.is_type3_font
    }
}

impl FontWrapper {
    pub fn tables(&self) -> FontRef {
        FontRef::from_index(&self.data, 0).unwrap()
    }
}

#[cfg(test)]
fn draw(font: &Font, glyphs: Option<Vec<(GlyphId, String)>>, name: &str) {
    use crate::canvas::Page;
    use crate::serialize::PageSerialize;
    use crate::Transform;

    let glyphs = glyphs.unwrap_or_else(|| {
        let file =
            std::fs::read("/Users/lstampfl/Programming/GitHub/krilla/src/font/emojis.txt").unwrap();
        let file = std::str::from_utf8(&file).unwrap();
        file.chars()
            .filter_map(|c| {
                font.font_ref()
                    .cmap()
                    .unwrap()
                    .map_codepoint(c)
                    .map(|g| (g, c.to_string()))
            })
            .collect::<Vec<_>>()
    });

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

    let page_size = tiny_skia_path::Size::from_wh(width as f32, height as f32).unwrap();
    let mut page = Page::new(page_size);
    let mut builder = page.builder();

    for (i, text) in glyphs.iter().cloned() {
        fn get_transform(cur_point: u32, size: u32, num_cols: u32, _: f32) -> Transform {
            let el = cur_point / size;
            let col = el % num_cols;
            let row = el / num_cols;

            Transform::from_row(
                1.0,
                0.0,
                0.0,
                1.0,
                col as f32 * size as f32,
                (row + 1) as f32 * size as f32,
            )
            // .pre_concat(Transform::from_scale(
            //     size as f32 / units_per_em,
            //     size as f32 / units_per_em,
            // ))
        }

        builder.push_transform(&get_transform(cur_point, size, num_cols, units_per_em));
        builder.fill_glyph(
            Glyph::new(i, text),
            font.clone(),
            FiniteF32::new(size as f32).unwrap(),
            &Transform::identity(),
            &crate::Fill::default(),
        );
        // let res = single_glyph(&font, GlyphId::new(i), &mut builder);
        builder.pop_transform();

        cur_point += size;
    }

    let stream = builder.finish();
    let sc = page.finish();

    let pdf = stream.serialize(sc, page_size);
    let finished = pdf.finish();
    let _ = std::fs::write(format!("out/{}.pdf", name), &finished);
    let _ = std::fs::write(format!("out/{}.txt", name), &finished);
}
