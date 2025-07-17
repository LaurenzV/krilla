//! PDF annotations, allowing you to add extra "content" to specific pages.
//!
//! PDF has the concept of annotations, which allow you to associate certain regions of
//! a page with an "annotation". The PDF reference defines many different actions, however,
//! krilla does not and never will expose all of them. As of right now, the only annotations
//! that are supported are "link annotations", which allow you associate a certain region of
//! the page with a link.

use core::f32;

use pdf_writer::types::AnnotationFlags;
use pdf_writer::{Chunk, Finish, Name, Ref, TextStr};

use crate::configure::{PdfVersion, ValidationError};
use crate::error::KrillaResult;
use crate::geom::{Quadrilateral, Rect};
use crate::interactive::action::Action;
use crate::interactive::destination::Destination;
use crate::page::page_root_transform;
use crate::serialize::SerializeContext;

/// An annotation.
pub struct Annotation {
    pub(crate) annotation_type: AnnotationType,
    pub(crate) alt: Option<String>,
    pub(crate) struct_parent: Option<i32>,
}

impl Annotation {
    /// Create a new link annotation with some alt text.
    ///
    /// Note that the alt text might be required in some cases, for example
    /// when exporting to PDF/UA.
    pub fn new_link(annotation: LinkAnnotation, alt_text: Option<String>) -> Self {
        Self {
            annotation_type: AnnotationType::Link(annotation),
            alt: alt_text,
            struct_parent: None,
        }
    }
}

impl From<LinkAnnotation> for Annotation {
    fn from(value: LinkAnnotation) -> Self {
        Self {
            annotation_type: AnnotationType::Link(value),
            alt: None,
            struct_parent: None,
        }
    }
}

impl Annotation {
    pub(crate) fn serialize(
        &self,
        sc: &mut SerializeContext,
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

        if let Some(alt_text) = &self.alt {
            annotation.contents(TextStr(alt_text));
        } else {
            sc.register_validation_error(ValidationError::MissingAnnotationAltText);
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
        sc: &mut SerializeContext,
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
    pub(crate) rect: Rect,
    pub(crate) quad_points: Option<Vec<Quadrilateral>>,
    pub(crate) target: Target,
}

impl LinkAnnotation {
    /// Create a new link annotation.
    ///
    /// `target`: The target of the link annotation.
    /// `quad_points`: An array of quadrilaterals that define where the link
    /// annotation should be activated. This is useful if you for example have
    /// a link annotation that is broken to one or multiple lines.
    pub fn new(quad_points: Vec<Quadrilateral>, target: Target) -> Self {
        assert!(!quad_points.is_empty());

        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for point in quad_points.iter().flat_map(|q| q.0) {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }

        // Expand the bounding box by a little. There is a bug in adobe acrobat
        // that sometimes prevents the quadpoints from being used if the quad
        // points lie exactly on the bounding rectangle.
        const EPSILON: f32 = 0.001;
        let rect = Rect::from_ltrb(
            min_x - EPSILON,
            min_y - EPSILON,
            max_x + EPSILON,
            max_y + EPSILON,
        )
        .unwrap();

        Self {
            rect,
            quad_points: Some(quad_points),
            target,
        }
    }

    /// Create a new link annotation.
    ///
    /// `rect`: The bounding box of the link annotation that it should cover on the page.
    /// `target`: The target of the link annotation.
    pub fn new_rect(rect: Rect, target: Target) -> Self {
        Self {
            rect,
            quad_points: None,
            target,
        }
    }

    fn serialize_type(
        &self,
        sc: &mut SerializeContext,
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

        if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf16 {
            self.quad_points.as_ref().map(|p| {
                annotation.quad_points(p.iter().flat_map(|q| q.0).flat_map(|p| {
                    let mut p = p.to_tsp();
                    page_root_transform(page_height).to_tsp().map_point(&mut p);
                    [p.x, p.y]
                }))
            });
        }

        match &self.target {
            Target::Destination(destination) => {
                destination.serialize(sc, annotation.insert(Name(b"Dest")))?
            }
            Target::Action(action) => action.serialize(sc, annotation.action())?,
        };

        Ok(())
    }
}
