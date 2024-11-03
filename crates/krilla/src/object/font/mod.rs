use crate::font::Font;
use crate::object::font::type3_font::Type3ID;
use crate::path::{Fill, Stroke};
use crate::resource::RegisterableResource;

pub(crate) mod cid_font;
pub(crate) mod type3_font;

impl PaintMode<'_> {
    pub fn to_owned(self) -> OwnedPaintMode {
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

impl RegisterableResource<crate::resource::Font> for FontIdentifier {}

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
    pub fn as_ref(&self) -> PaintMode {
        match self {
            OwnedPaintMode::Fill(f) => PaintMode::Fill(f),
            OwnedPaintMode::Stroke(s) => PaintMode::Stroke(s),
        }
    }
}
