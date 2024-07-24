use crate::canvas::Surface;
use crate::svg::path;
use crate::svg::util::convert_blend_mode;
use usvg::Node;

pub fn render<T>(group: &usvg::Group, transform: &usvg::Transform, surface: &mut T)
where
    T: Surface,
{
    if !group.filters().is_empty() {
        unimplemented!();
    }

    if group.isolate() {
        unimplemented!();
    }

    if group.mask().is_some() {
        unimplemented!();
    }

    if group.clip_path().is_some() {
        unimplemented!();
    }

    let transform = transform.pre_concat(group.transform());
    let mut blended = surface.blended(convert_blend_mode(&group.blend_mode()));
    let mut opacified = blended.opacified(group.opacity());

    for child in group.children() {
        match child {
            Node::Group(g) => render(g, &transform, &mut opacified),
            Node::Path(p) => path::render(p, &transform, &mut opacified),
            Node::Image(i) => unimplemented!(),
            Node::Text(t) => unimplemented!(),
        }
    }

    opacified.finish();
    blended.finish();
}
