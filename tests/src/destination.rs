use krilla::annotation::{LinkAnnotation, Target};
use krilla::destination::{NamedDestination, XyzDestination};
use krilla::geom::{Point, Rect};
use krilla_macros::snapshot;

use crate::{blue_fill, green_fill, rect_to_path, red_fill};
use crate::{cmyk_fill, Document};

#[snapshot(document)]
fn destination_named(d: &mut Document) {
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
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 100.0, 100.0));
    surface.set_fill(Some(green_fill(1.0)));
    surface.draw_path(&rect_to_path(100.0, 100.0, 200.0, 200.0));
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
    surface.set_fill(Some(blue_fill(1.0)));
    surface.draw_path(&rect_to_path(0.0, 0.0, 100.0, 100.0));
    surface.finish();
    page.finish();
}

// See <https://github.com/typst/typst/issues/7248>. We forgot that the entries
// need to be sorted by name. Not doing that can cause issues in Preview and
// Chrome.
#[snapshot(document)]
fn destination_named_sorting(d: &mut Document) {
    let mut page = d.start_page();

    for name in ["aaa", "bb", "a", "zzz", "y", "aa", "x", "ab"] {
        let dest = NamedDestination::new(
            name.to_string(),
            XyzDestination::new(0, Point::from_xy(0.0, 0.0)),
        );

        page.add_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(0.0, 0.0, 100.0, 100.0).unwrap(),
                Target::Destination(dest.into()),
            )
            .into(),
        );
    }
}
