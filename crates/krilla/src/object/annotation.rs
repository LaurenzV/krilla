//! PDF annotations, allowing you to add extra "content" to specific pages.
//!
//! PDF has the concept of annotations, which allow you to associate certain regions of
//! a page with an "annotation". The PDF reference defines many different actions, however,
//! krilla does not and never will expose all of them. As of right now, the only annotations
//! that are supported are "link annotations", which allow you associate a certain region of
//! the page with a link.

use pdf_writer::types::AnnotationFlags;
use pdf_writer::{Chunk, Finish, Name, Ref, TextStr};
use tiny_skia_path::Rect;

use crate::configure::{PdfVersion, ValidationError};
use crate::error::KrillaResult;
use crate::object::action::Action;
use crate::object::destination::Destination;
use crate::page::page_root_transform;
use crate::serialize::SerializeContext;
use crate::util::RectExt;
use crate::Point;

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
    pub(crate) quad_points: Option<Vec<Point>>,
    pub(crate) target: Target,
}

impl LinkAnnotation {
    /// Create a new link annotation.
    ///
    /// `rect`: The bounding box of the link annotation that it should cover on the page.
    /// `target`: The target of the link annotation.
    /// `quad_points`: An array of 4xn points, where each 4 points define the quadrilateral
    /// where the link annotation should be activated. This is useful if you for example have
    /// a link annotation that is broken to one or multiple lines. Note that the points
    /// have to be within the bounds defined by `rect`!
    pub fn new(rect: Rect, quad_points: Option<Vec<Point>>, target: Target) -> Self {
        Self {
            rect,
            quad_points,
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
                annotation.quad_points(p.iter().flat_map(|p| {
                    let mut p = p.to_tsp();
                    page_root_transform(page_height).map_point(&mut p);
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
