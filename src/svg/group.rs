use crate::surface::Surface;
use crate::svg::clip_path::{get_clip_path, SvgClipPath};
use crate::svg::mask::get_mask;
use crate::svg::util::{convert_blend_mode, convert_transform};
use crate::svg::{filter, image, path, text, ProcessContext};
use usvg::{Node, NormalizedF32};

pub fn render(
    group: &usvg::Group,
    surface: &mut Surface,
    process_context: &mut ProcessContext,
) {
    if !group.filters().is_empty() {
        filter::render(group, surface, process_context);
        return;
    }

    if group.isolate() {
        surface.push_isolated();
    }

    surface.push_transform(&convert_transform(&group.transform()));

    let svg_clip = group
        .clip_path()
        .map(|c| get_clip_path(group, c, surface.stream_surface(), process_context));

    if let Some(svg_clip) = svg_clip.clone() {
        match svg_clip {
            SvgClipPath::SimpleClip(rules) => {
                for rule in rules {
                    surface.push_clip_path(&rule.0, &rule.1);
                }
            }
            SvgClipPath::ComplexClip(mask) => surface.push_mask(mask),
        }
    }

    if let Some(mask) = group.mask() {
        let mask = get_mask(mask, surface.stream_surface(), process_context);
        surface.push_mask(mask);
    }

    surface.push_blend_mode(convert_blend_mode(&group.blend_mode()));

    // TODO: OPtimize alpha = 1 case.
    if group.opacity() != NormalizedF32::ONE {
        surface.push_opacified(group.opacity());
    }

    for child in group.children() {
        render_node(child, surface, process_context);
    }

    if group.opacity() != NormalizedF32::ONE {
        surface.pop();
    }

    surface.pop();

    if group.mask().is_some() {
        surface.pop();
    }

    // TODO: Remove clone
    if let Some(svg_clip) = svg_clip {
        match svg_clip {
            SvgClipPath::SimpleClip(rules) => {
                for _ in &rules {
                    surface.pop();
                }
            }
            SvgClipPath::ComplexClip(_) => {
                surface.pop();
            }
        }
    }

    surface.pop();

    if group.isolate() {
        surface.pop();
    }
}

pub fn render_node(
    node: &Node,
    surface: &mut Surface,
    process_context: &mut ProcessContext,
) {
    match node {
        Node::Group(g) => render(g, surface, process_context),
        Node::Path(p) => path::render(p, surface, process_context),
        Node::Image(i) => image::render(i, surface, process_context),
        Node::Text(t) => text::render(t, surface, process_context),
    }
}
