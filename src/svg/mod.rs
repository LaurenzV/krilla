use crate::stream::StreamBuilder;
use usvg::Node;

mod clip_path;
mod filter;
mod group;
mod image;
mod mask;
mod path;
// mod text;
mod util;

pub fn render_tree(tree: &usvg::Tree, stream_builder: &mut StreamBuilder) {
    group::render(tree.root(), stream_builder);
}

pub fn render_node(node: &Node, stream_builder: &mut StreamBuilder) {
    group::render_node(node, stream_builder);
}

#[cfg(test)]
mod tests {
    use crate::canvas::Page;
    use crate::serialize::{PageSerialize, SerializeSettings, SerializerContext};
    use crate::svg::render_tree;

    #[test]
    pub fn svg() {
        let data = std::fs::read("/Users/lstampfl/Programming/GitHub/svg2pdf/tests/svg/resvg/paint-servers/stop-opacity/simple-case.svg").unwrap();
        let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();
        let mut page = Page::new(tree.size());
        let mut stream_builder = page.builder();
        render_tree(&tree, &mut stream_builder);
        let stream = stream_builder.finish();
        let finished = stream
            .serialize(SerializeSettings::default(), tree.size())
            .finish();
        let _ = std::fs::write("out/svg.pdf", &finished);
        let _ = std::fs::write("out/svg.txt", &finished);
    }
}
