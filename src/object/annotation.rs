//! PDF annotations, allowing you to add extra "content" to specific pages.
//!
//! PDF has the concept of annotations, which allow you to associate certain regions of
//! a page with an "annotation". The PDF reference defines many different actions, however,
//! krilla does not and never will expose all of them. As of right now, the only annotations
//! that are supported are "link annotations", which allow you associate a certain region of
//! the page with a link.

use crate::error::KrillaResult;
use crate::object::action::Action;
use crate::object::destination::Destination;
use crate::page::page_root_transform;
use crate::serialize::SerializerContext;
use crate::util::RectExt;
use pdf_writer::types::AnnotationFlags;
use pdf_writer::{Chunk, Finish, Name, Ref};
use tiny_skia_path::Rect;

/// An annotation.
pub struct Annotation {
    pub(crate) annotation_type: AnnotationType,
    pub(crate) struct_parent: Option<i32>,
}

impl From<LinkAnnotation> for Annotation {
    fn from(value: LinkAnnotation) -> Self {
        Self {
            annotation_type: AnnotationType::Link(value),
            struct_parent: None,
        }
    }
}

impl Annotation {
    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        root_ref: Ref,
        page_height: f32,
    ) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();
        let mut annotation = chunk
            .indirect(root_ref)
            .start::<pdf_writer::writers::Annotation>();

        self.annotation_type
            .serialize_type(sc, &mut annotation, page_height)?;

        // Only required by PDF/A, but we always write this regardless.
        annotation.flags(AnnotationFlags::PRINT);

        if let Some(struct_parent) = self.struct_parent {
            annotation.struct_parent(struct_parent);
        }

        annotation.finish();

        Ok(chunk)
    }
}

/// A type of annotation.
pub enum AnnotationType {
    /// A link annotation.
    Link(LinkAnnotation),
}

impl AnnotationType {
    fn serialize_type(
        &self,
        sc: &mut SerializerContext,
        annotation: &mut pdf_writer::writers::Annotation,
        page_height: f32,
    ) -> KrillaResult<()> {
        match self {
            AnnotationType::Link(l) => l.serialize_type(sc, annotation, page_height),
        }
    }
}

/// An annotation target.
pub enum Target {
    /// A destination within the document.
    Destination(Destination),
    /// An action to be performed.
    Action(Action),
}

/// A link annotation.
pub struct LinkAnnotation {
    /// The bounding box of the link annotation that it should cover on the page.
    pub(crate) rect: Rect,
    /// The target of the link annotation.
    pub(crate) target: Target,
}

impl LinkAnnotation {
    /// Create a new link annotation.
    pub fn new(rect: Rect, target: Target) -> Self {
        Self { rect, target }
    }

    fn serialize_type(
        &self,
        sc: &mut SerializerContext,
        annotation: &mut pdf_writer::writers::Annotation,
        page_height: f32,
    ) -> KrillaResult<()> {
        annotation.subtype(pdf_writer::types::AnnotationType::Link);

        let actual_rect = self
            .rect
            .transform(page_root_transform(page_height))
            .unwrap();
        annotation.rect(actual_rect.to_pdf_rect());
        annotation.border(0.0, 0.0, 0.0, None);

        match &self.target {
            Target::Destination(destination) => destination.serialize(
                sc,
                annotation
                    .insert(Name(b"Dest"))
                    .start::<pdf_writer::writers::Destination>(),
            )?,
            Target::Action(action) => action.serialize(sc, annotation.action()),
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::document::{Document, PageSettings};
    use crate::object::action::LinkAction;
    use crate::object::annotation::{LinkAnnotation, Target};
    use crate::object::destination::XyzDestination;

    use crate::object::page::Page;

    use crate::tests::{green_fill, rect_to_path, red_fill};

    use crate::SerializeSettings;
    use krilla_macros::snapshot;
    use tiny_skia_path::{Point, Rect};

    #[snapshot(single_page)]
    fn annotation_to_link(page: &mut Page) {
        page.add_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
                Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
            )
            .into(),
        );
    }

    #[test]
    fn annotation_to_invalid_destination() {
        let mut d = Document::new_with(SerializeSettings::settings_1());
        let mut page = d.start_page_with(PageSettings::new(200.0, 200.0));
        page.add_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
                Target::Destination(XyzDestination::new(1, Point::from_xy(100.0, 100.0)).into()),
            )
            .into(),
        );
        page.finish();
        assert!(d.finish().is_err())
    }

    #[snapshot(document)]
    fn annotation_to_destination(d: &mut Document) {
        let mut page = d.start_page_with(PageSettings::new(200.0, 200.0));
        page.add_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(50.0, 0.0, 100.0, 100.0).unwrap(),
                Target::Destination(XyzDestination::new(1, Point::from_xy(100.0, 100.0)).into()),
            )
            .into(),
        );

        let mut surface = page.surface();
        surface.fill_path(&rect_to_path(50.0, 0.0, 150.0, 100.0), red_fill(1.0));
        surface.finish();
        page.finish();

        let mut page = d.start_page_with(PageSettings::new(200.0, 200.0));
        page.add_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(50.0, 100.0, 100.0, 100.0).unwrap(),
                Target::Destination(XyzDestination::new(0, Point::from_xy(0.0, 0.0)).into()),
            )
            .into(),
        );
        let mut my_surface = page.surface();
        my_surface.fill_path(&rect_to_path(50.0, 100.0, 150.0, 200.0), green_fill(1.0));

        my_surface.finish();
        page.finish();
    }
}
