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
