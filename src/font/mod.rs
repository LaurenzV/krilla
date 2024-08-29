use crate::serialize::SvgSettings;
use crate::surface::Surface;
use crate::type3_font::Type3ID;
use crate::util::{Prehashed, RectWrapper};
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

pub mod bitmap;
pub mod colr;
pub mod outline;
pub mod svg;

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

#[derive(Debug)]
struct LocationWrapper(Location);

impl Hash for LocationWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.coords().hash(state);
    }
}

impl PartialEq for LocationWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.coords().eq(other.0.coords())
    }
}

impl Eq for LocationWrapper {}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct FontInfo {
    index: u32,
    checksum: u32,
    location: LocationWrapper,
    pub(crate) units_per_em: u16,
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

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Font(Arc<Prehashed<Repr>>);

impl Debug for Font {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl FontInfo {
    pub fn new(data: &[u8], index: u32, location: Location) -> Option<Self> {
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

#[derive(Yokeable, Clone)]
struct FontRefWrapper<'a> {
    pub font_ref: FontRef<'a>,
}

impl Font {
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

    pub fn postscript_name(&self) -> Option<&str> {
        self.0.font_info.postscript_name.as_deref()
    }

    pub fn index(&self) -> u32 {
        self.font_info().index
    }

    pub fn font_info(&self) -> Arc<FontInfo> {
        self.0.font_info.clone()
    }

    pub fn cap_height(&self) -> Option<f32> {
        self.0.font_info.cap_height.map(|n| n.get())
    }

    pub fn ascent(&self) -> f32 {
        self.0.font_info.ascent.get()
    }

    pub fn weight(&self) -> f32 {
        self.0.font_info.weight.get()
    }

    pub fn descent(&self) -> f32 {
        self.0.font_info.descent.get()
    }

    pub fn is_monospaced(&self) -> bool {
        self.0.font_info.is_monospaced
    }

    pub fn italic_angle(&self) -> f32 {
        self.0.font_info.italic_angle.get()
    }

    pub fn units_per_em(&self) -> f32 {
        self.0.font_info.units_per_em as f32
    }

    pub fn bbox(&self) -> Rect {
        self.0.font_info.global_bbox.0
    }

    pub fn location_ref(&self) -> LocationRef {
        (&self.0.font_info.location.0).into()
    }

    pub fn font_ref(&self) -> &FontRef {
        &self.0.font_ref_yoke.get().font_ref
    }

    pub fn font_data(&self) -> Arc<dyn AsRef<[u8]> + Send + Sync> {
        self.0.font_data.clone()
    }

    pub fn advance_width(&self, glyph_id: GlyphId) -> Option<f32> {
        self.font_ref()
            .glyph_metrics(Size::unscaled(), self.location_ref())
            .advance_width(glyph_id)
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum GlyphType {
    Colr,
    Svg,
    Outline,
    Bitmap,
}

pub fn draw_glyph(
    font: Font,
    svg_settings: SvgSettings,
    glyph: GlyphId,
    surface: &mut Surface,
) -> Option<GlyphType> {
    let mut glyph_type = None;

    surface.push_transform(&Transform::from_scale(1.0, -1.0));

    if let Some(()) = colr::draw_glyph(font.clone(), glyph, surface) {
        glyph_type = Some(GlyphType::Colr);
    } else if let Some(()) = svg::draw_glyph(font.clone(), svg_settings, glyph, surface) {
        glyph_type = Some(GlyphType::Svg);
    } else if let Some(()) = bitmap::draw_glyph(font.clone(), glyph, surface) {
        glyph_type = Some(GlyphType::Bitmap);
    } else if let Some(()) = outline::draw_glyph(font.clone(), glyph, surface) {
        glyph_type = Some(GlyphType::Outline);
    }

    surface.pop();

    glyph_type
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct CIDIdentifer(pub Font);
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Type3Identifier(pub Font, pub Type3ID);

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum FontIdentifier {
    Cid(CIDIdentifer),
    Type3(Type3Identifier),
}

impl FontIdentifier {
    pub fn font(&self) -> Font {
        match self {
            FontIdentifier::Cid(cid) => cid.0.clone(),
            FontIdentifier::Type3(t3) => t3.0.clone(),
        }
    }
}
