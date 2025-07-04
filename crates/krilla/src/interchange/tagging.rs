//! Creating accessible PDF documents.
//!
//! # Introduction
//!
//! A document usually consists of many smaller semantic building blocks, like for example
//! titles, headings, paragraphs, tables, headers, footers, and so on. However, it is important
//! to understand that, by default, when exporting a document to PDF, all of this semantic
//! information is lost.
//!
//! By default, a PDF document doesn't have a notion of a table or a
//! paragraph. Instead, it consists of very low-level instructions, such as "draw
//! a path at that location" or "draw a line of glyphs at that font size". A table is
//! not encoded as a "table", but instead as a number of rectangle-like paths that happen
//! to surround lines of text. This is what made PDF popular in the first place, as encoding
//! information at such a low level allows to ensure a consistent viewing experience across
//! different platforms and viewers.
//!
//! However, this design has drawbacks, one of the main ones being that it leads to the
//! production of non-accessible documents. Especially in recent years, ensuring the
//! accessibility of documents has become an increasingly important requirement.
//! To address this deficiency, PDF introduced the notion of "tagged PDF", which consists
//! of enriching PDF documents with additional semantic information in a way that can be
//! interpreted by different consumers. krilla supports the creation of such documents.
//!
//! # A word on krilla's implementation
//!
//! As nearly everything in PDF, tagging is a really complex topic, and getting it right
//! is very hard. Because of this, in line with the general philosophy of krilla, some
//! of the potential capabilities of tagged PDF are not directly supported. Instead, only
//! a specific subset has been implemented, with a focus on features that improve the
//! accessibility of documents.
//!
//! The goal in krilla's implementation of tagged PDF is to make it possible for users to create
//! well-tagged PDF files simply by following the instructions in the documentation,
//! without having to read or consult the PDF specification.
//!
//! # Basic Principles
//!
//! The way tagged PDFs are created is by attaching a tag tree to the PDF document
//! that encodes the logical structure of the document. As mentioned above, a raw PDF file
//! mainly consists of text- and path-drawing primitives, which are not necessarily
//! drawn in the logical reading order of the document. What the tag tree does is
//! maping the different "snippets" of the PDF file to the tree-like structure in a way
//! that reflects the logical structure of the document, in reading-order.
//!
//! For example, a document can consist of multiple "sections", where each section might contain
//! headings, paragraphs or figures. A figure might consist of a table as well as a caption.
//! A table consists yet again of smaller semantic components, like a header, footer
//! and the data cells, which usually contain some text. These kinds of hierarchical structures
//! can be encoded with the help of tagging.
//!
//! A tag tree consists of two components:
//! - Group nodes, which represents a component with certain semantics. A group node must have
//!   at least one child, otherwise it's discarded.
//! - Leaf nodes, which represent the actual pieces on the page that form part of a group.
//!
//! # How to create a tagged document
//!
//! If you want to create a tagged document, you need to follow the following steps:
//!
//! 1) Ensure that you activate the `enable_tagging` attribute in [`SerializeSettings`].
//! 2) Create a [tag tree](TagTree), which represents the "root" of a tag tree.
//! 3) As you create your document, create new [tag groups](TagGroup) with corresponding [tags](Tag).
//!    Nest them with other tag groups, if necessary, by using the `push` method.
//! 4) Populate tag groups with [identifiers](Identifier), which represent the leaf nodes
//!    in the tag tree. Identifiers are unique and point to a sequence of content on the
//!    page. If you push an identifier to a tag group, then all content that is marked by
//!    that identifier belongs semantically to that tag group. There are currently two ways
//!    of obtaining an identifier:
//!
//!    - Use the `add_tagged_annotation` method on [`Page`], which allows you to associate
//!      annotations to the content they correspond to. Currently, krilla only supports link
//!      annotations, and a link annotation should always be a child in a tag group with the
//!      [Tag] `Link`, with its sibling being an identifier or another tag group that is
//!      to be associated with the link.
//!    - Use the `start_tagged` command on [`Surface`], which returns an [Identifier], and
//!      indicates that all content drawn on the surface should be associated with that
//!      identifier, until you call the `end_tagged` method. *Important*: Note that you cannot
//!      nest calls to `start_tagged`, and you have to ensure that you always call a corresponding
//!      `end_tagged`. Otherwise, krilla will panic.
//!
//!      It is very important that each identifier you create has exactly one parent in the tag
//!      tree. This means that you cannot create an identifier and not use it at all (0 parents),
//!      or use the same identifier in two different parts of the tree (1+ parents). Otherwise,
//!      export will fail.
//!
//! 5) Once you have built your tag tree, simply call `set_tag_tree` on [`Document`]. That's it!
//!
//! # Other notes
//!
//! Make sure that you carefully read the documentation of the other parts of this module, as
//! there are some more points as well as best practices you need to keep in mind
//! when creating well-tagged documents. The PDF specification is in some places very vague
//! on how a tagged document should look like, so there is quite a bit of room for flexibility.
//!
//! Apart from that, the PDF specification does make a few statements on requirements a well-tagged PDF
//! should follows, although  those are not really "strict" requirements in the sense that they can
//! be automatically checked by a PDF validator, so not conforming to some of those points does not
//! suddenly make your document a badly-tagged document! However, if possible, you should still
//! try to comply with the following requirements:
//!
//! - In general, all contents in your file should be tagged, either as an artifact or with
//!   Span/Other.
//! - The order of elements in the tag tree should represent the logical reading order, including
//!   annotations.
//! - Word breaks in text should be represented explicitly with spaces, instead of implicitly
//!   by not including them, but instead positioning text in a way that "simulates" the spaces.
//! - Hyphenation should be represented as a soft hyphen character (U+00AD) instead
//!   of a hard hyphen (U+002D).
//! - Tag groups should follow the best-practice of what kind of children they contain. See
//!   [Tag] for more information.
//! - You should provide "Alt" descriptions for formulas and images.
//! - In case you have a link annotation that applies to text that wraps into one or multiple
//!   new lines, you should use the `quad_points` functionality to indicate the exact covered
//!   areas of the link.
//!
//! [`SerializeSettings`]: crate::SerializeSettings
//! [`Page`]: crate::page::Page
//! [`Surface`]: crate::surface::Surface
//! [`Document`]: crate::Document

use std::cmp::PartialEq;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::io::Write as _;
use std::num::NonZeroU32;

use pdf_writer::types::{ArtifactSubtype, StructRole};
use pdf_writer::writers::{PropertyList, StructElement, TableAttributes};
use pdf_writer::{Chunk, Finish, Name, Ref, Str, TextStr};
use smallvec::SmallVec;

use crate::configure::{PdfVersion, ValidationError};
use crate::error::KrillaResult;
use crate::serialize::SerializeContext;
use crate::surface::Location;
use crate::util::lazy::{LazyGet, LazyInit};

/// A type of artifact.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ArtifactType {
    /// The header of a page.
    Header,
    /// The footer of the page.
    Footer,
    /// Page artifacts, such as for example cut marks or color bars.
    Page,
    /// Any other type of artifact (e.g. table strokes).
    Other,
}

impl ArtifactType {
    pub(crate) fn requires_properties(&self) -> bool {
        match self {
            ArtifactType::Header => true,
            ArtifactType::Footer => true,
            ArtifactType::Page => true,
            ArtifactType::Other => false,
        }
    }
}

/// A language identifier as specified in RFC 3066. It will not be validated, so
/// it's on the user of the library to ensure the tag is valid.
pub type Lang<'a> = &'a str;
/// An alternate text that describes some element in natural language.
pub type Alt<'a> = &'a str;
/// The actual intended textual content of a span. For example,
/// if you have a hyphenated word, you can use `ActualText` to describe
/// the same word without hyphens.
pub type ActualText<'a> = &'a str;
/// The expanded form of an abbreviation.
pub type Expanded<'a> = &'a str;

/// A content tag associated with the content it wraps.
#[derive(Clone, Copy, Debug)]
pub enum ContentTag<'a> {
    /// Artifacts represent pieces of content that are not really part of the logical structure
    /// of a document and should be excluded in the logical tree. These include for example headers,
    /// footers, page background and similar.
    Artifact(ArtifactType),
    /// A content tag that wraps some text with specific properties.
    ///
    /// Spans should not be too long. At most, they should contain a single line of text, but they
    /// can obviously be shorter, if text within a single line contains text with different styles
    /// or different languages.
    Span(SpanTag<'a>),
    /// Use this tag for anything else that does not semantically fit into `Span` or `Artifact`.
    /// This includes for example arbitrary paths, images or a mix of different content that cannot
    /// be split up more.
    Other,
}

impl ContentTag<'_> {
    pub(crate) fn name(&self) -> Name {
        match self {
            ContentTag::Artifact(_) => Name(b"Artifact"),
            ContentTag::Span(_) => Name(b"Span"),
            ContentTag::Other => Name(b"P"),
        }
    }

    pub(crate) fn write_properties(&self, sc: &mut SerializeContext, mut properties: PropertyList) {
        match self {
            ContentTag::Artifact(at) => {
                let mut artifact = properties.artifact();

                let artifact_type = match at {
                    ArtifactType::Header => pdf_writer::types::ArtifactType::Pagination,
                    ArtifactType::Footer => pdf_writer::types::ArtifactType::Pagination,
                    ArtifactType::Page => pdf_writer::types::ArtifactType::Page,
                    // This method should only be called with artifacts that actually
                    // require a property.
                    ArtifactType::Other => unreachable!(),
                };

                if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf17 {
                    if *at == ArtifactType::Header {
                        artifact.attached([pdf_writer::types::ArtifactAttachment::Top]);
                        artifact.subtype(ArtifactSubtype::Header);
                    }

                    if *at == ArtifactType::Footer {
                        artifact.attached([pdf_writer::types::ArtifactAttachment::Bottom]);
                        artifact.subtype(ArtifactSubtype::Footer);
                    }
                }

                artifact.kind(artifact_type);
            }
            ContentTag::Span(SpanTag {
                lang,
                alt_text,
                expanded,
                actual_text,
            }) => {
                if let Some(lang) = lang {
                    properties.pair(Name(b"Lang"), TextStr(lang));
                }

                if let Some(alt) = alt_text {
                    if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf15 {
                        properties.pair(Name(b"Alt"), TextStr(alt));
                    }
                }

                if let Some(exp) = expanded {
                    properties.pair(Name(b"E"), TextStr(exp));
                }

                if let Some(actual) = actual_text {
                    if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf15 {
                        properties.actual_text(TextStr(actual));
                    }
                }
            }
            ContentTag::Other => {}
        }
    }
}

/// A span tag.
#[derive(Clone, Copy, Debug)]
pub struct SpanTag<'a> {
    /// The language of the text.
    pub lang: Option<Lang<'a>>,
    /// An optional alternate text that describes the text (for example, if the text consists
    /// of a star symbol, the alt text should describe that in natural language).
    pub alt_text: Option<Alt<'a>>,
    /// If the content of the span is an abbreviation, the expanded form of the
    /// abbreviation should be provided here.
    pub expanded: Option<Expanded<'a>>,
    /// The actual text represented by the glyphs, i.e. if you have a hyphenated span
    /// `row-`, then you can wrap it in an `ActualText` to remove the hyphenation
    /// when copy-pasting.
    pub actual_text: Option<ActualText<'a>>,
}

impl<'a> SpanTag<'a> {
    /// An empty span tag.
    pub fn empty() -> Self {
        Self {
            lang: None,
            alt_text: None,
            expanded: None,
            actual_text: None,
        }
    }

    /// Sets [`SpanTag::lang`].
    pub fn with_lang(mut self, lang: Option<&'a str>) -> Self {
        self.lang = lang;
        self
    }

    /// Sets [`SpanTag::alt_text`].
    pub fn with_alt_text(mut self, alt_text: Option<&'a str>) -> Self {
        self.alt_text = alt_text;
        self
    }

    /// Sets [`SpanTag::expanded`].
    pub fn with_expanded(mut self, expanded: Option<&'a str>) -> Self {
        self.expanded = expanded;
        self
    }

    /// Sets [`SpanTag::actual_text`].
    pub fn with_actual_text(mut self, actual_text: Option<&'a str>) -> Self {
        self.actual_text = actual_text;
        self
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PageTagIdentifier {
    pub(crate) page_index: usize,
    pub(crate) mcid: i32,
}

impl From<PageTagIdentifier> for IdentifierType {
    fn from(value: PageTagIdentifier) -> Self {
        IdentifierType::PageIdentifier(value)
    }
}

impl From<PageTagIdentifier> for Identifier {
    fn from(value: PageTagIdentifier) -> Self {
        Identifier(IdentifierInner::Real(value.into()))
    }
}

impl PageTagIdentifier {
    pub(crate) fn new(page_index: usize, mcid: i32) -> Self {
        Self { page_index, mcid }
    }

    pub(crate) fn bump(&mut self) -> PageTagIdentifier {
        let old = *self;

        self.mcid = self.mcid.checked_add(1).unwrap();

        old
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct AnnotationIdentifier {
    pub(crate) page_index: usize,
    pub(crate) annot_index: usize,
}

impl From<AnnotationIdentifier> for IdentifierType {
    fn from(value: AnnotationIdentifier) -> Self {
        IdentifierType::AnnotationIdentifier(value)
    }
}

impl From<AnnotationIdentifier> for Identifier {
    fn from(value: AnnotationIdentifier) -> Self {
        Identifier(IdentifierInner::Real(value.into()))
    }
}

impl AnnotationIdentifier {
    pub fn new(page_index: usize, annot_index: usize) -> Self {
        Self {
            page_index,
            annot_index,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum IdentifierType {
    PageIdentifier(PageTagIdentifier),
    AnnotationIdentifier(AnnotationIdentifier),
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum IdentifierInner {
    Real(IdentifierType),
    Dummy,
}

/// An identifier for an annotation or certain parts of page content.
///
/// Need to be used as a leaf node in a tag tree.
#[derive(Copy, Clone)]
pub struct Identifier(pub(crate) IdentifierInner);

impl Identifier {
    pub(crate) fn new_annotation(page_index: usize, annot_index: usize) -> Self {
        AnnotationIdentifier::new(page_index, annot_index).into()
    }

    pub(crate) fn dummy() -> Self {
        Self(IdentifierInner::Dummy)
    }
}

/// A tag for group nodes.
#[derive(Debug, Clone)]
pub struct Tag {
    /// The structure element type.
    pub kind: TagKind,
    /// The identifier of this tag.
    /// Used in [`TableCellHeaders`].
    pub id: Option<TagId>,
    /// The language of this tag.
    pub lang: Option<String>,
    /// An optional alternate text that describes the text (for example, if the text consists
    /// of a star symbol, the alt text should describe that in natural language).
    pub alt_text: Option<String>,
    /// If the content of the tag is an abbreviation, the expanded form of the
    /// abbreviation should be provided here.
    pub expanded: Option<String>,
    /// The actual text represented by the content of this tag, i.e. if it contained
    /// some curves that artistically write some word. This should be the exact
    /// replacment text of the word.
    pub actual_text: Option<String>,
    /// The location of the tag.
    pub location: Option<Location>,
}

impl From<TagKind> for Tag {
    fn from(kind: TagKind) -> Self {
        Self::new(kind)
    }
}

impl Tag {
    /// Create a new tag with a specific kind.
    pub fn new(kind: TagKind) -> Self {
        Self {
            kind,
            id: None,
            lang: None,
            alt_text: None,
            expanded: None,
            actual_text: None,
            location: None,
        }
    }
}

/// Builder methods for a [`Tag`].
pub trait TagBuilder: Into<Tag> {
    /// Sets [`Tag::id`].
    fn with_id(self, id: Option<TagId>) -> Tag {
        let mut tag = self.into();
        tag.id = id;
        tag
    }

    /// Sets [`Tag::lang`].
    fn with_lang(self, lang: Option<String>) -> Tag {
        let mut tag = self.into();
        tag.lang = lang;
        tag
    }

    /// Sets [`Tag::alt_text`].
    fn with_alt_text(self, alt_text: Option<String>) -> Tag {
        let mut tag = self.into();
        tag.alt_text = alt_text;
        tag
    }

    /// Sets [`Tag::expanded`].
    fn with_expanded(self, expanded: Option<String>) -> Tag {
        let mut tag = self.into();
        tag.expanded = expanded;
        tag
    }

    /// Sets [`Tag::actual_text`].
    fn with_actual_text(self, actual_text: Option<String>) -> Tag {
        let mut tag = self.into();
        tag.actual_text = actual_text;
        tag
    }

    /// Sets [`Tag::location`].
    fn with_location(self, location: Option<Location>) -> Tag {
        let mut tag = self.into();
        tag.location = location;
        tag
    }
}

impl<T: Into<Tag>> TagBuilder for T {}

/// A structure element type.
#[derive(Debug, Clone, Eq, PartialEq)]
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
    Hn(NonZeroU32, Option<String>),
    /// A list.
    ///
    /// **Best practice**: Should consist of an optional caption followed by
    /// list items.
    // List numbering is only required for PDF/UA, but we just enforce it for always.
    L(ListNumbering),
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
    Table(Option<String>),
    /// A table row.
    ///
    /// **Best practice**: May contain table headers cells and table data cells.
    TR,
    /// A table header cell.
    // Table header scope is only required for PDF/UA, but we include it always for simplicity.
    TH(TableHeaderCell),
    /// A table data cell.
    TD(TableDataCell),
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

impl TagKind {
    pub(crate) fn write_kind(&self, struct_elem: &mut StructElement, sc: &mut SerializeContext) {
        let pdf_version = sc.serialize_settings().pdf_version();
        if self.minimum_version() > pdf_version {
            // Fall back to P in case the tag is not supported with the current
            // PDF version
            struct_elem.kind(StructRole::P);
            return;
        }

        match self {
            Self::Part => struct_elem.kind(StructRole::Part),
            Self::Article => struct_elem.kind(StructRole::Art),
            Self::Section => struct_elem.kind(StructRole::Sect),
            Self::BlockQuote => struct_elem.kind(StructRole::BlockQuote),
            Self::Caption => struct_elem.kind(StructRole::Caption),
            Self::TOC => struct_elem.kind(StructRole::TOC),
            Self::TOCI => struct_elem.kind(StructRole::TOCI),
            Self::Index => struct_elem.kind(StructRole::Index),
            Self::P => struct_elem.kind(StructRole::P),
            Self::Hn(n, _) if n.get() == 1 => struct_elem.kind(StructRole::H1),
            Self::Hn(n, _) if n.get() == 2 => struct_elem.kind(StructRole::H2),
            Self::Hn(n, _) if n.get() == 3 => struct_elem.kind(StructRole::H3),
            Self::Hn(n, _) if n.get() == 4 => struct_elem.kind(StructRole::H4),
            Self::Hn(n, _) if n.get() == 5 => struct_elem.kind(StructRole::H5),
            Self::Hn(n, _) if n.get() == 6 => struct_elem.kind(StructRole::H6),
            Self::L(_) => struct_elem.kind(StructRole::L),
            Self::LI => struct_elem.kind(StructRole::LI),
            Self::Lbl => struct_elem.kind(StructRole::Lbl),
            Self::LBody => struct_elem.kind(StructRole::LBody),
            Self::Table(_) => struct_elem.kind(StructRole::Table),
            Self::TR => struct_elem.kind(StructRole::TR),
            Self::TH(_) => struct_elem.kind(StructRole::TH),
            Self::TD(_) => struct_elem.kind(StructRole::TD),
            Self::THead => struct_elem.kind(StructRole::THead),
            Self::TBody => struct_elem.kind(StructRole::TBody),
            Self::TFoot => struct_elem.kind(StructRole::TFoot),
            Self::InlineQuote => struct_elem.kind(StructRole::Quote),
            Self::Note => struct_elem.kind(StructRole::Note),
            Self::Reference => struct_elem.kind(StructRole::Reference),
            Self::BibEntry => struct_elem.kind(StructRole::BibEntry),
            Self::Code => struct_elem.kind(StructRole::Code),
            Self::Link => struct_elem.kind(StructRole::Link),
            Self::Annot => struct_elem.kind(StructRole::Annot),
            Self::Figure => struct_elem.kind(StructRole::Figure),
            Self::Formula => struct_elem.kind(StructRole::Formula),
            // Every additional tag needs to be registered in the role map!
            Self::Datetime => struct_elem.custom_kind(Name(b"Datetime")),
            Self::Terms => struct_elem.custom_kind(Name(b"Terms")),
            Self::Title => struct_elem.custom_kind(Name(b"Title")),
            Self::Hn(level, _) => {
                // Dynamically register custom headings `Hn` with `n >= 7`
                if pdf_version < PdfVersion::Pdf20 {
                    sc.global_objects.custom_heading_roles.insert(*level);
                }
                let name = format!("H{level}");
                struct_elem.custom_kind(Name(name.as_bytes()))
            }
        };
    }

    pub(crate) fn minimum_version(&self) -> PdfVersion {
        match self {
            Self::Part => PdfVersion::Pdf14,
            Self::Article => PdfVersion::Pdf14,
            Self::Section => PdfVersion::Pdf14,
            Self::BlockQuote => PdfVersion::Pdf14,
            Self::Caption => PdfVersion::Pdf14,
            Self::TOC => PdfVersion::Pdf14,
            Self::TOCI => PdfVersion::Pdf14,
            Self::Index => PdfVersion::Pdf14,
            Self::P => PdfVersion::Pdf14,
            Self::Hn(_, _) => PdfVersion::Pdf14,
            Self::L(_) => PdfVersion::Pdf14,
            Self::LI => PdfVersion::Pdf14,
            Self::Lbl => PdfVersion::Pdf14,
            Self::LBody => PdfVersion::Pdf14,
            Self::Table(_) => PdfVersion::Pdf14,
            Self::TR => PdfVersion::Pdf14,
            Self::TH(_) => PdfVersion::Pdf14,
            Self::TD(_) => PdfVersion::Pdf14,
            Self::THead => PdfVersion::Pdf15,
            Self::TBody => PdfVersion::Pdf15,
            Self::TFoot => PdfVersion::Pdf15,
            Self::InlineQuote => PdfVersion::Pdf14,
            Self::Note => PdfVersion::Pdf14,
            Self::Reference => PdfVersion::Pdf14,
            Self::BibEntry => PdfVersion::Pdf14,
            Self::Code => PdfVersion::Pdf14,
            Self::Link => PdfVersion::Pdf14,
            Self::Annot => PdfVersion::Pdf15,
            Self::Figure => PdfVersion::Pdf15,
            Self::Formula => PdfVersion::Pdf15,
            Self::Datetime => PdfVersion::Pdf15,
            Self::Terms => PdfVersion::Pdf15,
            Self::Title => PdfVersion::Pdf15,
        }
    }

    pub(crate) fn should_have_alt(&self) -> bool {
        matches!(self, TagKind::Figure | TagKind::Formula)
    }

    pub(crate) fn title(&self) -> Option<&str> {
        match self {
            Self::Hn(_, s) => s.as_deref(),
            _ => None,
        }
    }

    pub(crate) fn can_have_title(&self) -> bool {
        matches!(self, Self::Hn(_, _))
    }
}

/// A node in a tag tree.
pub enum Node {
    /// A group node.
    Group(TagGroup),
    /// A leaf node.
    Leaf(Identifier),
}

impl Node {
    pub(crate) fn serialize(
        &self,
        sc: &mut SerializeContext,
        parent_tree_map: &mut HashMap<IdentifierType, Ref>,
        id_tree: &mut BTreeMap<TagId, Ref>,
        parent: Ref,
        note_id: &mut u32,
        struct_elems: &mut Vec<Chunk>,
    ) -> KrillaResult<Option<Reference>> {
        match self {
            Node::Group(g) => Ok(Some(g.serialize(
                sc,
                parent_tree_map,
                id_tree,
                parent,
                note_id,
                struct_elems,
            )?)),
            Node::Leaf(ci) => match ci.0 {
                IdentifierInner::Real(rci) => Ok(Some(Reference::ContentIdentifier(rci))),
                IdentifierInner::Dummy => Ok(None),
            },
        }
    }
}

impl From<TagGroup> for Node {
    fn from(value: TagGroup) -> Self {
        Node::Group(value)
    }
}

impl From<Identifier> for Node {
    fn from(value: Identifier) -> Self {
        Node::Leaf(value)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Reference {
    Ref(Ref),
    ContentIdentifier(IdentifierType),
}

/// A tag group.
pub struct TagGroup {
    /// The tag of the tag group.
    tag: Tag,
    /// The children of the tag group.
    children: Vec<Node>,
}

impl TagGroup {
    /// Create a new tag group with a specific tag.
    pub fn new(tag: impl Into<Tag>) -> Self {
        Self {
            tag: tag.into(),
            children: vec![],
        }
    }

    /// Create a new tag group with a specific tag and a list of children.
    pub fn with_children(tag: Tag, children: Vec<Node>) -> Self {
        Self { tag, children }
    }

    /// Append a new child to the tag group.
    pub fn push(&mut self, child: impl Into<Node>) {
        self.children.push(child.into())
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializeContext,
        parent_tree_map: &mut HashMap<IdentifierType, Ref>,
        id_tree: &mut BTreeMap<TagId, Ref>,
        parent_ref: Ref,
        note_id: &mut u32,
        struct_elems: &mut Vec<Chunk>,
    ) -> KrillaResult<Reference> {
        let elem_ref = sc.new_ref();
        let mut children_refs = vec![];

        for child in &self.children {
            let serialized = child.serialize(
                sc,
                parent_tree_map,
                id_tree,
                elem_ref,
                note_id,
                struct_elems,
            )?;
            if let Some(ref_) = serialized {
                children_refs.push(ref_);
            }
        }

        let mut chunk = Chunk::new();
        let mut struct_elem = chunk.struct_element(elem_ref);
        self.tag.kind.write_kind(&mut struct_elem, sc);
        struct_elem.parent(parent_ref);

        if let Some(id) = &self.tag.id {
            match id_tree.entry(id.clone()) {
                Entry::Vacant(vacant) => {
                    struct_elem.id(Str(id.as_bytes()));
                    vacant.insert(elem_ref);
                }
                Entry::Occupied(_) => {
                    sc.register_validation_error(ValidationError::DuplicateTagId(
                        id.clone(),
                        self.tag.location,
                    ));
                }
            }
        } else if TagKind::Note == self.tag.kind {
            // Explicitly don't use `TagId::from_bytes` to disambiguate note IDs
            // from user provided IDs.
            let mut id = TagId(SmallVec::new());
            _ = write!(&mut id.0, "Note {}", note_id);
            struct_elem.id(Str(id.as_bytes()));
            id_tree.insert(id, elem_ref);

            *note_id += 1;
        }

        if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf14 {
            if let Some(lang) = &self.tag.lang {
                struct_elem.lang(TextStr(lang));
            }
        }

        if let Some(alt) = &self.tag.alt_text {
            struct_elem.alt(TextStr(alt));
        } else if self.tag.kind.should_have_alt() {
            sc.register_validation_error(ValidationError::MissingAltText);
        }

        if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf15 {
            if let Some(expanded) = &self.tag.expanded {
                struct_elem.expanded(TextStr(expanded));
            }
        }

        if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf14 {
            if let Some(actual_text) = &self.tag.actual_text {
                struct_elem.actual_text(TextStr(actual_text));
            }
        }

        if let Some(title) = self.tag.kind.title() {
            struct_elem.title(TextStr(title));
        } else if self.tag.kind.can_have_title() {
            sc.register_validation_error(ValidationError::MissingHeadingTitle);
        }

        match self.tag.kind {
            TagKind::L(ln) => {
                struct_elem
                    .attributes()
                    .push()
                    .list()
                    .list_numbering(ln.to_pdf());
            }
            TagKind::TH(ref cell) => {
                // Laziliy initialize the table attributes, to avoid an empty list.
                let mut attributes = LazyInit::new(&mut struct_elem, |elem| elem.attributes());
                let mut table_attributes =
                    LazyInit::new(&mut attributes, |attrs| attrs.get().push().table());

                if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf15 {
                    table_attributes.get().scope(cell.scope.to_pdf());
                }
                serialize_table_cell_attributes(sc, &mut table_attributes, &cell.data);
            }
            TagKind::TD(ref cell) => {
                // Laziliy initialize the table attributes, to avoid an empty list.
                let mut attributes = LazyInit::new(&mut struct_elem, |elem| elem.attributes());
                let mut table_attributes =
                    LazyInit::new(&mut attributes, |attrs| attrs.get().push().table());

                serialize_table_cell_attributes(sc, &mut table_attributes, cell);
            }
            _ => {}
        }

        serialize_children(
            sc,
            elem_ref,
            children_refs,
            parent_tree_map,
            &mut struct_elem,
        )?;
        struct_elem.finish();
        struct_elems.push(chunk);

        Ok(Reference::Ref(elem_ref))
    }

    fn validate(&self, sc: &mut SerializeContext, id_tree: &BTreeMap<TagId, Ref>) {
        match &self.tag.kind {
            TagKind::TH(TableHeaderCell { data, .. }) | TagKind::TD(data) => {
                if let Some(ids) = data.headers.header_ids() {
                    for id in ids.iter() {
                        if !id_tree.contains_key(id) {
                            sc.register_validation_error(ValidationError::UnknownHeaderTagId(
                                id.clone(),
                                self.tag.location,
                            ));
                        }
                    }
                }
            }
            _ => (),
        }

        for child in self.children.iter() {
            if let Node::Group(group) = child {
                group.validate(sc, id_tree)
            }
        }
    }
}

fn serialize_table_cell_attributes<'a: 'b, 'b>(
    sc: &mut SerializeContext,
    mut table_attributes: impl LazyGet<TableAttributes<'a>>,
    cell: &TableDataCell,
) {
    if sc.serialize_settings().pdf_version() >= PdfVersion::Pdf15 {
        if let Some(ids) = cell.headers.header_ids() {
            let id_strs = ids.iter().map(|id| Str(id.as_bytes()));
            table_attributes.lazy_get().headers().items(id_strs);
        }
    }
    if let Some(n) = cell.span.row_span() {
        table_attributes.lazy_get().row_span(n.get() as i32);
    }
    if let Some(n) = cell.span.col_span() {
        table_attributes.lazy_get().col_span(n.get() as i32);
    }
}

/// A tag tree.
#[derive(Default)]
pub struct TagTree {
    children: Vec<Node>,
}

impl From<Vec<Node>> for TagTree {
    fn from(children: Vec<Node>) -> Self {
        Self { children }
    }
}

impl TagTree {
    /// Create a new tag tree.
    pub fn new() -> Self {
        Self { children: vec![] }
    }

    /// Append a new child to the tag tree.
    pub fn push(&mut self, child: impl Into<Node>) {
        self.children.push(child.into())
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializeContext,
        parent_tree_map: &mut HashMap<IdentifierType, Ref>,
        id_tree_map: &mut BTreeMap<TagId, Ref>,
        struct_tree_ref: Ref,
    ) -> KrillaResult<(Ref, Vec<Chunk>)> {
        let root_ref = sc.new_ref();
        let mut struct_elems = vec![];

        // Keeps track of the ID of notes in the IDTree. We currently only write IDs for notes,
        // which is why we use this simple variable, but this should be refactored if we write
        // the IDs for multiple types of struct elements in the future.
        let mut note_id = 1;

        let mut children_refs = vec![];

        for child in &self.children {
            let serialized = child.serialize(
                sc,
                parent_tree_map,
                id_tree_map,
                root_ref,
                &mut note_id,
                &mut struct_elems,
            )?;

            if let Some(ref_) = serialized {
                children_refs.push(ref_);
            }
        }

        let mut chunk = Chunk::new();
        let mut struct_elem = chunk.indirect(root_ref).start::<StructElement>();
        struct_elem.kind(StructRole::Document);
        struct_elem.parent(struct_tree_ref);
        serialize_children(
            sc,
            root_ref,
            children_refs,
            parent_tree_map,
            &mut struct_elem,
        )?;

        struct_elem.finish();
        struct_elems.push(chunk);

        // Not strictly necessary, but it's nicer to have them in DFS-order instead
        // of in reverse.
        struct_elems = struct_elems.into_iter().rev().collect::<Vec<_>>();

        Ok((root_ref, struct_elems))
    }

    pub(crate) fn validate(&self, sc: &mut SerializeContext, id_tree: &BTreeMap<TagId, Ref>) {
        for child in self.children.iter() {
            if let Node::Group(group) = child {
                group.validate(sc, id_tree)
            }
        }
    }
}

fn serialize_children(
    sc: &mut SerializeContext,
    parent_ref: Ref,
    children_refs: Vec<Reference>,
    parent_tree_map: &mut HashMap<IdentifierType, Ref>,
    struct_elem: &mut StructElement,
) -> KrillaResult<()> {
    // We can define a /Pg element on the struct element. If a marked content reference
    // is part of the same page as that entry, we can just write the mcid, otherwise, we
    // need to write a full marked content reference.
    // In our case, we just use the first marked content reference we can find as the
    // entry in the /Pg dict.
    let mut struct_page_ref = None;
    let mut struct_children = struct_elem.children();

    for child in children_refs {
        match child {
            Reference::Ref(r) => {
                struct_children.struct_element(r);
            }
            Reference::ContentIdentifier(it) => match it {
                IdentifierType::PageIdentifier(pi) => {
                    let page_ref = sc
                            .page_infos()
                            .get(pi.page_index)
                            .unwrap_or_else(|| panic!("tag tree contains identifier from page {}, but document only has {} pages",
                                pi.page_index + 1,
                                sc.page_infos().len()))
                            .ref_;

                    if struct_page_ref.is_none() {
                        struct_page_ref = Some(page_ref);
                    }

                    if parent_tree_map.contains_key(&pi.into()) {
                        panic!("the identifier {:?} appears twice in the tag tree", pi);
                    }

                    parent_tree_map.insert(pi.into(), parent_ref);

                    if struct_page_ref == Some(page_ref) {
                        struct_children.marked_content_id(pi.mcid);
                    } else {
                        struct_children
                            .marked_content_ref()
                            .marked_content_id(pi.mcid)
                            .page(page_ref);
                    }
                }
                IdentifierType::AnnotationIdentifier(ai) => {
                    let Some(page_info) = sc.page_infos_mut().get_mut(ai.page_index) else {
                        panic!(
                            "tag tree contains identifier from page {}, but document only has {} pages",
                            ai.page_index + 1,
                            sc.page_infos().len()
                        );
                    };

                    let page_ref = page_info.ref_;
                    let Some((annotation_ref, struct_parent)) =
                        page_info.annotations.get_mut(ai.annot_index)
                    else {
                        panic!(
                            "tag tree contains identifier from annotation {} on page {}, but page only has {} annotations",
                            ai.annot_index + 1,
                            ai.page_index + 1,
                            page_info.annotations.len()
                        );
                    };

                    if parent_tree_map.contains_key(&ai.into()) {
                        panic!("identifier {:?} appears twice in the tag tree", ai);
                    }
                    parent_tree_map.insert(ai.into(), *annotation_ref);

                    struct_parent.set(parent_ref).expect("only one parent");

                    struct_children
                        .object_ref()
                        .page(page_ref)
                        .object(*annotation_ref);
                }
            },
        }
    }
    struct_children.finish();

    if let Some(spr) = struct_page_ref {
        struct_elem.page(spr);
    }

    Ok(())
}

/// Where a layout artifact is attached to the page edge.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum ArtifactAttachment {
    Left,
    Top,
    Right,
    Bottom,
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
    fn to_pdf(self) -> pdf_writer::types::ListNumbering {
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

/// A table header cell.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TableHeaderCell {
    /// The scope of the table header.
    pub scope: TableHeaderScope,
    /// Attributes shared with `TD`.
    pub data: TableDataCell,
}

impl TableHeaderCell {
    /// Create a new table header cell.
    pub const fn new(scope: TableHeaderScope) -> Self {
        Self {
            scope,
            data: TableDataCell::new(),
        }
    }

    /// Sets [`TableDataCell::headers`].
    pub fn with_headers(mut self, headers: TableCellHeaders) -> Self {
        self.data.headers = headers;
        self
    }

    /// Sets [`TableDataCell::span`].
    pub fn with_span(mut self, span: TableCellSpan) -> Self {
        self.data.span = span;
        self
    }
}

/// A table data cell.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct TableDataCell {
    /// A list of associated headers.
    pub headers: TableCellHeaders,
    /// The column/row span of the table.
    pub span: TableCellSpan,
}

impl TableDataCell {
    /// Create a new table data cell.
    pub const fn new() -> Self {
        Self {
            headers: TableCellHeaders::NONE,
            span: TableCellSpan::ONE,
        }
    }

    /// Sets [`TableDataCell::headers`].
    pub fn with_headers(mut self, headers: TableCellHeaders) -> Self {
        self.headers = headers;
        self
    }

    /// Sets [`TableDataCell::span`].
    pub fn with_span(mut self, span: TableCellSpan) -> Self {
        self.span = span;
        self
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
    fn to_pdf(self) -> pdf_writer::types::TableHeaderScope {
        match self {
            TableHeaderScope::Row => pdf_writer::types::TableHeaderScope::Row,
            TableHeaderScope::Column => pdf_writer::types::TableHeaderScope::Column,
            TableHeaderScope::Both => pdf_writer::types::TableHeaderScope::Both,
        }
    }
}

/// A list of headers associated with a table cell.
/// Table data cells (`TD`) may specify a list of table headers (`TH`),
/// which can also specify a list of parent header cells (`TH`), and so on.
/// To determine the the list of associated headers this list is recursively
/// evaluated.
///
/// This allows specifying header hierarchies inside tables.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct TableCellHeaders {
    /// The list of header IDs.
    pub ids: SmallVec<[TagId; 1]>,
}

impl TableCellHeaders {
    /// An empty reference list.
    pub const NONE: Self = Self {
        ids: SmallVec::new_const(),
    };

    fn header_ids(&self) -> Option<&[TagId]> {
        (!self.ids.is_empty()).then_some(&self.ids)
    }
}

/// An identifier of a [`Tag`].
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TagId(SmallVec<[u8; 16]>);

impl TagId {
    /// Create an identifier from a byte slice.
    pub fn from_slice(bytes: &[u8]) -> Self {
        let mut inner = SmallVec::with_capacity(bytes.len() + 1);
        // HACK: Disambiguate ids provided by the user from ids automatically
        // assigned to notes by prefixing them with a `U`.
        inner.push(b'U');
        inner.extend_from_slice(bytes);
        Self(inner)
    }

    /// Create an identifier from a byte vec.
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        let mut inner = SmallVec::from_vec(bytes);
        // HACK: Disambiguate ids provided by the user from ids automatically
        // assigned to notes by prefixing them with a `U`.
        inner.insert(0, b'U');
        Self(inner)
    }

    /// Returns the identifier as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
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

    fn row_span(self) -> Option<NonZeroU32> {
        (self.rows != NonZeroU32::MIN).then_some(self.rows)
    }

    fn col_span(self) -> Option<NonZeroU32> {
        (self.cols != NonZeroU32::MIN).then_some(self.cols)
    }
}
