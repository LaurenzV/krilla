use krilla_macros::snapshot2;
use tiny_skia_path::{Point, Rect};

use krilla::annotation::{LinkAnnotation, Target};
use krilla::destination::{NamedDestination, XyzDestination};
use crate::{blue_fill, green_fill, rect_to_path, red_fill};
use crate::Document;

#[snapshot2(document)]
fn named_destination_basic(d: &mut Document) {
    let dest1 = NamedDestination::new(
        "hi".to_string(),
        XyzDestination::new(0, Point::from_xy(100.0, 100.0)),
    );
    let dest2 = NamedDestination::new(
        "by".to_string(),
        XyzDestination::new(1, Point::from_xy(0.0, 0.0)),
    );

    let mut page = d.start_page();
    page.add_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap(),
            None,
            Target::Destination(dest1.clone().into()),
        )
            .into(),
    );
    page.add_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(100.0, 100.0, 100.0, 100.0).unwrap(),
            None,
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
            None,
            Target::Destination(dest1.clone().into()),
        )
            .into(),
    );
    let mut surface = page.surface();
    surface.fill_path(&rect_to_path(0.0, 0.0, 100.0, 100.0), blue_fill(1.0));
    surface.finish();
    page.finish();
}