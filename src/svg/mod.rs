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

#[cfg(test)]
mod tests {
    use crate::serialize::{PageSerialize, SerializeSettings};
    use crate::svg::render;

    #[test]
    pub fn svg() {
        let data = std::fs::read("/Users/lstampfl/Programming/GitHub/svg2pdf/tests/svg/custom/paint-servers/linearGradient/linear-gradient-14.svg").unwrap();
        let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();
        let canvas = render(&tree);
        let finished = canvas.serialize(SerializeSettings::default()).finish();
        let _ = std::fs::write("out/svg.pdf", &finished);
        let _ = std::fs::write("out/svg.txt", &finished);
    }
}