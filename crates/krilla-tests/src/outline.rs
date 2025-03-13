use krilla::destination::XyzDestination;
use krilla::document::{Document, PageSettings};
use krilla::outline::{Outline, OutlineNode};
use krilla::Point;
use krilla_macros::snapshot;

use crate::{blue_fill, green_fill, rect_to_path, red_fill};

#[snapshot(document)]
fn outline_simple(d: &mut Document) {
    let fills = [red_fill(1.0), green_fill(1.0), blue_fill(1.0)];
    for (index, fill) in fills.into_iter().enumerate() {
        let factor = index as f32 * 50.0;
        let path = rect_to_path(factor, factor, 100.0 + factor, 100.0 + factor);
        let mut page = d.start_page_with(PageSettings::new(200.0, 200.0));
        let mut surface = page.surface();
        surface.set_fill(fill);
        surface.fill_path(&path);
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
