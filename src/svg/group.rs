use crate::surface::Surface;
use crate::svg::clip_path::{get_clip_path, SvgClipPath};
use crate::svg::util::{convert_blend_mode, convert_transform};
use crate::svg::{filter, image, mask, path, text, ProcessContext};
use usvg::Node;

pub fn render(group: &usvg::Group, surface: &mut Surface, process_context: &mut ProcessContext) {
    if !group.filters().is_empty() {
        filter::render(group, surface, process_context);
        return;
    }

    let mut pop_count = 0;

    if group.isolate() {
        surface.push_isolated();
        pop_count += 1
    }

    surface.push_transform(&convert_transform(&group.transform()));
    pop_count += 1;

    let svg_clip = group
        .clip_path()
        .map(|c| get_clip_path(group, c, surface.stream_surface(), process_context));

    if let Some(svg_clip) = svg_clip {
        match svg_clip {
            SvgClipPath::SimpleClip(rules) => {
                for rule in rules {
                    surface.push_clip_path(&rule.0, &rule.1);
                    pop_count += 1;
                }
            }
            SvgClipPath::ComplexClip(mask) => {
                surface.push_mask(mask);
                pop_count += 1;
            }
        }
    }

    if let Some(mask) = group.mask() {
        let mask = mask::render(mask, surface.stream_surface(), process_context);
        surface.push_mask(mask);
        pop_count += 1;
    }

    surface.push_blend_mode(convert_blend_mode(&group.blend_mode()));
    surface.push_opacified(group.opacity());
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
        Node::Image(i) => image::render(i, surface, process_context),
        Node::Text(t) => text::render(t, surface, process_context),
    }
}
