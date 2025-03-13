//! Text and font support.
//!
//! krilla has extensive support for OpenType fonts. It supports CFF-based as well
//! as glyf-based OpenType fonts. In addition to that, krilla also supports
//! all major tables used in color fonts, including the `SVG`, `COLR`, `sbix` and
//! `CBDT`/`EBDT` (only PNG) tables, something that, to the best of my knowledge, no other
//! Rust crates provides.
//!
//! Even better is the fact that you do not need to take care of choosing the right
//! table for drawing glyphs: All you need to do is to provide the [`Font`] object with
//! an appropriate index and variation coordinates.

use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use skrifa::instance::Location;
use skrifa::metrics::GlyphMetrics;
use skrifa::prelude::{LocationRef, Size};
use skrifa::raw::types::NameId;
use skrifa::raw::TableProvider;
use skrifa::{FontRef, MetadataProvider};
use tiny_skia_path::{FiniteF32, Rect};
use yoke::{Yoke, Yokeable};

use crate::object::font::PaintMode;
use crate::surface::Surface;
use crate::util::{Prehashed, RectWrapper};
use crate::{Data, Transform};

#[cfg(feature = "raster-images")]
pub(crate) mod bitmap;
pub(crate) mod colr;
pub(crate) mod outline;
pub(crate) mod svg;

/// An OpenType font. Can be a TrueType, OpenType font or a TrueType collection.
/// It holds a reference to the underlying data as well as some basic information
/// about the font.
///
/// Cloning and hashing this type is cheap. Creating it is a little expensive, so if
/// possible, the font should be cached.
///
/// While an object of this type is associated with an OTF font, it is only associated
/// with a specific instance, i.e. with specific variation coordinates and with a specific
/// index for TrueType collections. This means that if you want to use the same font with
/// different variation axes, you need to create separate instances of [`Font`].
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Font(Arc<Prehashed<Repr>>);

impl Font {
    /// Create a new font from some data.
    ///
    /// The `index` indicates the index that should be
    /// associated with this font for TrueType collections, otherwise this value should be
    /// set to 0. The location indicates the variation axes that should be associated with
    /// the font.
    ///
    /// The `allow_color` property allows you to specify whether krilla should render the font
    /// as a color font. When setting this property to false, krilla will always only use the
    /// `glyf`/`CFF` tables of the font. If you don't know what this means, just set it to `true`.
    ///
    /// Returns `None` if the index is invalid or the font couldn't be read.
    pub fn new(data: Data, index: u32, allow_color: bool) -> Option<Self> {
        let font_info = FontInfo::new(data.as_ref(), index, allow_color)?;

        Font::new_with_info(data.clone(), Arc::new(font_info))
    }

    #[doc(hidden)]
    pub fn new_with_info(data: Data, font_info: Arc<FontInfo>) -> Option<Self> {
        let font_ref_yoke =
            Yoke::<FontRefYoke<'static>, Arc<dyn AsRef<[u8]> + Send + Sync>>::attach_to_cart(
                data.0.clone(),
                |data| {
                    let font_ref = FontRef::from_index(data.as_ref(), 0).unwrap();
                    FontRefYoke {
                        font_ref: font_ref.clone(),
                        glyph_metrics: font_ref
                            .glyph_metrics(Size::unscaled(), LocationRef::default()),
                    }
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
    pub(crate) fn index(&self) -> u32 {
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

    pub(crate) fn allow_color(&self) -> bool {
        self.0.font_info.allow_color
    }

    pub(crate) fn weight(&self) -> f32 {
        self.0.font_info.weight.get()
    }

    pub(crate) fn stretch(&self) -> f32 {
        self.0.font_info.stretch.get()
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

    /// The units per em of the font.
    pub fn units_per_em(&self) -> f32 {
        self.0.font_info.units_per_em as f32
    }

    pub(crate) fn bbox(&self) -> Rect {
        self.0.font_info.global_bbox.0
    }

    // For now, location will always be default, until we support variable fonts.
    pub(crate) fn location_ref(&self) -> LocationRef {
        (&self.0.font_info.location).into()
    }

    /// Return the underlying `FontRef`.
    pub(crate) fn font_ref(&self) -> &FontRef {
        &self.0.font_ref_yoke.get().font_ref
    }

    pub(crate) fn glyph_metrics(&self) -> &GlyphMetrics {
        &self.0.font_ref_yoke.get().glyph_metrics
    }

    pub(crate) fn font_data(&self) -> Data {
        self.0.font_data.clone()
    }

    #[inline]
    pub(crate) fn advance_width(&self, glyph_id: GlyphId) -> Option<f32> {
        self.glyph_metrics().advance_width(glyph_id.to_skrifa())
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
// We unfortunately need to make this public so that `krilla-svg` can cache fonts.
#[derive(Debug, Hash, Eq, PartialEq)]
#[doc(hidden)]
pub struct FontInfo {
    index: u32,
    checksum: u32,
    location: Location,
    units_per_em: u16,
    global_bbox: RectWrapper,
    postscript_name: Option<String>,
    ascent: FiniteF32,
    allow_color: bool,
    descent: FiniteF32,
    cap_height: Option<FiniteF32>,
    is_monospaced: bool,
    italic_angle: FiniteF32,
    weight: FiniteF32,
    stretch: FiniteF32,
}

struct Repr {
    font_info: Arc<FontInfo>,
    font_data: Data,
    font_ref_yoke: Yoke<FontRefYoke<'static>, Arc<dyn AsRef<[u8]> + Send + Sync>>,
}

impl Hash for Repr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // We assume that if the font info is distinct, the font itself is distinct as well. This
        // doesn't have to be the case: while the font does have a checksum, it's "only" a
        // u32. The proper way would be to hash the whole font data, but this is just too expensive.
        // However, the odds of the checksum AND all font metrics (including font name) being the same
        // with the font being different is diminishingly low.
        self.font_info.hash(state);
    }
}

impl FontInfo {
    #[doc(hidden)]
    pub fn new(data: &[u8], index: u32, allow_color: bool) -> Option<Self> {
        let font_ref = FontRef::from_index(data, index).ok()?;
        let checksum = font_ref.head().ok()?.checksum_adjustment();

        let location = Location::default();
        let metrics = font_ref.metrics(Size::unscaled(), &location);
        let ascent = FiniteF32::new(metrics.ascent)?;
        let descent = FiniteF32::new(metrics.descent)?;
        let is_monospaced = metrics.is_monospace;
        let cap_height = metrics.cap_height.map(|n| FiniteF32::new(n).unwrap());
        let italic_angle = FiniteF32::new(metrics.italic_angle).unwrap();
        let weight = FiniteF32::new(font_ref.attributes().weight.value())?;
        let stretch = FiniteF32::new(font_ref.attributes().stretch.ratio())?;
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

                    None
                })
            } else {
                None
            }
        };

        Some(FontInfo {
            index,
            checksum,
            location,
            allow_color,
            units_per_em,
            postscript_name,
            ascent,
            cap_height,
            descent,
            is_monospaced,
            weight,
            stretch,
            italic_angle,
            global_bbox: RectWrapper(global_bbox),
        })
    }
}

/// A glyph ID.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct GlyphId(u32);

impl GlyphId {
    /// Create a new glyph ID.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the glyph ID as a u32.
    pub fn to_u32(&self) -> u32 {
        self.0
    }

    pub(crate) fn to_skrifa(self) -> skrifa::GlyphId {
        skrifa::GlyphId::new(self.0)
    }
}

/// A yoke so that we can attach a `FontRef` object to the corresponding `Font`,
/// without running into lifetime issues.
#[derive(Yokeable, Clone)]
struct FontRefYoke<'a> {
    pub font_ref: FontRef<'a>,
    pub glyph_metrics: GlyphMetrics<'a>,
}

/// Draw a color glyph to a surface.
pub(crate) fn draw_color_glyph(
    font: Font,
    glyph: GlyphId,
    paint_mode: PaintMode,
    base_transform: Transform,
    surface: &mut Surface,
) -> Option<()> {
    surface.push_transform(&base_transform);
    surface.push_transform(&Transform::from_scale(1.0, -1.0));

    let drawn = colr::draw_glyph(font.clone(), glyph, paint_mode, surface)
        .or_else(|| svg::draw_glyph(font.clone(), glyph, surface, paint_mode))
        .or_else(|| {
            #[cfg(feature = "raster-images")]
            let res = bitmap::draw_glyph(font.clone(), glyph, surface);

            #[cfg(not(feature = "raster-images"))]
            let res = None;

            res
        });

    surface.pop();
    surface.pop();

    drawn
}

/// Draw a color glyph or outline glyph to a surface.
pub(crate) fn draw_glyph(
    font: Font,
    glyph: GlyphId,
    paint_mode: PaintMode,
    base_transform: Transform,
    surface: &mut Surface,
) -> Option<()> {
    draw_color_glyph(font.clone(), glyph, paint_mode, base_transform, surface)
        .or_else(|| outline::draw_glyph(font, glyph, paint_mode, base_transform, surface))
}

/// A glyph with certain properties.
pub trait Glyph {
    /// The glyph ID of the glyph.
    fn glyph_id(&self) -> GlyphId;
    /// The range of bytes in the original text covered by the cluster that the glyph
    /// belongs to.
    fn text_range(&self) -> Range<usize>;
    /// The advance in the x direction of the glyph, at the given font size.
    fn x_advance(&self, size: f32) -> f32;
    /// The offset in the x direction of the glyph, at the given font size.
    fn x_offset(&self, size: f32) -> f32;
    /// The offset in the y direction of the glyph, at the given font size.
    fn y_offset(&self, size: f32) -> f32;
    /// The advance in the y direction of the glyph, at the given font size.
    fn y_advance(&self, size: f32) -> f32;
    /// A location identifying the glyph. If set, `krilla` will automatically call
    /// `set_location` before processing the glyph.
    fn location(&self) -> Option<crate::surface::Location>;
}

/// A glyph type that implements `Glyph`.
///
/// You can use it if you don't  have your own type of glyph that you want to use.
#[derive(Debug, Clone)]
pub struct KrillaGlyph {
    /// The glyph ID of the glyph.
    pub glyph_id: GlyphId,
    /// The range in the original text that corresponds to the
    /// cluster of the glyph.
    pub text_range: Range<usize>,
    /// The advance of the glyph.
    pub x_advance: f32,
    /// The x offset of the glyph.
    pub x_offset: f32,
    /// The y offset of the glyph.
    pub y_offset: f32,
    /// The y advance of the glyph.
    pub y_advance: f32,
    /// The location of the glyph.
    pub location: Option<crate::surface::Location>,
}

impl Glyph for KrillaGlyph {
    fn glyph_id(&self) -> GlyphId {
        self.glyph_id
    }

    fn text_range(&self) -> Range<usize> {
        self.text_range.clone()
    }

    fn x_advance(&self, size: f32) -> f32 {
        self.x_advance * size
    }

    fn x_offset(&self, size: f32) -> f32 {
        self.x_offset * size
    }

    fn y_offset(&self, size: f32) -> f32 {
        self.y_offset * size
    }

    fn y_advance(&self, size: f32) -> f32 {
        self.y_advance * size
    }

    fn location(&self) -> Option<crate::surface::Location> {
        self.location
    }
}

impl KrillaGlyph {
    /// Create a new Krilla glyph.
    /// 
    /// Important: `x_advance`, `x_offset`, `y_offset` and `y_advance`
    /// need to be normalized, i.e. divided by the units per em!
    pub fn new(
        glyph_id: GlyphId,
        x_advance: f32,
        x_offset: f32,
        y_offset: f32,
        y_advance: f32,
        range: Range<usize>,
        location: Option<crate::surface::Location>,
    ) -> Self {
        Self {
            glyph_id,
            x_advance,
            x_offset,
            y_offset,
            y_advance,
            text_range: range,
            location,
        }
    }
}
