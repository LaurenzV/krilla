//! Creating and using OpenType fonts.
//!
//! krilla has extensive support for OpenType fonts. It supports CFF-based as well
//! as glyf-based font OpenType fonts. In addition to that, krilla also supports
//! all of the major tables used in color fonts, including the `SVG`, `COLR`, `sbix`,
//! `CBDT`/`EBDT` tables, something that, to the best of my knowledge, no other
//! Rust crates provides.
//!
//! Even better is the fact that you do not need to take care of choosing the right
//! table for drawing glyphs: All you need to do is to provide the `Font` object with
//! an appropriate index and variation coordinates,
//!
//! krilla, in principle, also supports variable fonts. However, at the moment, variable
//! fonts are not encoded in the most efficient way (they are stored as Type3 fonts instead
//! of embedded TTF/CFF fonts, due to the lack of an instancing crate in the Rust ecosystem),
//! so if possible you should prefer static versions of font. But in principle, using fonts
//! with non-default variation coordinates should work, too.

use crate::error::KrillaResult;
use crate::serialize::SvgSettings;
use crate::surface::Surface;
use crate::type3_font::Type3ID;
use crate::util::{LocationWrapper, Prehashed, RectWrapper};
use skrifa::instance::Location;
use skrifa::outline::OutlinePen;
use skrifa::prelude::{LocationRef, Size};
use skrifa::raw::types::NameId;
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, Path, PathBuilder, Rect, Transform};
use yoke::{Yoke, Yokeable};
use std::ops::Range;

pub(crate) mod bitmap;
pub(crate) mod colr;
pub(crate) mod outline;
pub(crate) mod svg;

// TODO: Test TrueType collection

/// An OpenType font. Can be a TrueType, OpenType fonts or TrueType collections.
/// It holds a reference to the underlying data as well as some basic information
/// about the font.
///
/// Cloning and hashing this type is cheap. Creating it is a little expensive, so if
/// possible, the font should be cached.
///
/// While an object of this type is associated with an OTF font, it is only associated
/// with a specific instance, i.e. with specific variation coordinates and with a specific
/// index for TrueType collections. This means that if you want to use the same font with
/// different variation axes, you need to create separate instances.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Font(Arc<Prehashed<Repr>>);

impl Font {
    /// Create a new font from some data. The `index` indicates the index that should be
    /// associated with this font for TrueType collections, otherwise this value should be
    /// set to 0. The location indicates the variation axes that should be associated with
    /// the font.
    pub fn new(
        data: Arc<dyn AsRef<[u8]> + Send + Sync>,
        index: u32,
        location: Location,
    ) -> Option<Self> {
        let font_info = FontInfo::new(data.as_ref().as_ref(), index, location)?;

        Font::new_with_info(data, Arc::new(font_info))
    }

    pub(crate) fn new_with_info(
        data: Arc<dyn AsRef<[u8]> + Send + Sync>,
        font_info: Arc<FontInfo>,
    ) -> Option<Self> {
        let font_ref_yoke =
            Yoke::<FontRefWrapper<'static>, Arc<dyn AsRef<[u8]> + Send + Sync>>::attach_to_cart(
                data.clone(),
                |data| FontRefWrapper {
                    font_ref: FontRef::from_index(data.as_ref().as_ref(), 0).unwrap(),
                },
            );

        Some(Font(Arc::new(Prehashed::new(Repr {
            font_data: data,
            font_ref_yoke,
            font_info,
        }))))
    }

    pub(crate) fn postscript_name(&self) -> Option<&str> {
        self.0.font_info.postscript_name.as_deref()
    }

    /// Return the index of the font.
    pub fn index(&self) -> u32 {
        self.font_info().index
    }

    pub(crate) fn font_info(&self) -> Arc<FontInfo> {
        self.0.font_info.clone()
    }

    pub(crate) fn cap_height(&self) -> Option<f32> {
        self.0.font_info.cap_height.map(|n| n.get())
    }

    pub(crate) fn ascent(&self) -> f32 {
        self.0.font_info.ascent.get()
    }

    pub(crate) fn weight(&self) -> f32 {
        self.0.font_info.weight.get()
    }

    pub(crate) fn descent(&self) -> f32 {
        self.0.font_info.descent.get()
    }

    pub(crate) fn is_monospaced(&self) -> bool {
        self.0.font_info.is_monospaced
    }

    pub(crate) fn italic_angle(&self) -> f32 {
        self.0.font_info.italic_angle.get()
    }

    pub(crate) fn units_per_em(&self) -> f32 {
        self.0.font_info.units_per_em as f32
    }

    pub(crate) fn bbox(&self) -> Rect {
        self.0.font_info.global_bbox.0
    }

    /// Return the `LocationRef` of the font.
    pub fn location_ref(&self) -> LocationRef {
        (&self.0.font_info.location.0).into()
    }

    /// Return the `FontRef` of the font.
    pub fn font_ref(&self) -> &FontRef {
        &self.0.font_ref_yoke.get().font_ref
    }

    /// Return the underlying data of the font.
    pub fn font_data(&self) -> Arc<dyn AsRef<[u8]> + Send + Sync> {
        self.0.font_data.clone()
    }

    pub(crate) fn advance_width(&self, glyph_id: GlyphId) -> Option<f32> {
        self.font_ref()
            .glyph_metrics(Size::unscaled(), self.location_ref())
            .advance_width(glyph_id)
    }
}

impl Debug for Font {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Font {{..}}")
    }
}

/// `FontInfo` holds basic information about the font which is necessary
/// to distinguish a `Font` object from others. The `Hash` implementation
/// of the `Font` struct solely depends on its `FontInfo` object. The reason
/// we do this is to avoid hashing the whole font, which can be dozens of megabytes.
/// Instead, we parse the most basic information as well as additional distinguishing
/// information, such as the font name and the checksum, and has this instead.
/// This is much faster, and since we also include the checksum, the odds of two
/// different fonts ending up with the same hash is pretty much zero.
#[derive(Debug, Hash, Eq, PartialEq)]
pub(crate) struct FontInfo {
    index: u32,
    checksum: u32,
    location: LocationWrapper,
    units_per_em: u16,
    global_bbox: RectWrapper,
    postscript_name: Option<String>,
    ascent: FiniteF32,
    descent: FiniteF32,
    cap_height: Option<FiniteF32>,
    is_monospaced: bool,
    italic_angle: FiniteF32,
    weight: FiniteF32,
}

struct Repr {
    font_info: Arc<FontInfo>,
    font_data: Arc<dyn AsRef<[u8]> + Send + Sync>,
    font_ref_yoke: Yoke<FontRefWrapper<'static>, Arc<dyn AsRef<[u8]> + Send + Sync>>,
}

impl Hash for Repr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // We assume that if the font info is distinct, the font itself is distinct as well. This
        // strictly doesn't have to be the case, while the font does have a checksum, it's "only" a
        // u32. The proper way would be to hash the whole font data, but this is just too expensive.
        // However, the odds of the checksum AND all font metrics (including font name) being the same
        // with the font being different is diminishingly low.
        self.font_info.hash(state);
    }
}

impl FontInfo {
    pub(crate) fn new(data: &[u8], index: u32, location: Location) -> Option<Self> {
        let font_ref = FontRef::from_index(data, index).ok()?;
        let checksum = font_ref.head().ok()?.checksum_adjustment();

        let metrics = font_ref.metrics(Size::unscaled(), &location);
        let ascent = FiniteF32::new(metrics.ascent).unwrap();
        let descent = FiniteF32::new(metrics.descent).unwrap();
        let is_monospaced = metrics.is_monospace;
        let cap_height = metrics.cap_height.map(|n| FiniteF32::new(n).unwrap());
        let italic_angle = FiniteF32::new(metrics.italic_angle).unwrap();
        let weight = FiniteF32::new(font_ref.attributes().weight.value()).unwrap();
        let units_per_em = metrics.units_per_em;
        let global_bbox = metrics
            .bounds
            .and_then(|b| Rect::from_xywh(b.x_min, b.y_min, b.x_max, b.y_max))
            .unwrap_or(Rect::from_xywh(
                0.0,
                0.0,
                units_per_em as f32,
                units_per_em as f32,
            )?);

        let postscript_name = {
            if let Ok(name) = font_ref.name() {
                name.name_record().iter().find_map(|n| {
                    if n.name_id.get() == NameId::POSTSCRIPT_NAME {
                        if let Ok(string) = n.string(name.string_data()) {
                            return Some(string.to_string());
                        }
                    }

                    return None;
                })
            } else {
                return None;
            }
        };

        Some(FontInfo {
            index,
            checksum,
            location: LocationWrapper(location),
            units_per_em,
            postscript_name,
            ascent,
            cap_height,
            descent,
            is_monospaced,
            weight,
            italic_angle,
            global_bbox: RectWrapper(global_bbox),
        })
    }
}

/// A yoke so that we can attach a `FontRef` object to the corresponding `Font`,
/// without running into lifetime issues.
#[derive(Yokeable, Clone)]
struct FontRefWrapper<'a> {
    pub font_ref: FontRef<'a>,
}

/// The table from which a drawn glyph stems from.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub(crate) enum GlyphSource {
    Colr,
    Svg,
    Outline,
    Bitmap,
}

pub(crate) fn draw_glyph(
    font: Font,
    svg_settings: SvgSettings,
    glyph: GlyphId,
    surface: &mut Surface,
) -> KrillaResult<Option<GlyphSource>> {
    let mut glyph_source = None;

    surface.push_transform(&Transform::from_scale(1.0, -1.0));

    if let Some(()) = colr::draw_glyph(font.clone(), glyph, surface)? {
        glyph_source = Some(GlyphSource::Colr);
    } else if let Some(()) = svg::draw_glyph(font.clone(), glyph, surface, svg_settings)? {
        glyph_source = Some(GlyphSource::Svg);
    } else if let Some(()) = bitmap::draw_glyph(font.clone(), glyph, surface)? {
        glyph_source = Some(GlyphSource::Bitmap);
    } else if let Some(()) = outline::draw_glyph(font.clone(), glyph, surface)? {
        glyph_source = Some(GlyphSource::Outline);
    }

    surface.pop();

    Ok(glyph_source)
}

/// A unique CID identifier.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CIDIdentifer(pub Font);

/// A unique Type3 font identifier. Type3 fonts can only hold 256 glyphs, which
/// means that we might have to create more than one Type3 font. This is why we
/// additionally store an index that indicates which specific Type3Font we are
/// referring to.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct Type3Identifier(pub Font, pub Type3ID);

/// A font identifier for a PDF font.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) enum FontIdentifier {
    Cid(CIDIdentifer),
    Type3(Type3Identifier),
}

/// A wrapper struct for implementing the `OutlinePen` trait.
struct OutlineBuilder(PathBuilder);

impl OutlineBuilder {
    pub fn new() -> Self {
        Self(PathBuilder::new())
    }

    pub fn finish(self) -> Option<Path> {
        self.0.finish()
    }
}

impl OutlinePen for OutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.quad_to(cx0, cy0, x, y);
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }

    fn close(&mut self) {
        self.0.close()
    }
}

/// A single glyph.
///
/// *Note*: The units of `x_advance`, `x_offset` and `y_offset`
/// are in user space units!
#[derive(Debug, Clone)]
pub struct Glyph {
    pub glyph_id: GlyphId,
    pub range: Range<usize>,
    pub x_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub size: f32,
}

impl Glyph {
    /// Create a new glyph.
    pub fn new(
        glyph_id: GlyphId,
        x_advance: f32,
        x_offset: f32,
        y_offset: f32,
        range: Range<usize>,
        size: f32,
    ) -> Self {
        Self {
            glyph_id,
            x_advance,
            x_offset,
            y_offset,
            range,
            size,
        }
    }
}
