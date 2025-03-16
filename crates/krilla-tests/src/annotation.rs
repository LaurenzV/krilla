use krilla::document::{Document, PageSettings};
use krilla::interactive::destination::XyzDestination;
use krilla::page::Page;
use krilla::{Point, Rect};
use krilla_macros::snapshot;

use crate::{green_fill, rect_to_path, red_fill};
use crate::{settings_1, LinkAction};
use crate::{LinkAnnotation, Target};

#[snapshot]
fn annotation_to_link(page: &mut Page) {
    page.add_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
            None,
            Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
        )
        .into(),
    );
}

#[snapshot]
fn annotation_with_quad_points(page: &mut Page) {
    let mut surface = page.surface();
    let path1 = rect_to_path(0.0, 0.0, 50.0, 50.0);
    let path2 = rect_to_path(50.0, 50.0, 100.0, 100.0);
    surface.set_fill(green_fill(1.0));
    surface.fill_path(&path1);
    surface.fill_path(&path2);
    surface.finish();

    page.add_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap(),
            Some(vec![
                Point::from_xy(0.0, 0.0),
                Point::from_xy(50.0, 0.0),
                Point::from_xy(50.0, 50.0),
                Point::from_xy(0.0, 50.0),
                Point::from_xy(50.0, 50.0),
                Point::from_xy(100.0, 50.0),
                Point::from_xy(100.0, 100.0),
                Point::from_xy(50.0, 100.0),
            ]),
            Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
        )
        .into(),
    );
}

#[test]
#[should_panic]
fn annotation_to_invalid_destination() {
    let mut d = Document::new_with(settings_1());
    let mut page = d.start_page_with(PageSettings::new(200.0, 200.0));
    page.add_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
            None,
            Target::Destination(XyzDestination::new(1, Point::from_xy(100.0, 100.0)).into()),
        )
        .into(),
    );
    page.finish();

    let _ = d.finish();
}

#[snapshot(document)]
fn annotation_to_destination(d: &mut Document) {
    let mut page = d.start_page_with(PageSettings::new(200.0, 200.0));
    page.add_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 0.0, 100.0, 100.0).unwrap(),
            None,
            Target::Destination(XyzDestination::new(1, Point::from_xy(100.0, 100.0)).into()),
        )
        .into(),
    );

    let mut surface = page.surface();
    surface.set_fill(red_fill(1.0));
    surface.fill_path(&rect_to_path(50.0, 0.0, 150.0, 100.0));
    surface.finish();
    page.finish();

    let mut page = d.start_page_with(PageSettings::new(200.0, 200.0));
    page.add_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 100.0, 100.0, 100.0).unwrap(),
            None,
            Target::Destination(XyzDestination::new(0, Point::from_xy(0.0, 0.0)).into()),
        )
        .into(),
    );
    let mut my_surface = page.surface();
    my_surface.set_fill(green_fill(1.0));
    my_surface.fill_path(&rect_to_path(50.0, 100.0, 150.0, 200.0));

    my_surface.finish();
    page.finish();
}
