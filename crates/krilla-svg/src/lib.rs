//! Drawing SVG files to a surface.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use fontdb::Database;
use krilla::font::Font;
use krilla::path::FillRule;
use krilla::surface::Surface;
use krilla::SvgSettings;
use tiny_skia_path::{Rect, Size, Transform};
use usvg::{fontdb, Group, ImageKind, Node, Tree};

use crate::util::RectExt;

mod clip_path;
mod filter;
mod group;
mod image;
mod mask;
mod path;
mod text;
mod util;

pub trait SurfaceExt {
    fn draw_svg(&mut self, tree: &usvg::Tree, size: Size, svg_settings: SvgSettings) -> Option<()>;
}

impl SurfaceExt for Surface<'_> {
    fn draw_svg(&mut self, tree: &Tree, size: Size, svg_settings: SvgSettings) -> Option<()> {
        let transform = Transform::from_scale(
            size.width() / tree.size().width(),
            size.height() / tree.size().height(),
        );
        self.push_transform(&transform);
        self.push_clip_path(
            &Rect::from_xywh(0.0, 0.0, tree.size().width(), tree.size().height())
                .unwrap()
                .to_clip_path(),
            &FillRule::NonZero,
        );
        render_tree(tree, svg_settings, self);
        self.pop();
        self.pop();

        Some(())
    }
}

/// A struct that stores some information that is needed globally when processing an SVG.
struct ProcessContext {
    /// A map from font IDs to `Font` objects.
    fonts: HashMap<fontdb::ID, Font>,
    /// A number of settings that can be used to configure the behavior for converting the SVG.
    svg_settings: SvgSettings,
}

impl ProcessContext {
    /// Create a new `ProcessContext`.
    fn new(fonts: HashMap<fontdb::ID, Font>, svg_settings: SvgSettings) -> Self {
        Self {
            fonts,
            svg_settings,
        }
    }
}

/// Render a usvg `Tree` into a surface.
///
/// Returns `None` if the conversion was not successful (for example if a fontdb ID is
/// referenced that doesn't exist in the database).
pub fn render_tree(tree: &Tree, svg_settings: SvgSettings, surface: &mut Surface) {
    let mut db = tree.fontdb().clone();
    let mut fc = get_context_from_group(Arc::make_mut(&mut db), svg_settings, tree.root(), surface);
    group::render(tree.root(), surface, &mut fc);
}

/// Render a usvg `Node` into a surface.
pub fn render_node(
    node: &Node,
    mut tree_fontdb: Arc<Database>,
    svg_settings: SvgSettings,
    surface: &mut Surface,
) {
    let mut fc =
        get_context_from_node(Arc::make_mut(&mut tree_fontdb), svg_settings, node, surface);
    group::render_node(node, surface, &mut fc);
}

/// Get the `PorcessContext` from a `Group`.
fn get_context_from_group(
    tree_fontdb: &mut Database,
    svg_settings: SvgSettings,
    group: &Group,
    surface: &mut Surface,
) -> ProcessContext {
    let mut ids = HashSet::new();
    get_ids_from_group_impl(group, &mut ids);
    let ids = ids.into_iter().collect::<Vec<_>>();

    ProcessContext::new(surface.convert_fontdb(tree_fontdb, Some(ids)), svg_settings)
}

/// Get the `PorcessContext` from a `Node`.
fn get_context_from_node(
    tree_fontdb: &mut Database,
    svg_settings: SvgSettings,
    node: &Node,
    surface: &mut Surface,
) -> ProcessContext {
    let mut ids = HashSet::new();
    get_ids_impl(node, &mut ids);
    let ids = ids.into_iter().collect::<Vec<_>>();

    ProcessContext::new(surface.convert_fontdb(tree_fontdb, Some(ids)), svg_settings)
}

fn get_ids_from_group_impl(group: &Group, ids: &mut HashSet<fontdb::ID>) {
    for child in group.children() {
        get_ids_impl(child, ids);
    }
}

// Collect all used font IDs
fn get_ids_impl(node: &Node, ids: &mut HashSet<fontdb::ID>) {
    match node {
        Node::Text(t) => {
            for span in t.layouted() {
                for g in &span.positioned_glyphs {
                    ids.insert(g.font);
                }
            }
        }
        Node::Group(group) => {
            get_ids_from_group_impl(group, ids);
        }
        Node::Image(image) => {
            if let ImageKind::SVG(svg) = image.kind() {
                get_ids_from_group_impl(svg.root(), ids);
            }
        }
        _ => {}
    }

    node.subroots(|subroot| get_ids_from_group_impl(subroot, ids));
}
