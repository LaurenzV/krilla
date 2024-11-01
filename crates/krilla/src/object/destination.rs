//! Destinations in a PDF document.
//!
//! In some cases, you might want to refer to other locations within the same document, for
//! example when defining the outline, or when link to a different section in the document
//! from a link. To achieve this, you can use destinations, which are associated with a page
//! and a specific location on that page.

use crate::error::{KrillaError, KrillaResult};
use crate::serialize::SerializerContext;
use pdf_writer::{Chunk, Obj, Ref, Str};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tiny_skia_path::{Point, Transform};

/// The type of destination.
#[derive(Hash)]
pub enum Destination {
    /// An XYZ destination.
    Xyz(XyzDestination),
    /// A named destination.
    Named(NamedDestination),
}

impl Destination {
    pub(crate) fn serialize(&self, sc: &mut SerializerContext, buffer: Obj) -> KrillaResult<()> {
        match self {
            Destination::Xyz(xyz) => {
                let ref_ = sc.add_xyz_dest(xyz.clone());
                buffer.primitive(ref_);
                Ok(())
            }
            Destination::Named(named) => named.serialize(sc, buffer),
        }
    }
}

/// A destination associated with a name.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct NamedDestination {
    name: Arc<String>,
}

impl From<NamedDestination> for Destination {
    fn from(val: NamedDestination) -> Self {
        Destination::Named(val)
    }
}

impl NamedDestination {
    /// Create a new named destination.
    /// Note that named destinations need to be added via
    /// `add_named_destination` on [`Document`] before being used!
    ///
    /// [`Document`]: crate::Document
    pub fn new(name: String) -> Self {
        Self {
            name: Arc::new(name),
        }
    }

    pub(crate) fn inner(&self) -> &str {
        self.name.as_ref()
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        destination: Obj,
    ) -> KrillaResult<()> {
        sc.used_named_destinations.insert(self.clone());
        destination.primitive(Str(self.name.as_bytes()));
        Ok(())
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
        root_ref: Ref,
    ) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();
        let destination = chunk.destination(root_ref);

        let page_info = sc
            .page_infos()
            .get(self.page_index)
            .ok_or(KrillaError::UserError(format!(
                "attempted to link to page {}, but document only has {} pages",
                self.page_index + 1,
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

        Ok(chunk)
    }
}

#[cfg(test)]
mod tests {
    use crate::annotation::{LinkAnnotation, Target};
    use crate::destination::{NamedDestination, XyzDestination};
    use crate::error::KrillaError;
    use crate::tests::{blue_fill, green_fill, rect_to_path, red_fill};
    use crate::Document;
    use krilla_macros::snapshot;
    use tiny_skia_path::{Point, Rect};

    #[snapshot(document)]
    fn named_destination_basic(d: &mut Document) {
        let dest1 = NamedDestination::new("hi".to_string());
        let dest2 = NamedDestination::new("by".to_string());

        d.add_named_destination(
            dest1.clone(),
            XyzDestination::new(0, Point::from_xy(100.0, 100.0)),
        );
        d.add_named_destination(
            dest2.clone(),
            XyzDestination::new(1, Point::from_xy(0.0, 0.0)),
        );
        let mut page = d.start_page();
        page.add_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap(),
                Target::Destination(dest1.clone().into()),
            )
            .into(),
        );
        page.add_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(100.0, 100.0, 100.0, 100.0).unwrap(),
                Target::Destination(dest2.clone().into()),
            )
            .into(),
        );

        let mut surface = page.surface();
        surface.fill_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), red_fill(1.0));
        surface.fill_path(&rect_to_path(100.0, 100.0, 200.0, 200.0), green_fill(1.0));
        surface.finish();
        page.finish();

        let mut page = d.start_page();
        page.add_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap(),
                Target::Destination(dest1.clone().into()),
            )
            .into(),
        );
        let mut surface = page.surface();
        surface.fill_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), blue_fill(1.0));
        surface.finish();
        page.finish();
    }

    #[test]
    fn named_destination_not_registered() {
        let mut document = Document::new();
        let dest = NamedDestination::new("hi".to_string());

        let mut page = document.start_page();
        page.add_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap(),
                Target::Destination(dest.clone().into()),
            )
            .into(),
        );
        page.finish();

        assert!(matches!(document.finish(), Err(KrillaError::UserError(_))));
    }
}
