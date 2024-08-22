use crate::object::destination::XyzDestination;
use crate::serialize::{Object, SerializerContext};
use crate::util::RectExt;
use pdf_writer::types::AnnotationType;
use pdf_writer::{Chunk, Finish, Name, Ref};
use tiny_skia_path::{Rect, Transform};

pub trait Annotation {
    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref, page_size: f32) -> Chunk;
}

#[derive(Clone)]
pub enum LinkTarget {
    Destination(XyzDestination),
}

#[derive(Clone)]
pub struct LinkAnnotation {
    pub rect: Rect,
    pub link_target: LinkTarget,
}

impl Annotation for LinkAnnotation {
    fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref, page_size: f32) -> Chunk {
        let mut chunk = Chunk::new();

        let target_ref = sc.new_ref();

        match &self.link_target {
            LinkTarget::Destination(dest) => chunk.extend(&dest.serialize_into(sc, target_ref)),
        };

        let mut annotation = chunk
            .indirect(root_ref)
            .start::<pdf_writer::writers::Annotation>();

        let invert_transform = Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, page_size);
        let actual_rect = self.rect.transform(invert_transform).unwrap();
        annotation.subtype(AnnotationType::Link);
        annotation.rect(actual_rect.to_pdf_rect());
        annotation.border(0.0, 0.0, 0.0, None);

        match &self.link_target {
            LinkTarget::Destination(_) => annotation.pair(Name(b"Dest"), target_ref),
        };

        annotation.finish();

        chunk
    }
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::object::annotation::{LinkAnnotation, LinkTarget};
    use crate::object::destination::XyzDestination;
    use crate::rgb::Rgb;
    use crate::serialize::SerializeSettings;
    use crate::test_utils::{check_snapshot, rect_path};
    use crate::Fill;
    use tiny_skia_path::{Point, Rect, Size};

    #[test]
    fn simple() {
        let mut db = Document::new(SerializeSettings::default_test());
        let mut page = db.start_page(Size::from_wh(200.0, 200.0).unwrap());
        page.add_annotation(LinkAnnotation {
            rect: Rect::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap(),
            link_target: LinkTarget::Destination(XyzDestination::new(
                1,
                Point::from_xy(100.0, 100.0),
            )),
        });

        let mut surface = page.surface();
        surface.fill_path(&rect_path(0.0, 0.0, 100.0, 100.0), Fill::<Rgb>::default());
        surface.finish();
        page.finish();

        let mut page = db.start_page(Size::from_wh(200.0, 200.0).unwrap());
        page.add_annotation(LinkAnnotation {
            rect: Rect::from_xywh(100.0, 100.0, 100.0, 100.0).unwrap(),
            link_target: LinkTarget::Destination(XyzDestination::new(0, Point::from_xy(0.0, 0.0))),
        });
        let mut my_surface = page.surface();
        my_surface.fill_path(
            &rect_path(100.0, 100.0, 200.0, 200.0),
            Fill::<Rgb>::default(),
        );
        my_surface.finish();
        page.finish();

        check_snapshot("annotation/simple", &db.finish());
    }
}
