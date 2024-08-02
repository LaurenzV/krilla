use crate::font::Font;
use crate::stream::StreamBuilder;
use skrifa::instance::Location;
use skrifa::GlyphId;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
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

#[derive(Hash)]
struct SvgFont {
    pub font: Font,
    pub glyph_sets: BTreeMap<GlyphId, String>,
}

struct FontContext {
    fonts: HashMap<fontdb::ID, SvgFont>,
}

impl FontContext {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
        }
    }
}

pub fn render_tree(tree: &usvg::Tree, stream_builder: &mut StreamBuilder) {
    let mut fc = FontContext::new();
    get_context_from_group(tree.fontdb().clone(), tree.root(), &mut fc);
    group::render(tree.root(), stream_builder, &mut fc);
}

pub fn render_node(node: &Node, fontdb: Arc<fontdb::Database>, stream_builder: &mut StreamBuilder) {
    let mut fc = FontContext::new();
    get_context_from_node(fontdb, node, &mut fc);
    group::render_node(node, stream_builder, &mut fc);
}

fn get_context_from_group(
    fontdb: Arc<fontdb::Database>,
    group: &Group,
    render_context: &mut FontContext,
) {
    for child in group.children() {
        get_context_from_node(fontdb.clone(), child, render_context);
    }
}

fn get_context_from_node(
    fontdb: Arc<fontdb::Database>,
    node: &Node,
    render_context: &mut FontContext,
) {
    match node {
        Node::Text(t) => {
            for span in t.layouted() {
                for g in &span.positioned_glyphs {
                    let font = render_context.fonts.entry(g.font).or_insert_with(|| {
                        fontdb
                            .with_face_data(g.font, |data, _| {
                                // TODO: Avoid vector allocation somehow?
                                let font =
                                    Font::new(Rc::new(data.to_vec()), Location::default()).unwrap();
                                SvgFont {
                                    font,
                                    glyph_sets: BTreeMap::new(),
                                }
                            })
                            .unwrap()
                    });

                    font.glyph_sets
                        .insert(GlyphId::new(g.id.0 as u32), g.text.clone());
                }
            }
        }
        Node::Group(group) => {
            get_context_from_group(fontdb.clone(), group, render_context);
        }
        Node::Image(image) => {
            if let ImageKind::SVG(svg) = image.kind() {
                get_context_from_group(fontdb.clone(), svg.root(), render_context);
            }
        }
        _ => {}
    }

    node.subroots(|subroot| get_context_from_group(fontdb.clone(), subroot, render_context));
}

// #[cfg(test)]
// mod tests {
//     use crate::canvas::Page;
//     use crate::serialize::PageSerialize;
//     use crate::svg::render_tree;
//     use std::sync::Arc;
//     use usvg::fontdb;
//
//     #[test]
//     pub fn svg() {
//         let data = std::fs::read("/Users/lstampfl/Programming/GitHub/svg2pdf/test.svg").unwrap();
//         let mut db = fontdb::Database::new();
//         db.load_system_fonts();
//
//         let tree = usvg::Tree::from_data(
//             &data,
//             &usvg::Options {
//                 fontdb: Arc::new(db),
//                 ..Default::default()
//             },
//         )
//         .unwrap();
//
//         let mut page = Page::new(tree.size());
//         let mut stream_builder = page.builder();
//         render_tree(&tree, &mut stream_builder);
//         let stream = stream_builder.finish();
//         let serializer_context = page.finish();
//         let finished = stream.serialize(serializer_context, tree.size()).finish();
//         let _ = std::fs::write("out/svg.pdf", &finished);
//         let _ = std::fs::write("out/svg.txt", &finished);
//     }
// }
