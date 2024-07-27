use crate::canvas::Surface;
use crate::svg::clip_path::{get_clip_path, SvgClipPath};
use crate::svg::mask::get_mask;
use crate::svg::util::{convert_blend_mode, convert_transform};
use crate::svg::{filter, image, path};
use usvg::Node;

pub fn render(group: &usvg::Group, surface: &mut dyn Surface) {
    if !group.filters().is_empty() {
        filter::render(group, surface);
        return;
    }

    isolated(group, surface);
}

pub fn isolated(group: &usvg::Group, surface: &mut dyn Surface) {
    let isolated = if group.isolate() {
        &mut surface.isolated()
    } else {
        surface
    };

    transformed(group, isolated);
}

pub fn transformed(group: &usvg::Group, surface: &mut dyn Surface) {
    let mut transformed = surface.transformed(convert_transform(&group.transform()));
    clipped(group, &mut transformed);
}

pub fn clipped(group: &usvg::Group, surface: &mut dyn Surface) {
    let clipped: &mut dyn Surface = if let Some(clip_path) = group.clip_path() {
        let converted = get_clip_path(group, clip_path);
        match converted {
            SvgClipPath::SimpleClip(rules) => &mut surface.clipped_many(rules),
            SvgClipPath::ComplexClip(mask) => &mut surface.masked(mask),
        }
    } else {
        surface
    };

    blended_and_opacified(group, clipped);
}

pub fn blended_and_opacified(group: &usvg::Group, surface: &mut dyn Surface) {
    let masked = if let Some(mask) = group.mask() {
        &mut surface.masked(get_mask(mask))
    } else {
        surface
    };

    let mut blended = masked.blended(convert_blend_mode(&group.blend_mode()));
    let mut opacified = blended.opacified(group.opacity());

    for child in group.children() {
        render_node(child, &mut opacified);
    }
}

pub fn render_node(node: &Node, surface: &mut dyn Surface) {
    match node {
        Node::Group(g) => render(g, surface),
        Node::Path(p) => path::render(p, surface),
        Node::Image(i) => image::render(i, surface),
        Node::Text(t) => unimplemented!(),
    }
}
