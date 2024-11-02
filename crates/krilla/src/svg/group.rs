use crate::surface::Surface;
use crate::svg::util::{convert_blend_mode, convert_transform};
use crate::svg::{clip_path, filter, image, mask, path, text, ProcessContext};
use usvg::Node;

pub fn render(group: &usvg::Group, surface: &mut Surface, process_context: &mut ProcessContext) {
    if !group.filters().is_empty() {
        filter::render(group, surface);
        return;
    }

    let mut pop_count = 0;

    if group.isolate() {
        surface.push_isolated();
        pop_count += 1
    }

    surface.push_transform(&convert_transform(&group.transform()));
    pop_count += 1;

    if let Some(clip_path) = group.clip_path() {
        pop_count += clip_path::render(group, clip_path, surface, process_context);
    }

    if let Some(mask) = group.mask() {
        pop_count += mask::render(mask, surface, process_context);
    }

    surface.push_blend_mode(convert_blend_mode(&group.blend_mode()));
    surface.push_opacity(group.opacity());
    pop_count += 2;

    for child in group.children() {
        render_node(child, surface, process_context);
    }

    for _ in 0..pop_count {
        surface.pop();
    }
}

pub fn render_node(node: &Node, surface: &mut Surface, process_context: &mut ProcessContext) {
    match node {
        Node::Group(g) => render(g, surface, process_context),
        Node::Path(p) => path::render(p, surface, process_context),
        Node::Image(i) => {
            image::render(i, surface, process_context);
        }
        Node::Text(t) => text::render(t, surface, process_context),
    }
}
