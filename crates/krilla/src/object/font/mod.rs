//! PDF fonts.

use crate::content::PdfFont;
use crate::font::Font;
use crate::object::font::cid_font::CIDFont;
use crate::object::font::type3_font::{CoveredGlyph, Type3FontMapper, Type3ID};
use crate::path::{Fill, Stroke};

pub(crate) mod cid_font;
pub(crate) mod type3_font;

pub(crate) const PDF_UNITS_PER_EM: f32 = 1000.0;

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
