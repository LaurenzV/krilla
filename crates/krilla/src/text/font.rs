use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use skrifa::instance::{Location, LocationRef, Size};
use skrifa::metrics::GlyphMetrics;
use skrifa::raw::types::NameId;
use skrifa::raw::TableProvider;
use skrifa::{FontRef, MetadataProvider};
use tiny_skia_path::FiniteF32;
use yoke::{Yoke, Yokeable};

use crate::geom::Rect;
use crate::text::GlyphId;
use crate::util::Prehashed;
use crate::Data;

/// An OpenType font. Can be a TrueType, OpenType font or a TrueType collection.
/// It holds a reference to the underlying data as well as some basic information
/// about the font.
///
/// Cloning and hashing this type is cheap.
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

    pub(crate) fn new_with_info(data: Data, font_info: Arc<FontInfo>) -> Option<Self> {
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
        self.0.font_info.global_bbox
    }

    // For now, location will always be default, until we support variable fonts.
    pub(crate) fn location_ref(&self) -> LocationRef {
        (&self.0.font_info.location).into()
    }

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
#[derive(Debug, Hash, Eq, PartialEq)]
pub(crate) struct FontInfo {
    index: u32,
    checksum: u32,
    data_len: usize,
    location: Location,
    units_per_em: u16,
    global_bbox: Rect,
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
    pub(crate) fn new(data: &[u8], index: u32, allow_color: bool) -> Option<Self> {
        let font_ref = FontRef::from_index(data, index).ok()?;
        let data_len = data.len();
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
            data_len,
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
            global_bbox,
        })
    }
}

/// A yoke so that we can attach a `FontRef` object to the corresponding `Font`,
/// without running into lifetime issues.
#[derive(Yokeable, Clone)]
struct FontRefYoke<'a> {
    pub font_ref: FontRef<'a>,
    pub glyph_metrics: GlyphMetrics<'a>,
}
