use crate::util::Prehashed;
use skrifa::outline::OutlinePen;
use skrifa::prelude::{LocationRef, Size};
use skrifa::raw::traversal::SomeArray;
use skrifa::raw::types::{BoundingBox, Offset32};
use skrifa::raw::{FontData, FontRead, Offset, TableDirectory, TableProvider};
use skrifa::{MetadataProvider, Tag};
use std::collections::BTreeMap;
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, Path, PathBuilder, Rect};

mod colr;
mod outline;

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
    units_per_em: u16,
    global_bbox: Rect,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Font(Arc<Prehashed<Repr>>);

impl Font {
    pub fn new(data: Arc<Vec<u8>>) -> Option<Self> {
        let mut records = BTreeMap::new();
        let font_data = FontData::new(data.as_slice());
        let table_directory = TableDirectory::read(font_data).ok()?;

        for record in table_directory.table_records() {
            let start = Offset32::new(record.offset()).non_null()?;
            let len = record.length() as usize;
            records.insert(record.tag.get(), (start, start + len));
        }

        let font_wrapper = FontWrapper { data, records };
        let metrics = font_wrapper
            .tables()
            .metrics(Size::unscaled(), LocationRef::default());
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
