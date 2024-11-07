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
//!     It is very important that each identifier you create has exactly one parent in the tag
//!     tree. This means that you cannot create an identifier and not use it at all (0 parents),
//!     or use the same identifier in two different parts of the tree (1+ parents). Otherwise,
//!     export will fail.
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
//!
//! [`SerializeSettings`]: crate::SerializeSettings
//! [`Page`]: crate::page::Page
//! [`Surface`]: crate::surface::Surface
//! [`Document`]: crate::Document

// TODO: Other notes: broken links should use quadpoint (14.8.4.4.2)

use crate::error::{KrillaError, KrillaResult};
use crate::serialize::SerializerContext;
use crate::validation::ValidationError;
use crate::version::PdfVersion;
use pdf_writer::types::{
    ArtifactAttachment, ArtifactSubtype, ListNumbering, StructRole, TableHeaderScope,
};
use pdf_writer::writers::{PropertyList, StructElement};
use pdf_writer::{Chunk, Finish, Name, Ref, Str, TextStr};
use std::cmp::PartialEq;
use std::collections::{BTreeMap, HashMap};

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
    /// A content tag that wraps some text with specific properties. The properties you can/need to
    /// set are:
    /// - Lang: The language of the text. If the language is unknown, pass an empty string to it.
    /// - Alt: An optional alternate text that describes the text (for example, if the text consists
    ///   a star symbol, the alt text should describe that in natural language).
    /// - Expanded: If the content of the span is an abbreviation, the expanded form of the
    ///   abbreviation should be provided here.
    ///
    /// Spans should not be too long. At most, they should contain a single like of text, but they
    /// can obviously be shorter, if text within a single line contains text with different styles
    /// or different languages.
    Span(
        Lang<'a>,
        Option<Alt<'a>>,
        Option<Expanded<'a>>,
        Option<ActualText<'a>>,
    ),
    /// Use this tag for anything else that does not semantically fit into `Span` or `Artifact`.
    /// This includes for example arbitrary paths, images or a mix of different content that cannot
    /// be split up more.
    Other,
}

impl ContentTag<'_> {
    pub(crate) fn name(&self) -> Name {
        match self {
            ContentTag::Artifact(_) => Name(b"Artifact"),
            ContentTag::Span(_, _, _, _) => Name(b"Span"),
            ContentTag::Other => Name(b"P"),
        }
    }

    pub(crate) fn write_properties(
        &self,
        sc: &mut SerializerContext,
        mut properties: PropertyList,
    ) {
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

                if sc.serialize_settings().pdf_version >= PdfVersion::Pdf17 {
                    if *at == ArtifactType::Header {
                        artifact.attached([ArtifactAttachment::Top]);
                        artifact.subtype(ArtifactSubtype::Header);
                    }

                    if *at == ArtifactType::Footer {
                        artifact.attached([ArtifactAttachment::Bottom]);
                        artifact.subtype(ArtifactSubtype::Footer);
                    }
                }

                artifact.kind(artifact_type);
            }
            ContentTag::Span(lang, alt, exp, actual) => {
                properties.pair(Name(b"Lang"), TextStr(lang));

                if let Some(alt) = alt {
                    if sc.serialize_settings().pdf_version >= PdfVersion::Pdf15 {
                        properties.pair(Name(b"Alt"), TextStr(alt));
                    }
                }

                if let Some(exp) = exp {
                    properties.pair(Name(b"E"), TextStr(exp));
                }

                if let Some(actual) = actual {
                    if sc.serialize_settings().pdf_version >= PdfVersion::Pdf15 {
                        properties.actual_text(TextStr(actual));
                    }
                }
            }
            ContentTag::Other => {}
        }
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
    pub fn new(page_index: usize, mcid: i32) -> Self {
        Self { page_index, mcid }
    }

    pub fn bump(&mut self) -> PageTagIdentifier {
        let old = *self;

        self.mcid = self.mcid.checked_add(1).unwrap();

        old
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct AnnotationIdentifier {
    page_index: usize,
    annot_index: usize,
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
pub enum Tag {
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
    /// First-level heading, including an optional title of the heading.
    ///
    /// The title is required for some export modes, like for example PDF/UA.
    H1(Option<String>),
    /// Second-level heading, including an optional title of the heading.
    ///
    /// The title is required for some export modes, like for example PDF/UA.
    H2(Option<String>),
    /// Third-level heading, including an optional title of the heading.
    ///
    /// The title is required for some export modes, like for example PDF/UA.
    H3(Option<String>),
    /// Fourth-level heading, including an optional title of the heading.
    ///
    /// The title is required for some export modes, like for example PDF/UA.
    H4(Option<String>),
    /// Fifth-level heading, including an optional title of the heading.
    ///
    /// The title is required for some export modes, like for example PDF/UA.
    H5(Option<String>),
    /// Sixth-level heading, including an optional title of the heading.
    ///
    /// The title is required for some export modes, like for example PDF/UA.
    H6(Option<String>),
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
    /// A table.
    ///
    /// **Best practice**: Should consist of an optional table header row,
    /// one or more table body elements and an optional table footer. Can have
    /// caption as the first or last child.
    Table,
    /// A table row.
    ///
    /// **Best practice**: May contain table headers cells and table data cells.
    TR,
    /// A table header cell.
    // Table header scope is only required for PDF/UA, but we include it always for simplicity.
    TH(TableHeaderScope),
    /// A table data cell.
    TD,
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
    /// Item of graphical content, with some optional alt text.
    ///
    /// Providing the alt text is required in some export modes, like for example PDF/UA1.
    Figure(Option<String>),
    /// A mathematical formula, with some optional alt text.
    ///
    /// Providing the alt text is required in some export modes, like for example PDF/UA1.
    Formula(Option<String>),
    // All below are non-standard attributes.
    /// A date or time.
    Datetime,
    /// A list of terms.
    Terms,
    /// A title.
    Title,
}

impl Tag {
    pub(crate) fn write_kind(&self, struct_elem: &mut StructElement, pdf_version: PdfVersion) {
        if self.minimum_version() > pdf_version {
            // Fall back to P in case the tag is not supported with the current
            // PDF version
            struct_elem.kind(StructRole::P);
        } else {
            match self {
                Tag::Part => struct_elem.kind(StructRole::Part),
                Tag::Article => struct_elem.kind(StructRole::Art),
                Tag::Section => struct_elem.kind(StructRole::Sect),
                Tag::BlockQuote => struct_elem.kind(StructRole::BlockQuote),
                Tag::Caption => struct_elem.kind(StructRole::Caption),
                Tag::TOC => struct_elem.kind(StructRole::TOC),
                Tag::TOCI => struct_elem.kind(StructRole::TOCI),
                Tag::Index => struct_elem.kind(StructRole::Index),
                Tag::P => struct_elem.kind(StructRole::P),
                Tag::H1(_) => struct_elem.kind(StructRole::H1),
                Tag::H2(_) => struct_elem.kind(StructRole::H2),
                Tag::H3(_) => struct_elem.kind(StructRole::H3),
                Tag::H4(_) => struct_elem.kind(StructRole::H4),
                Tag::H5(_) => struct_elem.kind(StructRole::H5),
                Tag::H6(_) => struct_elem.kind(StructRole::H6),
                Tag::L(_) => struct_elem.kind(StructRole::L),
                Tag::LI => struct_elem.kind(StructRole::LI),
                Tag::Lbl => struct_elem.kind(StructRole::Lbl),
                Tag::LBody => struct_elem.kind(StructRole::LBody),
                Tag::Table => struct_elem.kind(StructRole::Table),
                Tag::TR => struct_elem.kind(StructRole::TR),
                Tag::TH(_) => struct_elem.kind(StructRole::TH),
                Tag::TD => struct_elem.kind(StructRole::TD),
                Tag::THead => struct_elem.kind(StructRole::THead),
                Tag::TBody => struct_elem.kind(StructRole::TBody),
                Tag::TFoot => struct_elem.kind(StructRole::TFoot),
                Tag::InlineQuote => struct_elem.kind(StructRole::Quote),
                Tag::Note => struct_elem.kind(StructRole::Note),
                Tag::Reference => struct_elem.kind(StructRole::Reference),
                Tag::BibEntry => struct_elem.kind(StructRole::BibEntry),
                Tag::Code => struct_elem.kind(StructRole::Code),
                Tag::Link => struct_elem.kind(StructRole::Link),
                Tag::Annot => struct_elem.kind(StructRole::Annot),
                Tag::Figure(_) => struct_elem.kind(StructRole::Figure),
                Tag::Formula(_) => struct_elem.kind(StructRole::Formula),
                // Every additional tag needs to be registered in the role map!
                Tag::Datetime => struct_elem.custom_kind(Name(b"Datetime")),
                Tag::Terms => struct_elem.custom_kind(Name(b"Terms")),
                Tag::Title => struct_elem.custom_kind(Name(b"Title")),
            };
        }
    }

    pub(crate) fn can_have_alt(&self) -> bool {
        matches!(self, Tag::Figure(_) | Tag::Formula(_))
    }

    pub(crate) fn alt(&self) -> Option<&str> {
        match self {
            Tag::Figure(s) => s.as_deref(),
            Tag::Formula(s) => s.as_deref(),
            _ => None,
        }
    }

    pub(crate) fn minimum_version(&self) -> PdfVersion {
        match self {
            Tag::Part => PdfVersion::Pdf14,
            Tag::Article => PdfVersion::Pdf14,
            Tag::Section => PdfVersion::Pdf14,
            Tag::BlockQuote => PdfVersion::Pdf14,
            Tag::Caption => PdfVersion::Pdf14,
            Tag::TOC => PdfVersion::Pdf14,
            Tag::TOCI => PdfVersion::Pdf14,
            Tag::Index => PdfVersion::Pdf14,
            Tag::P => PdfVersion::Pdf14,
            Tag::H1(_) => PdfVersion::Pdf14,
            Tag::H2(_) => PdfVersion::Pdf14,
            Tag::H3(_) => PdfVersion::Pdf14,
            Tag::H4(_) => PdfVersion::Pdf14,
            Tag::H5(_) => PdfVersion::Pdf14,
            Tag::H6(_) => PdfVersion::Pdf14,
            Tag::L(_) => PdfVersion::Pdf14,
            Tag::LI => PdfVersion::Pdf14,
            Tag::Lbl => PdfVersion::Pdf14,
            Tag::LBody => PdfVersion::Pdf14,
            Tag::Table => PdfVersion::Pdf14,
            Tag::TR => PdfVersion::Pdf14,
            Tag::TH(_) => PdfVersion::Pdf14,
            Tag::TD => PdfVersion::Pdf14,
            Tag::THead => PdfVersion::Pdf15,
            Tag::TBody => PdfVersion::Pdf15,
            Tag::TFoot => PdfVersion::Pdf15,
            Tag::InlineQuote => PdfVersion::Pdf14,
            Tag::Note => PdfVersion::Pdf14,
            Tag::Reference => PdfVersion::Pdf14,
            Tag::BibEntry => PdfVersion::Pdf14,
            Tag::Code => PdfVersion::Pdf14,
            Tag::Link => PdfVersion::Pdf14,
            Tag::Annot => PdfVersion::Pdf15,
            Tag::Figure(_) => PdfVersion::Pdf15,
            Tag::Formula(_) => PdfVersion::Pdf15,
            Tag::Datetime => PdfVersion::Pdf15,
            Tag::Terms => PdfVersion::Pdf15,
            Tag::Title => PdfVersion::Pdf15,
        }
    }

    pub(crate) fn title(&self) -> Option<&str> {
        match self {
            Tag::H1(s) => s.as_deref(),
            Tag::H2(s) => s.as_deref(),
            Tag::H3(s) => s.as_deref(),
            Tag::H4(s) => s.as_deref(),
            Tag::H5(s) => s.as_deref(),
            Tag::H6(s) => s.as_deref(),
            _ => None,
        }
    }

    pub(crate) fn can_have_title(&self) -> bool {
        matches!(
            self,
            Tag::H1(_) | Tag::H2(_) | Tag::H3(_) | Tag::H4(_) | Tag::H5(_) | Tag::H6(_)
        )
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
        sc: &mut SerializerContext,
        parent_tree_map: &mut HashMap<IdentifierType, Ref>,
        id_tree: &mut BTreeMap<String, Ref>,
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
    pub fn new(tag: Tag) -> Self {
        Self {
            tag,
            children: vec![],
        }
    }

    /// Append a new child to the tag group.
    pub fn push(&mut self, child: impl Into<Node>) {
        self.children.push(child.into())
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        parent_tree_map: &mut HashMap<IdentifierType, Ref>,
        id_tree: &mut BTreeMap<String, Ref>,
        parent: Ref,
        note_id: &mut u32,
        struct_elems: &mut Vec<Chunk>,
    ) -> KrillaResult<Reference> {
        let root_ref = sc.new_ref();
        let mut children_refs = vec![];

        for child in &self.children {
            let serialized =
                child.serialize(sc, parent_tree_map, id_tree, parent, note_id, struct_elems)?;
            if let Some(ref_) = serialized {
                children_refs.push(ref_);
            }
        }

        let mut chunk = Chunk::new();
        let mut struct_elem = chunk.struct_element(root_ref);
        self.tag
            .write_kind(&mut struct_elem, sc.serialize_settings().pdf_version);
        struct_elem.parent(parent);

        if let Some(alt) = self.tag.alt() {
            struct_elem.alt(TextStr(alt));
        } else if self.tag.can_have_alt() {
            sc.register_validation_error(ValidationError::MissingAltText);
        }

        if let Some(title) = self.tag.title() {
            struct_elem.title(TextStr(title));
        } else if self.tag.can_have_title() {
            sc.register_validation_error(ValidationError::MissingHeadingTitle);
        }

        match self.tag {
            Tag::L(ln) => {
                struct_elem.attributes().push().list().list_numbering(ln);
            }
            Tag::TH(ths) => {
                if sc.serialize_settings().pdf_version >= PdfVersion::Pdf15 {
                    struct_elem.attributes().push().table().scope(ths);
                }
            }
            Tag::Note => {
                let id = format!("Note {}", note_id);
                *note_id += 1;
                id_tree.insert(id.clone(), root_ref);
                struct_elem.id(Str(id.as_bytes()));
            }
            _ => {}
        }

        serialize_children(
            sc,
            root_ref,
            children_refs,
            parent_tree_map,
            &mut struct_elem,
        )?;
        struct_elem.finish();
        struct_elems.push(chunk);

        Ok(Reference::Ref(root_ref))
    }
}

/// A tag tree.
#[derive(Default)]
pub struct TagTree {
    children: Vec<Node>,
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
        sc: &mut SerializerContext,
        parent_tree_map: &mut HashMap<IdentifierType, Ref>,
        id_tree_map: &mut BTreeMap<String, Ref>,
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
}

fn serialize_children(
    sc: &mut SerializerContext,
    root_ref: Ref,
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
                            .ok_or(KrillaError::UserError(format!(
                                "tag tree contains identifier from page {}, but document only has {} pages",
                                pi.page_index + 1,
                                sc.page_infos().len()
                            )))?
                            .ref_;

                    if struct_page_ref.is_none() {
                        struct_page_ref = Some(page_ref);
                    }

                    if parent_tree_map.contains_key(&pi.into()) {
                        return Err(KrillaError::UserError(
                            "an identifier appears twice in the tag tree".to_string(),
                        ));
                    }

                    parent_tree_map.insert(pi.into(), root_ref);

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
                    let page_info =
                        sc.page_infos()
                            .get(ai.page_index)
                            .ok_or(KrillaError::UserError(format!(
                        "tag tree contains identifier from page {}, but document only has {} pages",
                        ai.page_index + 1,
                        sc.page_infos().len()
                    )))?;

                    let page_ref = page_info.ref_;
                    let annotation_ref =
                            *page_info.annotations
                                .get(ai.annot_index)
                                .ok_or(KrillaError::UserError(format!(
                                    "tag tree contains identifier from annotation {} on page {}, but page only has {} annotations",
                                    ai.annot_index + 1,
                                    ai.page_index + 1,
                                    page_info.annotations.len()
                                )))?;

                    if parent_tree_map.contains_key(&ai.into()) {
                        return Err(KrillaError::UserError(
                            "an identifier appears twice in the tag tree".to_string(),
                        ));
                    }
                    parent_tree_map.insert(ai.into(), annotation_ref);

                    struct_children
                        .object_ref()
                        .page(page_ref)
                        .object(annotation_ref);
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

#[cfg(test)]
mod tests {
    use crate::action::{Action, LinkAction};
    use crate::annotation::{LinkAnnotation, Target};
    use crate::error::KrillaError;
    use crate::font::Font;
    use crate::path::Fill;
    use crate::surface::{Surface, TextDirection};
    use crate::tagging::{ArtifactType, ContentTag, Tag, TagGroup, TagTree};
    use crate::tests::{green_fill, load_png_image, rect_to_path, NOTO_SANS, SVGS_PATH};
    use crate::{Document, SvgSettings};
    use krilla_macros::snapshot;
    use tiny_skia_path::{Rect, Transform};

    pub trait SurfaceExt {
        fn fill_text_(&mut self, y: f32, content: &str);
    }

    impl SurfaceExt for Surface<'_> {
        fn fill_text_(&mut self, y: f32, content: &str) {
            let font_data = NOTO_SANS.clone();
            let font = Font::new(font_data, 0, vec![]).unwrap();

            self.fill_text(
                tiny_skia_path::Point::from_xy(0.0, y),
                Fill::default(),
                font,
                20.0,
                &[],
                content,
                false,
                TextDirection::Auto,
            );
        }
    }

    #[snapshot(document)]
    fn tagging_empty(document: &mut Document) {
        let tag_root = TagTree::new();
        document.set_tag_tree(tag_root);
    }

    fn tagging_simple_impl(document: &mut Document) {
        let mut tag_tree = TagTree::new();
        let mut par = TagGroup::new(Tag::P);

        let mut page = document.start_page();
        let mut surface = page.surface();
        let id = surface.start_tagged(ContentTag::Span(
            "en",
            Some("an alt text"),
            Some("expanded"),
            Some("actual text"),
        ));
        surface.fill_text_(25.0, "a paragraph");
        surface.end_tagged();

        surface.finish();
        page.finish();

        par.push(id);
        tag_tree.push(par);

        document.set_tag_tree(tag_tree);
    }

    fn tagging_simple_with_link_impl(document: &mut Document) {
        let mut tag_tree = TagTree::new();
        let mut par = TagGroup::new(Tag::P);
        let mut link = TagGroup::new(Tag::Link);

        let mut page = document.start_page();
        let mut surface = page.surface();
        let id = surface.start_tagged(ContentTag::Span("", None, None, None));
        surface.fill_text_(25.0, "a paragraph");
        surface.end_tagged();

        surface.finish();

        let link_id = page.add_tagged_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(0.0, 0.0, 100.0, 25.0).unwrap(),
                Target::Action(Action::Link(LinkAction::new("www.youtube.com".to_string()))),
            )
            .into(),
        );

        page.finish();

        link.push(link_id);
        link.push(id);
        par.push(link);
        tag_tree.push(par);

        document.set_tag_tree(tag_tree);
    }

    #[snapshot(document)]
    fn tagging_simple(document: &mut Document) {
        tagging_simple_impl(document);
    }

    #[snapshot(document)]
    fn tagging_simple_with_link(document: &mut Document) {
        tagging_simple_with_link_impl(document);
    }

    #[snapshot(document, settings_12)]
    fn tagging_disabled(document: &mut Document) {
        tagging_simple_impl(document);
    }

    #[snapshot(document, settings_12)]
    fn tagging_disabled_2(document: &mut Document) {
        tagging_simple_with_link_impl(document);
    }

    pub(crate) fn sample_svg() -> usvg::Tree {
        let data = std::fs::read(SVGS_PATH.join("resvg_shapes_rect_simple_case.svg")).unwrap();
        usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap()
    }

    #[snapshot(document)]
    fn tagging_image_with_alt(document: &mut Document) {
        let mut tag_tree = TagTree::new();
        let mut image_group =
            TagGroup::new(Tag::Figure(Some("This is the alternate text.".to_string())));

        let mut page = document.start_page();
        let mut surface = page.surface();

        let id = surface.start_tagged(ContentTag::Other);
        let tree = sample_svg();
        surface.draw_svg(&tree, tree.size(), SvgSettings::default());
        surface.end_tagged();

        surface.finish();
        page.finish();

        image_group.push(id);
        tag_tree.push(image_group);

        document.set_tag_tree(tag_tree);
    }

    #[snapshot(document)]
    fn tagging_multiple_content_tags(document: &mut Document) {
        let mut tag_tree = TagTree::new();

        let mut page = document.start_page();
        let mut surface = page.surface();
        let id1 = surface.start_tagged(ContentTag::Span("", None, None, None));
        surface.fill_text_(25.0, "a span");
        surface.end_tagged();
        let id2 = surface.start_tagged(ContentTag::Artifact(ArtifactType::Header));
        surface.fill_text_(50.0, "a header artifact");
        surface.end_tagged();
        let id3 = surface.start_tagged(ContentTag::Other);
        surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), Fill::default());
        surface.end_tagged();

        let id4 = surface.start_tagged(ContentTag::Other);
        let tree = sample_svg();
        surface.push_transform(&Transform::from_translate(100.0, 100.0));
        surface.draw_svg(&tree, tree.size(), SvgSettings::default());
        surface.pop();
        surface.end_tagged();

        let id5 = surface.start_tagged(ContentTag::Other);
        let image = load_png_image("rgb8.png");
        surface.push_transform(&Transform::from_translate(100.0, 300.0));
        surface.draw_image(image.clone(), image.size());
        surface.pop();
        surface.end_tagged();

        let id6 = surface.start_tagged(ContentTag::Artifact(ArtifactType::Other));
        surface.fill_text_(75.0, "a different type of artifact");
        surface.end_tagged();

        surface.finish();
        page.finish();

        tag_tree.push(id1);
        tag_tree.push(id2);
        tag_tree.push(id3);
        tag_tree.push(id4);
        tag_tree.push(id5);
        tag_tree.push(id6);

        document.set_tag_tree(tag_tree);
    }

    #[snapshot(document)]
    fn tagging_multiple_pages(document: &mut Document) {
        let mut tag_tree = TagTree::new();
        let mut par_1 = TagGroup::new(Tag::P);
        let mut par_2 = TagGroup::new(Tag::P);
        let mut heading_1 = TagGroup::new(Tag::H1(Some("first heading".to_string())));
        let mut heading_2 = TagGroup::new(Tag::H1(Some("second heading".to_string())));

        let mut page = document.start_page();
        let mut surface = page.surface();
        let h1 = surface.start_tagged(ContentTag::Span("", None, None, None));
        surface.fill_text_(25.0, "a heading");
        surface.end_tagged();
        let p1 = surface.start_tagged(ContentTag::Span("", None, None, None));
        surface.fill_text_(50.0, "a paragraph");
        surface.end_tagged();
        surface.finish();
        page.finish();

        let mut page = document.start_page();
        let mut surface = page.surface();
        let p2 = surface.start_tagged(ContentTag::Span("", None, None, None));
        surface.fill_text_(75.0, "a second paragraph");
        surface.end_tagged();
        surface.finish();
        page.finish();

        let mut page = document.start_page();
        let mut surface = page.surface();
        let h2 = surface.start_tagged(ContentTag::Span("", None, None, None));
        surface.fill_text_(25.0, "another heading");
        surface.end_tagged();
        let p3 = surface.start_tagged(ContentTag::Span("", None, None, None));
        surface.fill_text_(50.0, "another paragraph");
        surface.end_tagged();
        surface.finish();
        page.finish();

        heading_1.push(h1);
        par_1.push(p1);
        par_1.push(p2);

        heading_2.push(h2);
        par_2.push(p3);

        let mut sect1 = TagGroup::new(Tag::Section);
        sect1.push(heading_1);
        sect1.push(par_1);
        let mut sect2 = TagGroup::new(Tag::Section);
        sect2.push(heading_2);
        sect2.push(par_2);

        tag_tree.push(sect1);
        tag_tree.push(sect2);

        document.set_tag_tree(tag_tree);
    }

    #[snapshot(document)]
    fn tagging_two_footnotes(document: &mut Document) {
        let mut tag_tree = TagTree::new();
        let mut fn_group_1 = TagGroup::new(Tag::Note);
        let mut fn_group_2 = TagGroup::new(Tag::Note);

        let mut page = document.start_page();
        let mut surface = page.surface();

        let id1 = surface.start_tagged(ContentTag::Other);
        surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), green_fill(1.0));
        surface.end_tagged();

        let id2 = surface.start_tagged(ContentTag::Other);
        surface.fill_path(&rect_to_path(100.0, 100.0, 150.0, 150.0), green_fill(1.0));
        surface.end_tagged();

        surface.finish();
        page.finish();

        fn_group_1.push(id1);
        fn_group_2.push(id2);
        tag_tree.push(fn_group_1);
        tag_tree.push(fn_group_2);

        document.set_tag_tree(tag_tree);
    }

    #[test]
    fn tagging_page_identifer_appears_twice() {
        let mut document = Document::new();
        let mut tag_tree = TagTree::new();
        let mut fn_group_1 = TagGroup::new(Tag::P);
        let mut fn_group_2 = TagGroup::new(Tag::P);

        let mut page = document.start_page();
        let mut surface = page.surface();

        let id1 = surface.start_tagged(ContentTag::Other);
        surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), green_fill(1.0));
        surface.end_tagged();

        surface.finish();
        page.finish();

        fn_group_1.push(id1);
        fn_group_2.push(id1);
        tag_tree.push(fn_group_1);
        tag_tree.push(fn_group_2);

        document.set_tag_tree(tag_tree);

        assert!(matches!(document.finish(), Err(KrillaError::UserError(_))))
    }

    #[test]
    fn tagging_annotation_identifer_appears_twice() {
        let mut document = Document::new();
        let mut tag_tree = TagTree::new();
        let mut fn_group_1 = TagGroup::new(Tag::P);
        let mut fn_group_2 = TagGroup::new(Tag::P);

        let mut page = document.start_page();
        let link_id = page.add_tagged_annotation(
            LinkAnnotation::new(
                Rect::from_xywh(0.0, 0.0, 100.0, 25.0).unwrap(),
                Target::Action(Action::Link(LinkAction::new("www.youtube.com".to_string()))),
            )
            .into(),
        );
        page.finish();

        fn_group_1.push(link_id);
        fn_group_2.push(link_id);
        tag_tree.push(fn_group_1);
        tag_tree.push(fn_group_2);

        document.set_tag_tree(tag_tree);

        assert!(matches!(document.finish(), Err(KrillaError::UserError(_))))
    }

    #[test]
    fn tagging_missing_identifier_in_tree() {
        let mut document = Document::new();
        let tag_tree = TagTree::new();

        let mut page = document.start_page();
        let mut surface = page.surface();

        let _ = surface.start_tagged(ContentTag::Other);
        surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), green_fill(1.0));
        surface.end_tagged();

        surface.finish();
        page.finish();

        document.set_tag_tree(tag_tree);

        assert!(matches!(document.finish(), Err(KrillaError::UserError(_))))
    }
}
