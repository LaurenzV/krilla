//! # Example
//! ```
//! use std::num::NonZeroU32;
//! use krilla::tagging::{TagGroup, TagTree};
//! use krilla::tagging::tag::{TableCellSpan, TableHeaderScope, Tag, TagId};
//!
//! let tag = Tag::TH(TableHeaderScope::Row)
//!     .with_id(TagId::from(*b"this id"))
//!     .with_span(TableCellSpan::col(NonZeroU32::new(3).unwrap()))
//!     .with_headers([TagId::from(*b"parent id")]);
//! let group = TagGroup::new(tag);
//!
//! let mut tree = TagTree::new();
//! tree.push(group);
//! ```

use std::marker::PhantomData;
use std::num::NonZeroU32;

use smallvec::SmallVec;

use crate::surface::Location;

macro_rules! if_present {
    (if ($present:tt) { $then:item } else { $other:item }) => {
        $then
    };
    (if () { $then:item } else { $other:item }) => {
        $other
    };
}

macro_rules! set_attr {
    (attr, ($attrs:ident, $list_attrs:ident, $table_attrs:ident), $variant:ident($name:ident)) => {
        $attrs.set(Attr::$variant($name));
    };
    (list_attr, ($attrs:ident, $list_attrs:ident, $table_attrs:ident), $variant:ident($name:ident)) => {
        $list_attrs.set(ListAttr::$variant($name));
    };
    (table_attr, ($attrs:ident, $list_attrs:ident, $table_attrs:ident), $variant:ident($name:ident)) => {
        $table_attrs.set(TableAttr::$variant($name));
    };
}

macro_rules! tag_kinds {
    (
        $(#[doc = $tag_doc:expr])+
        pub enum TagKind {
            $(
                $(#[doc = $doc:expr])+
                $variant:ident$((
                    $($name:ident: $attr_mod:ident::$required_attr:ident($attr_ty:ty)),*
                    $(; $(Option<$o_attr_mod:ident::$optional_attr:ident>),+)?
                ))?,
            )+
        }
    ) => {
        $(#[doc = $tag_doc])+
        #[derive(Clone, Debug, PartialEq)]
        pub enum TagKind {
            $(
                $(#[doc = $doc])+
                $variant(Tag<$variant>),
            )+
        }

        impl TagKind {
            /// Type erased inner tag.
            pub(crate) fn inner(&self) -> &Tag<()> {
                match self {
                    $(
                        // SAFETY: The tag is only used in PhantomData thus
                        // doesn't have any effect on layout.
                        Self::$variant(tag) => unsafe { std::mem::transmute::<&Tag<$variant>, &Tag<()>>(tag) },
                    )+
                }
            }
        }

        $(
            impl Tag<$variant> {
                if_present! {
                    if ($($($name)*)?) {
                        $(#[doc = $doc])+
                        #[allow(non_snake_case)]
                        pub fn $variant($($($name: $attr_ty),*)?) -> Self {
                            #[allow(unused_mut)]
                            let mut attrs =  BSet::new();
                            #[allow(unused_mut)]
                            let mut list_attrs = BSet::new();
                            #[allow(unused_mut)]
                            let mut table_attrs = BSet::new();

                            $($(
                                set_attr!($attr_mod, (attrs, list_attrs, table_attrs), $required_attr($name));
                            )*)?

                            Tag {
                                attrs,
                                list_attrs,
                                table_attrs,
                                location: None,
                                ty: PhantomData,
                            }
                        }
                    } else {
                        $(#[doc = $doc])+
                        #[allow(non_upper_case_globals)]
                        pub const $variant: Self = Tag {
                            attrs: BSet::new(),
                            list_attrs: BSet::new(),
                            table_attrs: BSet::new(),
                            location: None,
                            ty: PhantomData,
                        };
                    }
                }
            }
        )+

        $(
            $(#[doc = $doc])+
            #[derive(Clone, Debug, PartialEq)]
            pub struct $variant;

            impl From<Tag<$variant>> for TagKind {
                fn from(value: Tag<$variant>) -> Self {
                    TagKind::$variant(value)
                }
            }

            $($(
                $(
                    impl impls::$o_attr_mod::$optional_attr for $variant {}
                )+
            )?)?
        )+
    }
}

tag_kinds! {
    /// A tag kind.
    pub enum TagKind {
        /// A part of a document that may contain multiple articles or sections.
        Part,
        /// An article with largely self-contained content.
        Article,
        /// Section of a larger document.
        Section,
        /// A paragraph-level quote.
        BlockQuote,
        /// An image or figure caption.
        ///
        /// **Best Practice**: In the tag tree, this should appear
        /// as a sibling after the image (or other) content it describes.
        Caption,
        /// Table of contents.
        ///
        /// **Best Practice**: Should consist of TOCIs or other nested TOCs.
        TOC,
        /// Item in the table of contents.
        ///
        /// **Best Practice**: Should only appear within a TOC. Should only consist of
        /// labels, references, paragraphs and TOCs.
        TOCI,
        /// Index of the key terms in the document.
        ///
        /// **Best Practice**: Should contain a sequence of text accompanied by
        /// reference elements pointing to their occurrence in the text.
        Index,
        /// A paragraph.
        P,
        /// Heading level `n`, including an optional title of the heading.
        ///
        /// The title is required for some export modes, like for example PDF/UA.
        Hn(level: attr::HeadingLevel(NonZeroU32); Option<attr::Title>),
        /// A list.
        ///
        /// **Best practice**: Should consist of an optional caption followed by
        /// list items.
        // List numbering is only required for PDF/UA, but we just enforce it for always.
        L(numbering: list_attr::Numbering(ListNumbering)),
        /// A list item.
        ///
        /// **Best practice**: Should consist of one or more list labels and/or list bodies.
        LI,
        /// Label for a list item.
        Lbl,
        /// Description of the list item.
        LBody,
        /// A table, with an optional summary describing the purpose and structure.
        ///
        /// **Best practice**: Should consist of an optional table header row,
        /// one or more table body elements and an optional table footer. Can have
        /// caption as the first or last child.
        Table(; Option<table_attr::Summary>),
        /// A table row.
        ///
        /// **Best practice**: May contain table headers cells and table data cells.
        TR,
        /// A table header cell.
        // Table header scope is only required for PDF/UA, but we include it always for simplicity.
        TH(scope: table_attr::HeaderScope(TableHeaderScope); Option<table_attr::CellHeaders>, Option<table_attr::CellSpan>),
        /// A table data cell.
        TD(; Option<table_attr::CellHeaders>, Option<table_attr::CellSpan>),
        /// A table header row group.
        THead,
        /// A table data row group.
        TBody,
        /// A table footer row group.
        TFoot,
        /// An inline quotation.
        InlineQuote,
        /// A foot- or endnote, potentially referred to from within the text.
        ///
        /// **Best practice**: It may have a label as a child.
        Note,
        /// A reference to elsewhere in the document.
        ///
        /// **Best practice**: The first child of a tag group with this tag should be a link annotation
        /// linking to a destination in the document, and the second child should consist of
        /// the children that should be associated with that reference.
        Reference,
        /// A reference to the external source of some cited document.
        ///
        /// **Best practice**: It may have a label as a child.
        BibEntry,
        /// Computer code.
        Code,
        /// A link.
        ///
        /// **Best practice**: The first child of a tag group with this tag should be a link annotation
        /// linking to an URL, and the second child should consist of the children that should
        /// be associated with that link.
        Link,
        /// An association between an annotation and the content it belongs to. PDF
        ///
        /// **Best practice**: Should be used for all annotations, except for link annotations and
        /// widget annotations. The first child should be the identifier of a non-link annotation,
        /// and all other subsequent children should be content identifiers associated with that
        /// annotation.
        Annot,
        /// Item of graphical content.
        ///
        /// Providing [`Tag::alt_text`] is required in some export modes, like for example PDF/UA1.
        Figure,
        /// A mathematical formula.
        ///
        /// Providing [`Tag::alt_text`] is required in some export modes, like for example PDF/UA1.
        Formula,
        // All below are non-standard attributes.
        /// A date or time.
        Datetime,
        /// A list of terms.
        Terms,
        /// A title.
        Title,
    }
}

/// An ordered set using binary search to find and insert items.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BSet<A> {
    items: Vec<A>,
}

impl<A> BSet<A> {
    pub(crate) const fn new() -> Self {
        Self { items: Vec::new() }
    }
}

impl<A> std::ops::Deref for BSet<A> {
    type Target = [A];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<A: Ordinal> BSet<A> {
    pub(crate) fn set(&mut self, attr: A) {
        let res = self.items.binary_search_by_key(&attr.ordinal(), A::ordinal);
        match res {
            Ok(idx) => self.items[idx] = attr,
            Err(idx) => self.items.insert(idx, attr),
        }
    }

    pub(crate) fn get<U: Unwrap<A>>(&self) -> Option<&U::Item> {
        let idx = self
            .items
            .binary_search_by_key(&U::ORDINAL, A::ordinal)
            .ok()?;
        Some(U::unwrap(&self.items[idx]))
    }
}

pub(crate) trait Ordinal {
    fn ordinal(&self) -> usize;
}

pub(crate) trait Unwrap<A> {
    type Item;

    const ORDINAL: usize;

    fn unwrap(attr: &A) -> &Self::Item;
}

/// A tag for group nodes.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct Tag<T> {
    /// The location of the tag.
    pub location: Option<Location>,
    pub(crate) attrs: BSet<Attr>,
    pub(crate) list_attrs: BSet<ListAttr>,
    pub(crate) table_attrs: BSet<TableAttr>,
    /// The type of this tag containing required attributes.
    pub(crate) ty: PhantomData<T>,
}

impl<T> Tag<T> {
    /// Sets the tag id.
    pub fn with_id(mut self, id: TagId) -> Self {
        self.attrs.set(Attr::Id(id));
        self
    }

    /// The tag id.
    pub fn id(&self) -> Option<&TagId> {
        self.attrs.get::<attr::Id>()
    }

    /// The language of this tag.
    pub fn with_lang(mut self, lang: String) -> Self {
        self.attrs.set(Attr::Lang(lang));
        self
    }
    /// The language of this tag.
    pub fn lang(&self) -> Option<&str> {
        self.attrs.get::<attr::Lang>().map(|s| s.as_str())
    }

    /// An optional alternate text that describes the text (for example, if the text consists
    /// of a star symbol, the alt text should describe that in natural language).
    pub fn with_alt_text(mut self, alt_text: String) -> Self {
        self.attrs.set(Attr::AltText(alt_text));
        self
    }

    /// An optional alternate text that describes the text (for example, if the text consists
    /// of a star symbol, the alt text should describe that in natural language).
    pub fn alt_text(&self) -> Option<&str> {
        self.attrs.get::<attr::AltText>().map(|s| s.as_str())
    }

    /// If the content of the tag is an abbreviation, the expanded form of the
    /// abbreviation should be provided here.
    pub fn with_expanded(mut self, expanded: String) -> Self {
        self.attrs.set(Attr::Expanded(expanded));
        self
    }

    /// If the content of the tag is an abbreviation, the expanded form of the
    /// abbreviation should be provided here.
    pub fn expanded(&self) -> Option<&str> {
        self.attrs.get::<attr::Expanded>().map(|s| s.as_str())
    }

    /// The actual text represented by the content of this tag, i.e. if it contained
    /// some curves that artistically represent some word. This should be the exact
    /// replacement text of the word.
    pub fn with_actual_text(mut self, actual_text: String) -> Self {
        self.attrs.set(Attr::ActualText(actual_text));
        self
    }

    /// The actual text represented by the content of this tag, i.e. if it contained
    /// some curves that artistically represent some word. This should be the exact
    /// replacement text of the word.
    pub fn actual_text(&self) -> Option<&str> {
        self.attrs.get::<attr::ActualText>().map(|s| s.as_str())
    }

    /// Sets the location.
    pub fn with_location(mut self, location: Option<Location>) -> Self {
        self.location = location;
        self
    }
}

macro_rules! gen_struct_and_unwrap_impl {
    ($ordinal:expr; ) => {};
    ($ordinal:expr; $name:ident::$variant:ident($ty:ty) $($tail_name:ident::$tail_variant:ident($tail_ty:ty))*) => {
        #[derive(Clone, Debug, PartialEq)]
        pub struct $variant;

        impl Unwrap<$name> for $variant {
            type Item = $ty;

            const ORDINAL: usize = $ordinal;

            fn unwrap(value: &$name) -> &Self::Item {
                match value {
                    $name::$variant(val) => val,
                    #[allow(unreachable_patterns)]
                    _ => unreachable!(),
                }
            }
        }

        gen_struct_and_unwrap_impl! { $ordinal + 1; $($tail_name::$tail_variant($tail_ty))* }
    };
}

macro_rules! attrs {
    (
        $(
            pub(crate) mod $attr_mod:ident;
            pub(crate) enum $name:ident {
                $(
                    $variant:ident($ty:ty),
                )+
            }
        )+
    ) => {
        $(
            #[derive(Clone, Debug, PartialEq)]
            pub(crate) enum $name {
                $(
                    $variant($ty),
                )+
            }

            impl Ordinal for $name {
                fn ordinal(&self) -> usize {
                    match self {
                        $(
                            $name::$variant(_) => $attr_mod::$variant::ORDINAL,
                        )+
                    }
                }
            }

            pub(crate) mod $attr_mod {
                use super::*;

                gen_struct_and_unwrap_impl! {
                    0_usize;
                    $($name::$variant($ty))+
                }
            }
        )+

        pub(crate) mod impls {
            $(
                pub mod $attr_mod {
                    $(
                        #[allow(unused)]
                        pub trait $variant {}
                    )+
                }
            )+
        }
    }
}

attrs! {
    pub(crate) mod attr;
    pub(crate) enum Attr {
        Id(TagId),
        Lang(String),
        AltText(String),
        Expanded(String),
        ActualText(String),
        Title(String),

        // Not really an attribute, but it fits here quite well.
        HeadingLevel(NonZeroU32),
    }

    pub(crate) mod list_attr;
    pub(crate) enum ListAttr {
        Numbering(ListNumbering),
    }

    pub(crate) mod table_attr;
    pub(crate) enum TableAttr {
        Summary(String),
        HeaderScope(TableHeaderScope),
        CellHeaders(SmallVec<[TagId; 1]>),
        CellSpan(TableCellSpan),
    }
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

impl<T: impls::attr::Title> Tag<T> {
    /// Sets the title.
    pub fn with_title(mut self, title: String) -> Self {
        self.attrs.set(Attr::Title(title));
        self
    }
}

impl<T> Tag<T> {
    /// Gets the title.
    pub fn title(&self) -> Option<&str> {
        self.attrs.get::<attr::Title>().map(|s| s.as_str())
    }
}

impl Tag<Hn> {
    /// The heading level.
    pub fn level(&self) -> NonZeroU32 {
        *self.attrs.get::<attr::HeadingLevel>().unwrap()
    }
}

impl<T: impls::list_attr::Numbering> Tag<T> {
    /// The list numbering.
    pub fn with_numbering(mut self, numbering: ListNumbering) -> Self {
        self.list_attrs.set(ListAttr::Numbering(numbering));
        self
    }
}

impl Tag<L> {
    /// The list numbering.
    pub fn numbering(&self) -> ListNumbering {
        *self.list_attrs.get::<list_attr::Numbering>().unwrap()
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

impl<T: impls::table_attr::Summary> Tag<T> {
    /// Sets the summary.
    pub fn with_summary(mut self, summary: String) -> Self {
        self.table_attrs.set(TableAttr::Summary(summary));
        self
    }
}

impl Tag<TH> {
    /// The table header scope.
    pub fn header_scope(&self) -> TableHeaderScope {
        *self.table_attrs.get::<table_attr::HeaderScope>().unwrap()
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

impl<T: impls::table_attr::CellHeaders> Tag<T> {
    /// A list of headers associated with a table cell.
    /// Table data cells (`TD`) may specify a list of table headers (`TH`),
    /// which can also specify a list of parent header cells (`TH`), and so on.
    /// To determine the list of associated headers this list is recursively
    /// evaluated.
    ///
    /// This allows specifying header hierarchies inside tables.
    pub fn with_headers(mut self, headers: impl IntoIterator<Item = TagId>) -> Self {
        let headers = headers.into_iter().collect();
        self.table_attrs.set(TableAttr::CellHeaders(headers));
        self
    }
}

impl<T> Tag<T> {
    pub(crate) fn table_headers(&self) -> Option<&[TagId]> {
        self.table_attrs
            .get::<table_attr::CellHeaders>()
            .map(|s| s.as_slice())
    }
}

impl<T: impls::table_attr::CellSpan> Tag<T> {
    /// Sets the row/column span of this table cell.
    pub fn with_span(mut self, span: TableCellSpan) -> Self {
        self.table_attrs.set(TableAttr::CellSpan(span));
        self
    }
}

/// The span of a table cell.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TableCellSpan {
    /// The number of spanned rows inside the enclosing table.
    pub rows: NonZeroU32,
    /// The number of spanned cells inside the enclosing table.
    pub cols: NonZeroU32,
}

impl Default for TableCellSpan {
    fn default() -> Self {
        Self::ONE
    }
}

impl TableCellSpan {
    /// A table cell that spans only one row and column.
    pub const ONE: Self = Self::new(NonZeroU32::MIN, NonZeroU32::MIN);

    /// Create a new table cell span.
    pub const fn new(rows: NonZeroU32, cols: NonZeroU32) -> Self {
        Self { rows, cols }
    }

    /// Create a new table cell span that spans a number of rows.
    pub const fn row(rows: NonZeroU32) -> Self {
        Self {
            rows,
            cols: NonZeroU32::MIN,
        }
    }

    /// Create a new table cell span that spans a number of columns.
    pub const fn col(cols: NonZeroU32) -> Self {
        Self {
            rows: NonZeroU32::MIN,
            cols,
        }
    }

    pub(crate) fn row_span(self) -> Option<NonZeroU32> {
        (self.rows != NonZeroU32::MIN).then_some(self.rows)
    }

    pub(crate) fn col_span(self) -> Option<NonZeroU32> {
        (self.cols != NonZeroU32::MIN).then_some(self.cols)
    }
}
