use krilla::destination::XyzDestination;
use krilla::geom::Point;
use krilla::outline::{Outline, OutlineNode};
use krilla::page::PageSettings;
use krilla::Document;
use krilla_macros::snapshot;

use crate::{blue_fill, green_fill, rect_to_path, red_fill};

#[snapshot(document)]
fn outline_simple(d: &mut Document) {
    let fills = [red_fill(1.0), green_fill(1.0), blue_fill(1.0)];
    for (index, fill) in fills.into_iter().enumerate() {
        let factor = index as f32 * 50.0;
        let path = rect_to_path(factor, factor, 100.0 + factor, 100.0 + factor);
        let mut page = d.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());
        let mut surface = page.surface();
        surface.set_fill(Some(fill));
        surface.draw_path(&path);
        surface.finish();
        page.finish();
    }
    let mut outline = Outline::new();

    let mut child1 = OutlineNode::new(
        "Heading 1".to_string(),
        XyzDestination::new(0, Point::from_xy(0.0, 0.0)),
    );
    child1.push_child(OutlineNode::new(
        "Heading 1.1".to_string(),
        XyzDestination::new(1, Point::from_xy(50.0, 50.0)),
    ));

    let child2 = OutlineNode::new(
        "Heading 2".to_string(),
        XyzDestination::new(2, Point::from_xy(100.0, 100.0)),
    );

    outline.push_child(child1);
    outline.push_child(child2);

    d.set_outline(outline);
}
#[snapshot(document)]
fn outline_open_state(d: &mut Document) {
    let fills = [red_fill(1.0), green_fill(1.0), blue_fill(1.0)];
    for (index, fill) in fills.into_iter().enumerate() {
        let factor = index as f32 * 50.0;
        let path = rect_to_path(factor, factor, 100.0 + factor, 100.0 + factor);
        let mut page = d.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());
        let mut surface = page.surface();
        surface.set_fill(Some(fill));
        surface.draw_path(&path);
        surface.finish();
        page.finish();
    }
    let mut outline = Outline::new();

    let mut child1 = OutlineNode::new(
        "Heading 1".to_string(),
        XyzDestination::new(0, Point::from_xy(0.0, 0.0)),
    );
    child1.set_open(true);
    child1.push_child(OutlineNode::new(
        "Heading 1.1".to_string(),
        XyzDestination::new(1, Point::from_xy(50.0, 50.0)),
    ));

    let child2 = OutlineNode::new(
        "Heading 2".to_string(),
        XyzDestination::new(2, Point::from_xy(100.0, 100.0)),
    );

    outline.push_child(child1);
    outline.push_child(child2);

    d.set_outline(outline);
}

#[snapshot(document)]
fn outline_mixed_state(d: &mut Document) {
    let fills = [red_fill(1.0), green_fill(1.0), blue_fill(1.0)];
    for (index, fill) in fills.into_iter().enumerate() {
        let factor = index as f32 * 50.0;
        let path = rect_to_path(factor, factor, 100.0 + factor, 100.0 + factor);
        let mut page = d.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());
        let mut surface = page.surface();
        surface.set_fill(Some(fill));
        surface.draw_path(&path);
        surface.finish();
        page.finish();
    }
    let mut outline = Outline::new();

    // Open parent containing an open child, so the root /Outlines must count
    // grand-children as visible too.
    let mut child1 = OutlineNode::new(
        "Heading 1".to_string(),
        XyzDestination::new(0, Point::from_xy(0.0, 0.0)),
    )
    .with_open(true);
    let mut nested = OutlineNode::new(
        "Heading 1.1".to_string(),
        XyzDestination::new(1, Point::from_xy(50.0, 50.0)),
    )
    .with_open(true);
    nested.push_child(OutlineNode::new(
        "Heading 1.1.1".to_string(),
        XyzDestination::new(1, Point::from_xy(60.0, 60.0)),
    ));
    child1.push_child(nested);

    // Closed parent; its child should contribute to the magnitude but not be
    // visible from the root's perspective.
    let mut child2 = OutlineNode::new(
        "Heading 2".to_string(),
        XyzDestination::new(2, Point::from_xy(100.0, 100.0)),
    );
    child2.push_child(OutlineNode::new(
        "Heading 2.1".to_string(),
        XyzDestination::new(2, Point::from_xy(120.0, 120.0)),
    ));

    outline.push_child(child1);
    outline.push_child(child2);

    d.set_outline(outline);
}

#[snapshot(document)]
fn outline_with_empty_title(d: &mut Document) {
    let mut outline = Outline::new();

    let child = OutlineNode::new(
        "".to_string(),
        XyzDestination::new(0, Point::from_xy(0.0, 0.0)),
    );

    outline.push_child(child);

    d.set_outline(outline);
}
