//! # Example
//! ```
//! use std::num::NonZeroU32;
//! use krilla::tagging::{TagGroup, TagTree};
//! use krilla::tagging::tag::{TableHeaderScope, Tag, TagId};
//!
//! let tag = Tag::TH(TableHeaderScope::Row)
//!     .with_id(Some(TagId::from(*b"this id")))
//!     .with_col_span(Some(NonZeroU32::new(3).unwrap()))
//!     .with_headers([TagId::from(*b"parent id")])
//!     .with_width(Some(250.0))
//!     .with_height(Some(100.0));
//! let group = TagGroup::new(tag);
//!
//! let mut tree = TagTree::new();
//! tree.push(group);
//! ```

use std::marker::PhantomData;
use std::num::NonZeroU32;

use smallvec::SmallVec;

use crate::geom::Rect;
use crate::surface::Location;

include!("generated.rs");

impl TagKind {
    /// The location.
    pub fn location(&self) -> Option<Location> {
        self.as_any().location
    }

    /// The location.
    pub fn set_location(&mut self, location: Option<Location>) {
        self.as_any_mut().location = location;
    }

    /// The location.
    pub fn with_location(mut self, location: Option<Location>) -> Self {
        self.as_any_mut().location = location;
        self
    }
}

/// A raw tag, which allows reading all attributes and additionally writing all
/// global ones.
#[derive(Clone, Debug, PartialEq)]
pub struct Tag<T> {
    inner: AnyTag,
    /// Compile time marker for a type-safe API.
    pub(crate) ty: PhantomData<T>,
}

/// A "raw" tag that allows reading all attributes and writing global ones.
#[derive(Clone, Debug, PartialEq)]
pub struct AnyTag {
    /// The location of the tag.
    pub location: Option<Location>,
    pub(crate) attrs: OrdinalSet<Attr>,
    pub(crate) list_attrs: OrdinalSet<ListAttr>,
    pub(crate) table_attrs: OrdinalSet<TableAttr>,
    pub(crate) layout_attrs: OrdinalSet<LayoutAttr>,
}

impl AnyTag {
    pub(crate) const fn new() -> Self {
        Self {
            attrs: OrdinalSet::new(),
            list_attrs: OrdinalSet::new(),
            table_attrs: OrdinalSet::new(),
            layout_attrs: OrdinalSet::new(),
            location: None,
        }
    }
}

impl<T> Tag<T> {
    /// This can't be public, otherwise tags could be constructed without
    /// providing required attributes.
    pub(crate) const fn new() -> Self {
        Self {
            inner: AnyTag::new(),
            ty: PhantomData,
        }
    }

    /// A raw tag, which allows reading all attributes.
    pub fn as_any(&self) -> &AnyTag {
        &self.inner
    }

    /// A raw tag, which allows reading all attributes and additionally writing
    /// all global ones.
    pub fn as_any_mut(&mut self) -> &mut AnyTag {
        &mut self.inner
    }

    /// The location.
    pub fn with_location(mut self, location: Option<Location>) -> Self {
        self.inner.location = location;
        self
    }
}

/// An ordered set using ordinal numbers to sort and identify elements.
/// Due to the strict ordering, elements can be located using binary search.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct OrdinalSet<A> {
    items: Vec<A>,
}

impl<A> OrdinalSet<A> {
    pub(crate) const fn new() -> Self {
        Self { items: Vec::new() }
    }
}

impl<A> std::ops::Deref for OrdinalSet<A> {
    type Target = [A];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<A: Ordinal> OrdinalSet<A> {
    pub(crate) fn set(&mut self, attr: A) {
        let res = self.items.binary_search_by_key(&attr.ordinal(), A::ordinal);
        match res {
            Ok(idx) => self.items[idx] = attr,
            Err(idx) => self.items.insert(idx, attr),
        }
    }

    pub(crate) fn remove<const ORDINAL: usize>(&mut self) {
        let idx = self.items.binary_search_by_key(&ORDINAL, A::ordinal);
        if let Ok(idx) = idx {
            self.items.remove(idx);
        }
    }

    pub(crate) fn get<const ORDINAL: usize>(&self) -> Option<&A> {
        let idx = self.items.binary_search_by_key(&ORDINAL, A::ordinal).ok()?;
        Some(&self.items[idx])
    }

    pub(crate) fn set_or_remove<const ORDINAL: usize>(&mut self, attr: Option<A>) {
        match attr {
            Some(attr) => self.set(attr),
            None => self.remove::<ORDINAL>(),
        }
    }
}

/// Identifies elements using an ordianl number.
pub(crate) trait Ordinal {
    fn ordinal(&self) -> usize;
}

/// An identifier of a [`Tag`].
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TagId(pub(crate) SmallVec<[u8; 16]>);

impl<I: IntoIterator<Item = u8>> From<I> for TagId {
    fn from(value: I) -> Self {
        // Disambiguate ids provided by the user from ids automatically assigned
        // to notes by prefixing them with a `U`.
        let bytes = std::iter::once(b'U').chain(value).collect();
        TagId(bytes)
    }
}

impl TagId {
    /// Returns the identifier as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

/// The list numbering type.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ListNumbering {
    /// No numbering.
    None,
    /// Solid circular bullets.
    Disc,
    /// Open circular bullets.
    Circle,
    /// Solid square bullets.
    Square,
    /// Decimal numbers.
    Decimal,
    /// Lowercase Roman numerals.
    LowerRoman,
    /// Uppercase Roman numerals.
    UpperRoman,
    /// Lowercase letters.
    LowerAlpha,
    /// Uppercase letters.
    UpperAlpha,
}

impl ListNumbering {
    pub(crate) fn to_pdf(self) -> pdf_writer::types::ListNumbering {
        match self {
            ListNumbering::None => pdf_writer::types::ListNumbering::None,
            ListNumbering::Disc => pdf_writer::types::ListNumbering::Disc,
            ListNumbering::Circle => pdf_writer::types::ListNumbering::Circle,
            ListNumbering::Square => pdf_writer::types::ListNumbering::Square,
            ListNumbering::Decimal => pdf_writer::types::ListNumbering::Decimal,
            ListNumbering::LowerRoman => pdf_writer::types::ListNumbering::LowerRoman,
            ListNumbering::UpperRoman => pdf_writer::types::ListNumbering::UpperRoman,
            ListNumbering::LowerAlpha => pdf_writer::types::ListNumbering::LowerAlpha,
            ListNumbering::UpperAlpha => pdf_writer::types::ListNumbering::UpperAlpha,
        }
    }
}

/// The scope of a table header cell.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum TableHeaderScope {
    /// The header cell refers to the row.
    Row,
    /// The header cell refers to the column.
    Column,
    /// The header cell refers to both the row and the column.
    Both,
}

impl TableHeaderScope {
    pub(crate) fn to_pdf(self) -> pdf_writer::types::TableHeaderScope {
        match self {
            TableHeaderScope::Row => pdf_writer::types::TableHeaderScope::Row,
            TableHeaderScope::Column => pdf_writer::types::TableHeaderScope::Column,
            TableHeaderScope::Both => pdf_writer::types::TableHeaderScope::Both,
        }
    }
}

/// The positioning of the element with respect to the enclosing reference area
/// and other content.
/// When applied to an ILSE, any value except Inline shall cause the element to
/// be treated as a BLSE instead.
///
/// Default value: [`Placement::Inline`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Placement {
    /// tacked in the block-progression direction within an enclosing reference
    /// area or parent BLSE.
    Block,
    /// Packed in the inline-progression direction within an enclosing BLSE.
    #[default]
    Inline,
    /// Placed so that the before edge of the element’s allocation rectangle.
    /// (see “Content and Allocation Rectangles” in 14.8.5.4, “Layout Attributes”)
    /// coincides with that of the nearest enclosing reference area. The element
    /// may float, if necessary, to achieve the specified placement. The element
    /// shall be treated as a block occupying the full extent of the enclosing
    /// reference area in the inline direction. Other content shall be stacked
    /// so as to begin at the after edge of the element’s allocation rectangle.
    Before,
    /// Placed so that the start edge of the element’s allocation rectangle
    /// (see “Content and Allocation Rectangles” in 14.8.5.4, “Layout Attributes”)
    /// coincides with that of the nearest enclosing reference area. The element
    /// may float, if necessary, to achieve the specified placement. Other
    /// content that would intrude into the element’s allocation rectangle
    /// shall be laid out as a runaround.
    Start,
    /// Placed so that the end edge of the element’s allocation rectangle
    /// (see “Content and Allocation Rectangles” in 14.8.5.4, “Layout Attributes”)
    /// coincides with that of the nearest enclosing reference area. The element
    /// may float, if necessary, to achieve the specified placement. Other
    /// content that would intrude into the element’s allocation rectangle
    /// shall be laid out as a runaround.
    End,
}

impl Placement {
    pub(crate) fn to_pdf(self) -> pdf_writer::types::Placement {
        match self {
            Placement::Block => pdf_writer::types::Placement::Block,
            Placement::Inline => pdf_writer::types::Placement::Inline,
            Placement::Before => pdf_writer::types::Placement::Before,
            Placement::Start => pdf_writer::types::Placement::Start,
            Placement::End => pdf_writer::types::Placement::End,
        }
    }
}

/// The directions of layout progression for packing of ILSEs (inline progression)
/// and stacking of BLSEs (block progression).
/// The specified layout directions shall apply to the given structure element
/// and all of its descendants to any level of nesting.
///
/// Default value: [`WritingMode::LrTb`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WritingMode {
    /// Inline progression from left to right; block progression from top to
    /// bottom. This is the typical writing mode for Western writing systems.
    #[default]
    LrTb,
    /// Inline progression from right to left; block progression from top to
    /// bottom. This is the typical writing mode for Arabic and Hebrew writing
    /// systems.
    RlTb,
    /// Inline progression from top to bottom; block progression from right to
    /// left. This is the typical writing mode for Chinese and Japanese writing
    /// systems.
    TbRl,
}

impl WritingMode {
    pub(crate) fn to_pdf(self) -> pdf_writer::types::WritingMode {
        match self {
            WritingMode::LrTb => pdf_writer::types::WritingMode::LtrTtb,
            WritingMode::RlTb => pdf_writer::types::WritingMode::RtlTtb,
            WritingMode::TbRl => pdf_writer::types::WritingMode::TtbRtl,
        }
    }
}
