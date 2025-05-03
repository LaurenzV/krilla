/*!
An extension to krilla that allows rendering SVG files to a PDF file.

It is based on [usvg](https://github.com/linebender/resvg) and passes nearly the whole
resvg test suite. See the [examples]( https://github.com/LaurenzV/krilla/tree/main/crates/krilla-svg/examples)
directory for an example on how to use this crate in combination with krilla to convert SVG files
to PDF.
*/

#![deny(missing_docs)]

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::Arc;

use fontdb::Database;
use krilla::color::rgb;
use krilla::geom::{Point, Rect, Size, Transform};
use krilla::paint::FillRule;
use krilla::surface::Surface;
use krilla::text::Font;
use krilla::text::GlyphId;
use usvg::{fontdb, roxmltree, Group, ImageKind, Node, Tree};

use crate::util::RectExt;

mod clip_path;
mod filter;
mod group;
mod image;
mod mask;
mod path;
mod text;
mod util;

/// Settings that should be applied when converting a SVG.
#[derive(Copy, Clone, Debug)]
pub struct SvgSettings {
    /// Whether text should be embedded as properly selectable text. Otherwise,
    /// it will be drawn as outlined paths instead.
    pub embed_text: bool,
    /// How much filters, which will be converted to bitmaps, should be scaled. Higher values
    /// mean better quality, but also bigger file sizes.
    pub filter_scale: f32,
}

impl Default for SvgSettings {
    fn default() -> Self {
        Self {
            embed_text: true,
            filter_scale: 4.0,
        }
    }
}

/// An extension trait for the `Surface` struct that allows you to draw SVGs onto a surface.
pub trait SurfaceExt {
    /// Draw a `usvg` tree onto a surface with the given size and settings.
    fn draw_svg(&mut self, tree: &Tree, size: Size, svg_settings: SvgSettings) -> Option<()>;

    /// Draw a `usvg` tree onto a surface with the given size and settings, at a specified position.
    fn draw_svg_with_position(
        &mut self,
        tree: &Tree,
        size: Size,
        svg_settings: SvgSettings,
        position: Point,
    ) -> Option<()>;
}

impl SurfaceExt for Surface<'_> {
    fn draw_svg(&mut self, tree: &Tree, size: Size, svg_settings: SvgSettings) -> Option<()> {
        self.draw_svg_with_position(tree, size, svg_settings, Point::from_xy(0.0, 0.0))
    }

    fn draw_svg_with_position(
        &mut self,
        tree: &Tree,
        size: Size,
        svg_settings: SvgSettings,
        position: Point,
    ) -> Option<()> {
        let old_fill = self.get_fill().cloned();
        let old_stroke = self.get_stroke().cloned();    
        let sx = size.width() / tree.size().width();
        let sy = size.height() / tree.size().height();
        let tx = position.x;
        let ty = position.y;
        let transform = Transform::from_row(sx, 0.0, 0.0, sy, tx, ty);
        self.push_transform(&transform);
        self.push_clip_path(
            &Rect::from_xywh(0.0, 0.0, size.width(), size.height())
                .unwrap()
                .to_clip_path(),
            &FillRule::NonZero,
        );
        render_tree(tree, svg_settings, self);
        self.pop();
        self.pop();

        self.set_fill(old_fill);
        self.set_stroke(old_stroke);

        Some(())
    }
}

struct ProcessContext {
    fonts: HashMap<fontdb::ID, Font>,
    svg_settings: SvgSettings,
}

impl ProcessContext {
    fn new(fonts: HashMap<fontdb::ID, Font>, svg_settings: SvgSettings) -> Self {
        Self {
            fonts,
            svg_settings,
        }
    }
}

pub(crate) fn render_tree(tree: &Tree, svg_settings: SvgSettings, surface: &mut Surface) {
    let mut db = tree.fontdb().clone();
    let mut fc = get_context_from_group(Arc::make_mut(&mut db), svg_settings, tree.root());
    group::render(tree.root(), surface, &mut fc);
}

pub(crate) fn render_node(
    node: &Node,
    mut tree_fontdb: Arc<Database>,
    svg_settings: SvgSettings,
    surface: &mut Surface,
) {
    let mut fc = get_context_from_node(Arc::make_mut(&mut tree_fontdb), svg_settings, node);
    group::render_node(node, surface, &mut fc);
}

/// Render an SVG glyph from an OpenType font into a surface. You can plug this method into the
/// `render_svg_glyph_fn` field of `SerializeSettings` in krilla..
pub fn render_svg_glyph(
    data: &[u8],
    context_color: rgb::Color,
    glyph: GlyphId,
    surface: &mut Surface,
) -> Option<()> {
    let mut data = data;
    let settings = SvgSettings::default();

    let mut decoded = vec![];
    if data.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = flate2::read::GzDecoder::new(data);
        decoder.read_to_end(&mut decoded).ok()?;
        data = &decoded;
    }

    let xml = std::str::from_utf8(data).ok()?;
    let document = roxmltree::Document::parse(xml).ok()?;

    // Reparsing every time might be pretty slow in some cases, because Noto Color Emoji
    // for example contains hundreds of glyphs in the same SVG document, meaning that we have
    // to reparse it every time. However, Twitter Color Emoji does have each glyph in a
    // separate SVG document, and since we use COLRv1 for Noto Color Emoji anyway, this is
    // good enough.
    let opts = usvg::Options {
        style_sheet: Some(format!(
            "svg {{ color: rgb({}, {}, {}) }}",
            context_color.red(),
            context_color.green(),
            context_color.blue()
        )),
        ..Default::default()
    };
    let tree = Tree::from_xmltree(&document, &opts).ok()?;

    if let Some(node) = tree.node_by_id(&format!("glyph{}", glyph.to_u32())) {
        render_node(node, tree.fontdb().clone(), settings, surface)
    } else {
        // Twitter Color Emoji SVGs contain the glyph ID on the root element, which isn't saved by
        // usvg. So in this case, we simply draw the whole document.
        render_tree(&tree, settings, surface)
    };

    Some(())
}

fn get_context_from_group(
    tree_fontdb: &mut Database,
    svg_settings: SvgSettings,
    group: &Group,
) -> ProcessContext {
    let mut ids = HashSet::new();
    get_ids_from_group_impl(group, &mut ids);
    let ids = ids.into_iter().collect::<Vec<_>>();
    let db = convert_fontdb(tree_fontdb, Some(ids));

    ProcessContext::new(db, svg_settings)
}

fn get_context_from_node(
    tree_fontdb: &mut Database,
    svg_settings: SvgSettings,
    node: &Node,
) -> ProcessContext {
    let mut ids = HashSet::new();
    get_ids_impl(node, &mut ids);
    let ids = ids.into_iter().collect::<Vec<_>>();
    let db = convert_fontdb(tree_fontdb, Some(ids));

    ProcessContext::new(db, svg_settings)
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

fn convert_fontdb(db: &mut Database, ids: Option<Vec<fontdb::ID>>) -> HashMap<fontdb::ID, Font> {
    let mut map = HashMap::new();

    let ids = ids.unwrap_or(db.faces().map(|f| f.id).collect::<Vec<_>>());

    for id in ids {
        // What we could do is just go through each font and then create a new Font object for each of them.
        // However, this is somewhat wasteful and expensive, because we have to hash each font, which
        // can go be multiple MB. So instead, we first construct a font info object, which is much
        // cheaper, and then check whether we already have a corresponding font object in the cache.
        // If not, we still need to construct it.
        if let Some((font_data, index)) = unsafe { db.make_shared_face_data(id) } {
            if let Some(font) = Font::new(font_data.into(), index) {
                map.insert(id, font);
            }
        }
    }

    map
}
