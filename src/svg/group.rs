use crate::surface::{CanvasBuilder, Surface};
use crate::svg::clip_path::{get_clip_path, SvgClipPath};
use crate::svg::mask::get_mask;
use crate::svg::util::{convert_blend_mode, convert_transform};
use crate::svg::{filter, image, path, text, FontContext};
use usvg::{Node, NormalizedF32};

pub fn render(
    group: &usvg::Group,
    canvas_builder: &mut CanvasBuilder,
    font_context: &mut FontContext,
) {
    if !group.filters().is_empty() {
        filter::render(group, canvas_builder);
        return;
    }

    if group.isolate() {
        canvas_builder.push_isolated();
    }

    canvas_builder.push_transform(&convert_transform(&group.transform()));

    let svg_clip = group
        .clip_path()
        .map(|c| get_clip_path(group, c, canvas_builder.sub_canvas(), font_context));

    if let Some(ref svg_clip) = svg_clip {
        match svg_clip {
            SvgClipPath::SimpleClip(rules) => {
                for rule in rules {
                    canvas_builder.push_clip_path(&rule.0, &rule.1);
                }
            }
            SvgClipPath::ComplexClip(mask) => canvas_builder.push_mask(mask.clone()),
        }
    }

    if let Some(mask) = group.mask() {
        let mask = get_mask(mask, canvas_builder.sub_canvas(), font_context);
        canvas_builder.push_mask(mask);
    }

    canvas_builder.push_blend_mode(convert_blend_mode(&group.blend_mode()));

    // TODO: OPtimize alpha = 1 case.
    if group.opacity() != NormalizedF32::ONE {
        canvas_builder.push_opacified(group.opacity());
    }

    for child in group.children() {
        render_node(child, canvas_builder, font_context);
    }

    if group.opacity() != NormalizedF32::ONE {
        canvas_builder.pop_opacified();
    }

    canvas_builder.pop_blend_mode();

    if group.mask().is_some() {
        canvas_builder.pop_mask();
    }

    if let Some(svg_clip) = svg_clip {
        match svg_clip {
            SvgClipPath::SimpleClip(rules) => {
                for _ in &rules {
                    canvas_builder.pop_clip_path();
                }
            }
            SvgClipPath::ComplexClip(_) => {
                canvas_builder.pop_mask();
            }
        }
    }

    canvas_builder.pop_transform();

    if group.isolate() {
        canvas_builder.pop_isolated();
    }
}

pub fn render_node(
    node: &Node,
    canvas_builder: &mut CanvasBuilder,
    font_context: &mut FontContext,
) {
    match node {
        Node::Group(g) => render(g, canvas_builder, font_context),
        Node::Path(p) => path::render(p, canvas_builder, font_context),
        Node::Image(i) => image::render(i, canvas_builder, font_context),
        Node::Text(t) => text::render(t, canvas_builder, font_context),
    }
}
