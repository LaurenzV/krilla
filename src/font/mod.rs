use skrifa::outline::OutlinePen;
use skrifa::raw::traversal::SomeArray;
use skrifa::raw::types::Offset32;
use skrifa::raw::{FontData, FontRead, Offset, TableDirectory, TableProvider};
use skrifa::Tag;
use std::collections::BTreeMap;
use std::sync::Arc;
use tiny_skia_path::{Path, PathBuilder};
use crate::util::Prehashed;

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
pub struct Repr {
    data: Arc<Vec<u8>>,
    records: BTreeMap<Tag, (usize, usize)>,
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

        Some(Self(Arc::new(Prehashed::new(Repr { data, records }))))
    }
}

impl Font {
    pub fn tables(&self) -> FontTables {
        FontTables(self)
    }
}

pub struct FontTables<'a>(&'a Font);

impl<'a> TableProvider<'a> for FontTables<'a> {
    fn data_for_tag(&self, tag: Tag) -> Option<FontData<'a>> {
        let (start, end) = self.0 .0.records.get(&tag)?;
        Some(FontData::new(self.0 .0.data.as_slice().get(*start..*end)?))
    }
}
