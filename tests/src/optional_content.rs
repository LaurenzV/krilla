use krilla::page::Page;
use krilla::Document;
use krilla_macros::snapshot;

use crate::{blue_fill, green_fill, rect_to_path, red_fill};

#[snapshot]
fn marked_content_bare_tag(page: &mut Page) {
    let mut surface = page.surface();
    surface.start_marked_content("myTag");
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(20.0, 20.0, 80.0, 80.0));
    surface.end_marked_content();
    surface.finish();
}

#[snapshot]
fn marked_content_nested_in_tagged_section(page: &mut Page) {
    use krilla::tagging::ContentTag;

    let mut surface = page.surface();
    surface.start_tagged(ContentTag::Other);
    surface.start_marked_content("inner");
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(20.0, 20.0, 80.0, 80.0));
    surface.end_marked_content();
    surface.end_tagged();
    surface.finish();
}

#[snapshot(document)]
fn optional_content_single_group(d: &mut Document) {
    let layer = d.add_optional_content_group("My Layer", true);

    let mut page = d.start_page();
    let mut surface = page.surface();
    surface.start_optional_content(layer);
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(20.0, 20.0, 80.0, 80.0));
    surface.end_optional_content();
    surface.finish();
    page.finish();
}

#[snapshot(document)]
fn optional_content_hidden_by_default(d: &mut Document) {
    let layer = d.add_optional_content_group("Hidden Layer", false);

    let mut page = d.start_page();
    let mut surface = page.surface();
    surface.start_optional_content(layer);
    surface.set_fill(Some(green_fill(1.0)));
    surface.draw_path(&rect_to_path(20.0, 20.0, 80.0, 80.0));
    surface.end_optional_content();
    surface.finish();
    page.finish();
}

#[snapshot(document)]
fn optional_content_two_groups(d: &mut Document) {
    let visible = d.add_optional_content_group("Visible Layer", true);
    let hidden = d.add_optional_content_group("Hidden Layer", false);

    let mut page = d.start_page();
    let mut surface = page.surface();
    surface.start_optional_content(visible);
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(20.0, 20.0, 80.0, 80.0));
    surface.end_optional_content();
    surface.start_optional_content(hidden);
    surface.set_fill(Some(blue_fill(1.0)));
    surface.draw_path(&rect_to_path(100.0, 100.0, 160.0, 160.0));
    surface.end_optional_content();
    surface.finish();
    page.finish();
}

#[snapshot(document)]
fn optional_content_nested_with_marked_content(d: &mut Document) {
    let layer = d.add_optional_content_group("Outer Layer", true);

    let mut page = d.start_page();
    let mut surface = page.surface();
    surface.start_optional_content(layer);
    surface.start_marked_content("inner");
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(20.0, 20.0, 80.0, 80.0));
    surface.end_marked_content();
    surface.end_optional_content();
    surface.finish();
    page.finish();
}

/// Registering a group from the surface, mid-page, is equivalent to registering it on the document
/// before the page was started -- the case a producer that only learns which groups it needs while
/// drawing cannot express through `Document::add_optional_content_group` at all, because the page holds
/// the document's borrow for as long as it is open.
#[snapshot(document)]
fn optional_content_group_added_from_surface(d: &mut Document) {
    let mut page = d.start_page();
    let mut surface = page.surface();
    let layer = surface.add_optional_content_group("Drawn Layer", true);
    surface.start_optional_content(layer);
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(20.0, 20.0, 80.0, 80.0));
    surface.end_optional_content();
    surface.finish();
    page.finish();
}

/// Two groups minted mid-page, one hidden, interleaved with a bare marked-content tag -- the shape a
/// layer-aware producer actually emits.
#[snapshot(document)]
fn optional_content_groups_added_from_surface_while_drawing(d: &mut Document) {
    let mut page = d.start_page();
    let mut surface = page.surface();

    let shown = surface.add_optional_content_group("Shown", true);
    surface.start_optional_content(shown);
    surface.start_marked_content("1");
    surface.set_fill(Some(red_fill(1.0)));
    surface.draw_path(&rect_to_path(20.0, 20.0, 80.0, 80.0));
    surface.end_marked_content();
    surface.end_optional_content();

    let hidden = surface.add_optional_content_group("Hidden", false);
    surface.start_optional_content(hidden);
    surface.start_marked_content("2");
    surface.set_fill(Some(blue_fill(1.0)));
    surface.draw_path(&rect_to_path(100.0, 100.0, 160.0, 160.0));
    surface.end_marked_content();
    surface.end_optional_content();

    surface.finish();
    page.finish();
}
