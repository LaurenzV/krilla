use tiny_skia_path::Transform;
use crate::canvas::Canvas;

mod path;
mod util;
mod group;

pub fn render(tree: &usvg::Tree) -> Canvas {
    let mut canvas = Canvas::new(tree.size());
    group::render(tree.root(), &Transform::identity(), &mut canvas);
    canvas
}