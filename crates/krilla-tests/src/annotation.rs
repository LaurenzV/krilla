use krilla::annotation::LinkBorder;
use krilla::destination::XyzDestination;
use krilla::geom::{Point, Quadrilateral, Rect};
use krilla::page::{Page, PageSettings};
use krilla::Document;
use krilla_macros::{snapshot, visreg};

use crate::{green_fill, load_pdf, rect_to_path, red_fill};
use crate::{settings_1, LinkAction};
use crate::{LinkAnnotation, Target};

#[snapshot]
fn annotation_to_link(page: &mut Page) {
    page.add_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
            Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
        )
        .into(),
    );
}

#[snapshot]
fn annotation_with_border(page: &mut Page) {
    use krilla::color::{rgb, Color};
    page.add_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(50.0, 50.0, 100.0, 100.0).unwrap(),
            Target::Action(LinkAction::new("https://www.youtube.com".to_string()).into()),
        )
        .with_border(LinkBorder::new(1.0, Color::Rgb(rgb::Color::new(255, 0, 0))))
        .into(),
    );
}

#[snapshot]
fn annotation_with_quad_points(page: &mut Page) {
    let mut surface = page.surface();
    let path1 = rect_to_path(0.0, 0.0, 50.0, 50.0);
    let path2 = rect_to_path(50.0, 50.0, 100.0, 100.0);
    surface.set_fill(Some(green_fill(1.0)));
    surface.draw_path(&path1);
    surface.draw_path(&path2);
    surface.finish();

    page.add_annotation(
        LinkAnnotation::new_with_quad_points(
            vec![
                Quadrilateral([
                    Point::from_xy(0.0, 50.0),
                    Point::from_xy(50.0, 50.0),
                    Point::from_xy(50.0, 0.0),
                    Point::from_xy(0.0, 0.0),
                ]),
                Rect::from_xywh(50.0, 50.0, 50.0, 50.0).unwrap().into(),
            ],
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
            Target::Destination(XyzDestination::new(1, Point::from_xy(100.0, 100.0)).into()),
        )
        .into(),
    );

    let mut surface = page.surface();
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(50.0, 0.0, 150.0, 100.0));
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
    my_surface.set_fill(Some(green_fill(1.0)));
    my_surface.draw_path(&rect_to_path(50.0, 100.0, 150.0, 200.0));

    my_surface.finish();
    page.finish();
}

#[snapshot(document)]
fn annotation_to_embedded_pdf_page(document: &mut Document) {
    let pdf = load_pdf("page_media_box_bottom_right.pdf");
    document.embed_pdf_pages(&pdf, &[0]);

    let mut page = document.start_page_with(PageSettings::new(200.0, 200.0));
    page.add_annotation(
        LinkAnnotation::new(
            Rect::from_xywh(0.0, 0.0, 200.0, 200.0).unwrap(),
            Target::Destination(XyzDestination::new(0, Point::from_xy(100.0, 100.0)).into()),
        )
        .into(),
    );

    let mut surface = page.surface();
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(50.0, 0.0, 150.0, 100.0));
    surface.finish();
    page.finish();
}
