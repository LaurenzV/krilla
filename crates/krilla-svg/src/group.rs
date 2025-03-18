//! Group conversion

use krilla::surface::Surface;
use krilla::num::NormalizedF32;
use usvg::Node;

use crate::util::{convert_blend_mode, UsvgTransformExt};
use crate::{clip_path, filter, image, mask, path, text, ProcessContext};

pub(crate) fn render(
    group: &usvg::Group,
    surface: &mut Surface,
    process_context: &mut ProcessContext,
) {
    if !group.filters().is_empty() {
        filter::render(group, surface, process_context);
        return;
    }

    let mut pop_count = 0;

    if group.isolate() {
        surface.push_isolated();
        pop_count += 1
    }

    surface.push_transform(&group.transform().to_krilla());
    pop_count += 1;

    if let Some(clip_path) = group.clip_path() {
        pop_count += clip_path::render(group, clip_path, surface, process_context);
    }

    if let Some(mask) = group.mask() {
        pop_count += mask::render(mask, surface, process_context);
    }

    surface.push_blend_mode(convert_blend_mode(&group.blend_mode()));
    surface.push_opacity(NormalizedF32::new(group.opacity().get()).unwrap());
    pop_count += 2;

    for child in group.children() {
        render_node(child, surface, process_context);
    }

    for _ in 0..pop_count {
        surface.pop();
    }
}

pub(crate) fn render_node(
    node: &Node,
    surface: &mut Surface,
    process_context: &mut ProcessContext,
) {
    match node {
        Node::Group(g) => render(g, surface, process_context),
        Node::Path(p) => path::render(p, surface, process_context),
        Node::Image(i) => {
            image::render(i, surface, process_context);
        }
        Node::Text(t) => text::render(t, surface, process_context),
    }
}
