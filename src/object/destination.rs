//! Destinations in a PDF document.
//!
//! In some cases, you might want to refer to other locations within the same document, for
//! example when defining the outline, or when link to a different section in the document
//! from a link. To achieve this, you can use destinations, which are associated with a page
//! and a specific location on that page.

use crate::error::{KrillaError, KrillaResult};
use crate::serialize::SerializerContext;
use std::hash::{Hash, Hasher};
use tiny_skia_path::{Point, Transform};

/// The type of destination.
#[derive(Hash)]
pub enum Destination {
    /// An XYZ destination.
    Xyz(XyzDestination),
}

impl Destination {
    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        destination: pdf_writer::writers::Destination,
    ) -> KrillaResult<()> {
        match self {
            Destination::Xyz(xyz) => xyz.serialize(sc, destination),
        }
    }
}

/// A destination pointing to a specific location at a specific page.
#[derive(Clone, Debug)]
pub struct XyzDestination {
    page_index: usize,
    point: Point,
}

impl Hash for XyzDestination {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.page_index.hash(state);
        self.point.x.to_bits().hash(state);
        self.point.y.to_bits().hash(state);
    }
}

impl From<XyzDestination> for Destination {
    fn from(val: XyzDestination) -> Self {
        Destination::Xyz(val)
    }
}

impl XyzDestination {
    /// Create a new XYZ destination. `page_index` should be the index (i.e. number) of the
    /// target page, and point indicates the specific location on that page that should be
    /// targeted. If the `page_index` is out of range, export will fail gracefully.
    pub fn new(page_index: usize, point: Point) -> Self {
        Self { page_index, point }
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        destination: pdf_writer::writers::Destination,
    ) -> KrillaResult<()> {
        // Silently ignore invalid destinations that point to a non-existing page.
        let page_info = sc
            .page_infos()
            .get(self.page_index)
            .ok_or(KrillaError::UserError(format!(
                "attempted to link to page {}, but document only has {} pages",
                self.page_index,
                sc.page_infos().len()
            )))?;
        let page_ref = page_info.ref_;
        let page_size = page_info.surface_size.height();

        let mut mapped_point = self.point;
        // Convert to PDF coordinates
        let invert_transform = Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, page_size);
        invert_transform.map_point(&mut mapped_point);

        destination
            .page(page_ref)
            .xyz(mapped_point.x, mapped_point.y, None);

        Ok(())
    }
}
