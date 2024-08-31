//! A collection of annotations, allowing you to specify region-specific behavior, such as links.

use crate::error::KrillaResult;
use crate::object::action::Action;
use crate::object::destination::Destination;
use crate::serialize::{Object, SerializerContext};
use crate::util::RectExt;
use pdf_writer::types::AnnotationType;
use pdf_writer::{Chunk, Finish, Name, Ref};
use tiny_skia_path::{Rect, Transform};

/// A type of annotation.
pub enum Annotation {
    /// A link annotation.
    Link(LinkAnnotation),
}

impl Annotation {
    pub(crate) fn serialize_into(
        &self,
        sc: &mut SerializerContext,
        root_ref: Ref,
        page_size: f32,
    ) -> KrillaResult<Chunk> {
        match self {
            Annotation::Link(link) => link.serialize_into(sc, root_ref, page_size),
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
    pub rect: Rect,
    /// The target of the link annotation.
    pub target: Target,
}

impl Into<Annotation> for LinkAnnotation {
    fn into(self) -> Annotation {
        Annotation::Link(self)
    }
}

impl LinkAnnotation {
    fn serialize_into(
        &self,
        sc: &mut SerializerContext,
        root_ref: Ref,
        // TODO: Change how page size is handled here?
        page_size: f32,
    ) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let target_ref = sc.new_ref();

        match &self.target {
            Target::Destination(dest) => chunk.extend(&dest.serialize_into(sc, target_ref)?),
            Target::Action(_) => {}
        };

        let mut annotation = chunk
            .indirect(root_ref)
            .start::<pdf_writer::writers::Annotation>();

        let invert_transform = Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, page_size);
        let actual_rect = self.rect.transform(invert_transform).unwrap();
        annotation.subtype(AnnotationType::Link);
        annotation.rect(actual_rect.to_pdf_rect());
        annotation.border(0.0, 0.0, 0.0, None);

        match &self.target {
            Target::Destination(_) => {
                annotation.pair(Name(b"Dest"), target_ref);
            }
            Target::Action(action) => action.serialize_into(sc, annotation.action()),
        };

        annotation.finish();

        Ok(chunk)
    }
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::object::action::LinkAction;
    use crate::object::annotation::{LinkAnnotation, Target};
    use crate::object::destination::XyzDestination;

    use crate::tests::{default_size, rect_to_path};
    use crate::{Fill, Paint, rgb};
    use krilla_macros::snapshot;
    use tiny_skia_path::{Point, Rect, Size};
    use usvg::NormalizedF32;
    use crate::rgb::Rgb;
    use crate::serialize::SerializeSettings;
    use crate::surface::PageBuilder;

    #[snapshot(single_page)]
    fn annotation_to_link(page: &mut PageBuilder) {
        page.add_annotation(
            LinkAnnotation {
                rect: Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
                target: Target::Action(
                    LinkAction::new("https://www.youtube.com".to_string()).into(),
                ),
            }
                .into(),
        );
    }

    #[test]
    fn annotation_to_invalid_destination() {
        let mut db = Document::new(SerializeSettings::settings_1());
        let mut page = db.start_page(default_size());
        page.add_annotation(
            LinkAnnotation {
                rect: Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
                target: Target::Destination(
                    XyzDestination::new(1, Point::from_xy(100.0, 100.0)).into(),
                ),
            }
                .into(),
        );
        page.finish();
        assert!(db.finish().is_err())
    }

    #[snapshot(document)]
    fn annotation_to_destination(db: &mut Document) {
        let mut page = db.start_page(Size::from_wh(200.0, 200.0).unwrap());
        page.add_annotation(
            LinkAnnotation {
                rect: Rect::from_xywh(50.0, 0.0, 100.0, 100.0).unwrap(),
                target: Target::Destination(
                    XyzDestination::new(1, Point::from_xy(100.0, 100.0)).into(),
                ),
            }
            .into(),
        );

        let mut surface = page.surface();
        surface.fill_path(
            &rect_to_path(50.0, 0.0, 150.0, 100.0),
            Fill {
                paint: Paint::<Rgb>::Color(rgb::Color::new(255, 0, 0)),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            },
        );
        surface.finish();
        page.finish();

        let mut page = db.start_page(Size::from_wh(200.0, 200.0).unwrap());
        page.add_annotation(
            LinkAnnotation {
                rect: Rect::from_xywh(50.0, 100.0, 100.0, 100.0).unwrap(),
                target: Target::Destination(
                    XyzDestination::new(0, Point::from_xy(0.0, 0.0)).into(),
                ),
            }
            .into(),
        );
        let mut my_surface = page.surface();
        my_surface.fill_path(
            &rect_to_path(50.0, 100.0, 150.0, 200.0),
            Fill {
                paint: Paint::<Rgb>::Color(rgb::Color::new(0, 255, 0)),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            },
        );

        my_surface.finish();
        page.finish();
    }
}
