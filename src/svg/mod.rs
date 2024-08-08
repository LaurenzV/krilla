use crate::font::FontInfo;
use crate::surface::Surface;
use fontdb::Database;
use skrifa::instance::LocationRef;
use skrifa::FontRef;
use std::collections::HashMap;
use std::sync::Arc;
use usvg::{fontdb, Group, ImageKind, Node};

mod clip_path;
mod filter;
mod group;
mod image;
mod mask;
mod path;
mod text;
mod util;

struct FontContext<'a> {
    fonts: HashMap<fontdb::ID, (fontdb::ID, u16)>,
    fontdb: &'a mut Database,
}

impl<'a> FontContext<'a> {
    pub fn new(fontdb: &'a mut Database) -> Self {
        Self {
            fonts: HashMap::new(),
            fontdb,
        }
    }
}

pub fn render_tree(tree: &usvg::Tree, canvas_builder: &mut Surface, fontdb: &mut Database) {
    let mut fc = get_context_from_group(tree.fontdb().clone(), tree.root(), fontdb);
    group::render(tree.root(), canvas_builder, &mut fc);
}

pub fn render_node(
    node: &Node,
    tree_fontdb: Arc<fontdb::Database>,
    canvas_builder: &mut Surface,
    fontdb: &mut Database,
) {
    let mut fc = get_context_from_node(tree_fontdb, node, fontdb);
    group::render_node(node, canvas_builder, &mut fc);
}

fn get_context_from_group<'a>(
    tree_fontdb: Arc<Database>,
    group: &Group,
    fontdb: &'a mut Database,
) -> FontContext<'a> {
    let mut font_context = FontContext::new(fontdb);
    get_context_from_group_impl(tree_fontdb, group, &mut font_context);
    font_context
}

fn get_context_from_node<'a>(
    tree_fontdb: Arc<Database>,
    node: &Node,
    fontdb: &'a mut Database,
) -> FontContext<'a> {
    let mut font_context = FontContext::new(fontdb);
    get_context_from_node_impl(tree_fontdb, node, &mut font_context);
    font_context
}

fn get_context_from_group_impl(
    tree_fontdb: Arc<fontdb::Database>,
    group: &Group,
    render_context: &mut FontContext,
) {
    for child in group.children() {
        get_context_from_node_impl(tree_fontdb.clone(), child, render_context);
    }
}

fn get_context_from_node_impl(
    tree_fontdb: Arc<fontdb::Database>,
    node: &Node,
    render_context: &mut FontContext,
) {
    match node {
        Node::Text(t) => {
            for span in t.layouted() {
                for g in &span.positioned_glyphs {
                    render_context.fonts.entry(g.font).or_insert_with(|| {
                        let (source, index) = tree_fontdb.face_source(g.font).unwrap();

                        // TODO: Deduplicate fonts
                        let upem = tree_fontdb
                            .with_face_data(g.font, |data, index| {
                                FontInfo::new(
                                    FontRef::from_index(data, index).unwrap(),
                                    index,
                                    LocationRef::default(),
                                )
                                .unwrap()
                                .units_per_em
                            })
                            .unwrap();

                        let ids = render_context.fontdb.load_font_source(source);
                        (ids[index as usize], upem)
                    });
                }
            }
        }
        Node::Group(group) => {
            get_context_from_group_impl(tree_fontdb.clone(), group, render_context);
        }
        Node::Image(image) => {
            if let ImageKind::SVG(svg) = image.kind() {
                get_context_from_group_impl(tree_fontdb.clone(), svg.root(), render_context);
            }
        }
        _ => {}
    }

    node.subroots(|subroot| {
        get_context_from_group_impl(tree_fontdb.clone(), subroot, render_context)
    });
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::serialize::{PageSerialize, SerializeSettings};
    use crate::svg::render_tree;
    use std::sync::Arc;
    use usvg::fontdb;

    // #[test]
    // pub fn svg() {
    //     let data = std::fs::read("/Users/lstampfl/Programming/GitHub/svg2pdf/test.svg").unwrap();
    //     let mut db = fontdb::Database::new();
    //     db.load_system_fonts();
    //
    //     let tree = usvg::Tree::from_data(
    //         &data,
    //         &usvg::Options {
    //             fontdb: Arc::new(db.clone()),
    //             ..Default::default()
    //         },
    //     )
    //     .unwrap();
    //
    //     let mut document_builder = Document::new(SerializeSettings::default());
    //     let mut stream_builder = document_builder.start_page(tree.size());
    //     render_tree(&tree, &mut stream_builder, &mut db);
    //     stream_builder.finish();
    //     let finished = document_builder.finish(&db);
    //     let _ = std::fs::write("out/svg.pdf", &finished);
    //     let _ = std::fs::write("out/svg.txt", &finished);
    // }
}
