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
use tiny_skia_path::FiniteF32;
use yoke::{Yoke, Yokeable};

use crate::graphics::paint::{Fill, Stroke};
use crate::surface::Surface;
use crate::text::cid::CIDFont;
use crate::text::type3::{CoveredGlyph, Type3Font, Type3FontMapper, Type3ID};
use crate::{Data, Rect, Transform};

#[cfg(feature = "raster-images")]
pub(crate) mod bitmap;
pub(crate) mod cid;
pub(crate) mod colr;
pub(crate) mod font;
pub(crate) mod group;
pub(crate) mod outline;
#[cfg(feature = "simple-text")]
pub(crate) mod shape;
pub(crate) mod svg;
pub(crate) mod type3;

pub use font::*;
#[cfg(feature = "simple-text")]
pub use shape::TextDirection;

pub(crate) const PDF_UNITS_PER_EM: f32 = 1000.0;

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

impl PaintMode<'_> {
    pub(crate) fn to_owned(self) -> OwnedPaintMode {
        match self {
            PaintMode::Fill(f) => OwnedPaintMode::Fill((*f).clone()),
            PaintMode::Stroke(s) => OwnedPaintMode::Stroke((*s).clone()),
        }
    }
}

/// A wrapper enum for fills/strokes. We use that to keep track whether a Type3 font contains
/// filled or stroked outlines of a glyph.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PaintMode<'a> {
    Fill(&'a Fill),
    Stroke(&'a Stroke),
}

/// A unique CID identifier.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct CIDIdentifier(pub Font);

/// A unique Type3 font identifier. Type3 fonts can only hold 256 glyphs, which
/// means that we might have to create more than one Type3 font. This is why we
/// additionally store an index that indicates which specific Type3Font we are
/// referring to.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct Type3Identifier(pub Font, pub Type3ID);

/// A font identifier for a PDF font.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) enum FontIdentifier {
    Cid(CIDIdentifier),
    Type3(Type3Identifier),
}

/// The owned version of `PaintMode`.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) enum OwnedPaintMode {
    Fill(Fill),
    Stroke(Stroke),
}

impl From<Fill> for OwnedPaintMode {
    fn from(value: Fill) -> Self {
        Self::Fill(value)
    }
}

impl From<Stroke> for OwnedPaintMode {
    fn from(value: Stroke) -> Self {
        Self::Stroke(value)
    }
}

impl OwnedPaintMode {
    pub(crate) fn as_ref(&self) -> PaintMode {
        match self {
            OwnedPaintMode::Fill(f) => PaintMode::Fill(f),
            OwnedPaintMode::Stroke(s) => PaintMode::Stroke(s),
        }
    }
}

/// A glyph that belongs either to a CID font or a Type 3 font.
#[derive(Clone, Copy)]
pub(crate) enum PDFGlyph {
    Type3(u8),
    Cid(u16),
}

impl PDFGlyph {
    /// Encode the glyph into a content stream.
    pub(crate) fn encode_into(&self, slice: &mut Vec<u8>) {
        match self {
            PDFGlyph::Type3(cg) => slice.push(*cg),
            PDFGlyph::Cid(cid) => {
                slice.push((cid >> 8) as u8);
                slice.push((cid & 0xff) as u8);
            }
        }
    }
}

/// A container that holds all PDF fonts belonging to an OTF font.
#[derive(Debug)]
pub(crate) enum FontContainer {
    Type3(Type3FontMapper),
    CIDFont(CIDFont),
}

impl FontContainer {
    #[inline]
    pub(crate) fn font_identifier(&self, glyph: CoveredGlyph) -> Option<FontIdentifier> {
        match self {
            FontContainer::Type3(t3) => t3.id_from_glyph(&glyph.to_owned()),
            FontContainer::CIDFont(cid) => cid.get_cid(glyph.glyph_id).map(|_| cid.identifier()),
        }
    }

    #[inline]
    pub(crate) fn get_from_identifier_mut(
        &mut self,
        font_identifier: FontIdentifier,
    ) -> Option<&mut dyn PdfFont> {
        match self {
            FontContainer::Type3(t3) => {
                if let Some(t3_font) = t3.font_mut_from_id(font_identifier) {
                    Some(t3_font)
                } else {
                    None
                }
            }
            FontContainer::CIDFont(cid) => {
                if cid.identifier() == font_identifier {
                    Some(cid)
                } else {
                    None
                }
            }
        }
    }

    #[inline]
    pub(crate) fn get_from_identifier(
        &self,
        font_identifier: FontIdentifier,
    ) -> Option<&dyn PdfFont> {
        match self {
            FontContainer::Type3(t3) => {
                if let Some(t3_font) = t3.font_from_id(font_identifier) {
                    Some(t3_font)
                } else {
                    None
                }
            }
            FontContainer::CIDFont(cid) => {
                if cid.identifier() == font_identifier {
                    Some(cid)
                } else {
                    None
                }
            }
        }
    }

    #[inline]
    pub(crate) fn add_glyph(&mut self, glyph: CoveredGlyph) -> (FontIdentifier, PDFGlyph) {
        match self {
            FontContainer::Type3(t3) => {
                let (identifier, gid) = t3.add_glyph(glyph.to_owned());
                (identifier, PDFGlyph::Type3(gid))
            }
            FontContainer::CIDFont(cid_font) => {
                let cid = cid_font.add_glyph(glyph.glyph_id);
                (cid_font.identifier(), PDFGlyph::Cid(cid))
            }
        }
    }
}

pub(crate) trait PdfFont {
    fn units_per_em(&self) -> f32;
    fn font(&self) -> Font;
    fn get_codepoints(&self, pdf_glyph: PDFGlyph) -> Option<&str>;
    fn set_codepoints(
        &mut self,
        pdf_glyph: PDFGlyph,
        text: String,
        location: Option<crate::surface::Location>,
    );
    fn get_gid(&self, glyph: CoveredGlyph) -> Option<PDFGlyph>;
    fn force_fill(&self) -> bool;
}

impl PdfFont for Type3Font {
    fn units_per_em(&self) -> f32 {
        self.unit_per_em()
    }

    fn font(&self) -> Font {
        Type3Font::font(self)
    }

    #[track_caller]
    fn get_codepoints(&self, pdf_glyph: PDFGlyph) -> Option<&str> {
        match pdf_glyph {
            PDFGlyph::Type3(t3) => self.get_codepoints(t3),
            PDFGlyph::Cid(_) => panic!("attempted to pass cid to type 3 font"),
        }
    }

    #[track_caller]
    fn set_codepoints(
        &mut self,
        pdf_glyph: PDFGlyph,
        text: String,
        location: Option<crate::surface::Location>,
    ) {
        match pdf_glyph {
            PDFGlyph::Type3(t3) => self.set_codepoints(t3, text, location),
            PDFGlyph::Cid(_) => panic!("attempted to pass cid to type 3 font"),
        }
    }

    fn get_gid(&self, glyph: CoveredGlyph) -> Option<PDFGlyph> {
        self.get_gid(&glyph.to_owned()).map(PDFGlyph::Type3)
    }

    fn force_fill(&self) -> bool {
        true
    }
}

impl PdfFont for CIDFont {
    fn units_per_em(&self) -> f32 {
        self.units_per_em()
    }

    fn font(&self) -> Font {
        CIDFont::font(self)
    }

    #[track_caller]
    fn get_codepoints(&self, pdf_glyph: PDFGlyph) -> Option<&str> {
        match pdf_glyph {
            PDFGlyph::Type3(_) => panic!("attempted to pass type 3 glyph to cid font"),
            PDFGlyph::Cid(cid) => self.get_codepoints(cid),
        }
    }

    #[track_caller]
    fn set_codepoints(
        &mut self,
        pdf_glyph: PDFGlyph,
        text: String,
        location: Option<crate::surface::Location>,
    ) {
        match pdf_glyph {
            PDFGlyph::Type3(_) => panic!("attempted to pass type 3 glyph to cid font"),
            PDFGlyph::Cid(cid) => self.set_codepoints(cid, text, location),
        }
    }

    fn get_gid(&self, glyph: CoveredGlyph) -> Option<PDFGlyph> {
        self.get_cid(glyph.glyph_id).map(PDFGlyph::Cid)
    }

    fn force_fill(&self) -> bool {
        false
    }
}
