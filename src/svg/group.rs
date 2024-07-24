use crate::canvas::Surface;
use crate::svg::clip_path::{get_clip_path, SvgClipPath};
use crate::svg::path;
use crate::svg::util::{convert_blend_mode, convert_transform};
use usvg::Node;

pub fn render(group: &usvg::Group, surface: &mut dyn Surface) {
    if !group.filters().is_empty() {
        unimplemented!();
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
    let clipped = if let Some(clip_path) = group.clip_path() {
        let converted = get_clip_path(group, clip_path);
        match converted {
            SvgClipPath::SimpleClip(path, rule) => &mut surface.clipped(path, rule),
        }
    } else {
        surface
    };

    blended_and_opacified(group, clipped);
}

pub fn blended_and_opacified(group: &usvg::Group, surface: &mut dyn Surface) {
    if group.mask().is_some() {
        unimplemented!();
    }

    let mut blended = surface.blended(convert_blend_mode(&group.blend_mode()));
    let mut opacified = blended.opacified(group.opacity());

    for child in group.children() {
        match child {
            Node::Group(g) => render(g, &mut opacified),
            Node::Path(p) => path::render(p, &mut opacified),
            Node::Image(i) => unimplemented!(),
            Node::Text(t) => unimplemented!(),
        }
    }
}
