use crate::canvas::{Canvas, Surface};
use crate::serialize::{PageSerialize, SerializeSettings};
use crate::util::Prehashed;
use skrifa::instance::Location;
use skrifa::outline::OutlinePen;
use skrifa::prelude::{LocationRef, Size};
use skrifa::raw::types::Offset32;
use skrifa::raw::{FontData, FontRead, Offset, TableDirectory, TableProvider};
use skrifa::{FontRef, GlyphId, MetadataProvider, Tag};
use std::collections::BTreeMap;
use std::sync::Arc;
use tiny_skia_path::{Path, PathBuilder, Rect};

mod colr;
mod outline;
mod svg;

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
    records: BTreeMap<Tag, (usize, usize)>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Repr {
    font_wrapper: FontWrapper,
    location: Location,
    units_per_em: u16,
    // Note that the bbox only applied to non-variable font settings
    global_bbox: Rect,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Font(Arc<Prehashed<Repr>>);

impl Font {
    pub fn new(data: Arc<Vec<u8>>, location: Location) -> Option<Self> {
        let mut records = BTreeMap::new();
        let font_data = FontData::new(data.as_slice());
        let table_directory = TableDirectory::read(font_data).ok()?;

        for record in table_directory.table_records() {
            let start = Offset32::new(record.offset()).non_null()?;
            let len = record.length() as usize;
            records.insert(record.tag.get(), (start, start + len));
        }

        let font_wrapper = FontWrapper { data, records };
        let metrics = font_wrapper.tables().metrics(Size::unscaled(), &location);
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

        let font_wrapper = Self(Arc::new(Prehashed::new(Repr {
            font_wrapper,
            units_per_em,
            global_bbox,
            location,
        })));

        Some(font_wrapper)
    }

    pub fn font_ref(&self) -> FontTables {
        self.0.font_wrapper.tables()
    }

    pub fn units_per_em(&self) -> u16 {
        self.0.units_per_em
    }

    pub fn bbox(&self) -> Rect {
        self.0.global_bbox
    }

    pub fn location_ref<'a>(&'a self) -> LocationRef<'a> {
        (&self.0.location).into()
    }
}

impl FontWrapper {
    pub fn tables(&self) -> FontTables {
        FontTables(self)
    }
}

pub struct FontTables<'a>(&'a FontWrapper);

impl<'a> TableProvider<'a> for FontTables<'a> {
    fn data_for_tag(&self, tag: Tag) -> Option<FontData<'a>> {
        let (start, end) = self.0.records.get(&tag)?;
        Some(FontData::new(self.0.data.as_slice().get(*start..*end)?))
    }
}

#[cfg(test)]
fn draw(
    font_ref: &FontRef,
    location_ref: LocationRef,
    glyphs: &[u32],
    name: &str,
    single_glyph: impl Fn(&FontRef, LocationRef, GlyphId) -> Option<Canvas>,
) {
    let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), location_ref);
    let num_glyphs = glyphs.len();
    let width = 2000;

    let size = 40u32;
    let num_cols = width / size;
    let height = (num_glyphs as f32 / num_cols as f32).ceil() as u32 * size;
    let units_per_em = metrics.units_per_em as f32;
    let mut cur_point = 0;

    let mut parent_canvas = Canvas::new(crate::Size::from_wh(width as f32, height as f32).unwrap());

    for i in glyphs.iter().copied() {
        let Some(canvas) = single_glyph(&font_ref, location_ref, GlyphId::new(i)) else {
            continue;
        };

        fn get_transform(
            cur_point: u32,
            size: u32,
            num_cols: u32,
            units_per_em: f32,
        ) -> crate::Transform {
            let el = cur_point / size;
            let col = el % num_cols;
            let row = el / num_cols;

            crate::Transform::from_row(
                (1.0 / units_per_em) * size as f32,
                0.0,
                0.0,
                (1.0 / units_per_em) * size as f32,
                col as f32 * size as f32,
                row as f32 * size as f32,
            )
        }

        let mut transformed = parent_canvas.transformed(
            get_transform(cur_point, size, num_cols, units_per_em).pre_concat(
                tiny_skia_path::Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, units_per_em as f32),
            ),
        );
        transformed.draw_canvas(canvas);
        transformed.finish();

        cur_point += size;
    }

    let pdf = parent_canvas.serialize(SerializeSettings::default());
    let finished = pdf.finish();
    let _ = std::fs::write(format!("out/{}.pdf", name), &finished);
    let _ = std::fs::write(format!("out/{}.txt", name), &finished);
}
