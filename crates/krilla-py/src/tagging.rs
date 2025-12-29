//! Accessibility and tagging types for krilla Python bindings.

use pyo3::prelude::*;

/// A location identifier for tracking render operations.
///
/// Used to associate render operations with a unique identifier,
/// which helps backtrack validation errors to specific content.
#[pyclass(frozen)]
#[derive(Clone, Copy)]
pub struct Location {
    pub(crate) inner: krilla::surface::Location,
}

#[pymethods]
impl Location {
    /// Create a new location from a positive integer.
    ///
    /// Raises ValueError if the value is zero.
    #[new]
    fn new(value: u64) -> PyResult<Self> {
        std::num::NonZeroU64::new(value)
            .map(|n| Location { inner: n })
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("Location value must be non-zero")
            })
    }

    /// Get the underlying integer value.
    fn get(&self) -> u64 {
        self.inner.get()
    }

    fn __repr__(&self) -> String {
        format!("Location({})", self.inner.get())
    }

    fn __eq__(&self, other: &Location) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        self.inner.get()
    }
}

impl Location {
    pub fn into_inner(self) -> krilla::surface::Location {
        self.inner
    }
}

/// Type of artifact in a PDF document.
///
/// Artifacts represent pieces of content that are not part of the logical
/// structure and should be excluded from accessibility tools.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArtifactType {
    /// The header of a page.
    Header,
    /// The footer of a page.
    Footer,
    /// Page artifacts (e.g., cut marks, color bars).
    Page,
    /// Any other type of artifact (e.g., table strokes).
    Other,
}

impl ArtifactType {
    pub fn into_inner(self) -> krilla::tagging::ArtifactType {
        match self {
            ArtifactType::Header => krilla::tagging::ArtifactType::Header,
            ArtifactType::Footer => krilla::tagging::ArtifactType::Footer,
            ArtifactType::Page => krilla::tagging::ArtifactType::Page,
            ArtifactType::Other => krilla::tagging::ArtifactType::Other,
        }
    }
}

/// A span tag with text properties for accessibility.
///
/// Spans should not be too long - at most a single line of text.
#[pyclass]
#[derive(Clone)]
pub struct SpanTag {
    /// The language of the text (e.g., "en-US").
    #[pyo3(get, set)]
    pub lang: Option<String>,
    /// Alternate text describing the content.
    #[pyo3(get, set)]
    pub alt_text: Option<String>,
    /// Expanded form of an abbreviation.
    #[pyo3(get, set)]
    pub expanded: Option<String>,
    /// The actual text if different from displayed text.
    #[pyo3(get, set)]
    pub actual_text: Option<String>,
}

#[pymethods]
impl SpanTag {
    /// Create a new span tag.
    #[new]
    #[pyo3(signature = (lang=None, alt_text=None, expanded=None, actual_text=None))]
    fn new(
        lang: Option<String>,
        alt_text: Option<String>,
        expanded: Option<String>,
        actual_text: Option<String>,
    ) -> Self {
        SpanTag {
            lang,
            alt_text,
            expanded,
            actual_text,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "SpanTag(lang={:?}, alt_text={:?}, expanded={:?}, actual_text={:?})",
            self.lang, self.alt_text, self.expanded, self.actual_text
        )
    }
}

/// A content tag for accessibility marking.
///
/// Content tags associate content with semantic meaning for accessibility.
#[pyclass]
#[derive(Clone)]
pub enum ContentTag {
    /// An artifact (content not part of logical structure).
    Artifact { artifact_type: ArtifactType },
    /// A text span with properties.
    Span { tag: SpanTag },
    /// Other content that doesn't fit into Span or Artifact.
    Other {},
}

#[pymethods]
impl ContentTag {
    /// Create an artifact content tag.
    #[staticmethod]
    fn artifact(artifact_type: ArtifactType) -> Self {
        ContentTag::Artifact { artifact_type }
    }

    /// Create a span content tag.
    #[staticmethod]
    fn span(tag: SpanTag) -> Self {
        ContentTag::Span { tag }
    }

    /// Create an "other" content tag.
    #[staticmethod]
    fn other() -> Self {
        ContentTag::Other {}
    }

    fn __repr__(&self) -> String {
        match self {
            ContentTag::Artifact { artifact_type } => {
                format!("ContentTag.Artifact({:?})", artifact_type)
            }
            ContentTag::Span { .. } => "ContentTag.Span(...)".to_string(),
            ContentTag::Other {} => "ContentTag.Other".to_string(),
        }
    }
}

impl ContentTag {
    pub fn to_inner(&self) -> krilla::tagging::ContentTag<'_> {
        match self {
            ContentTag::Artifact { artifact_type } => {
                krilla::tagging::ContentTag::Artifact(artifact_type.into_inner())
            }
            ContentTag::Span { tag } => {
                krilla::tagging::ContentTag::Span(krilla::tagging::SpanTag {
                    lang: tag.lang.as_deref(),
                    alt_text: tag.alt_text.as_deref(),
                    expanded: tag.expanded.as_deref(),
                    actual_text: tag.actual_text.as_deref(),
                })
            }
            ContentTag::Other {} => krilla::tagging::ContentTag::Other,
        }
    }
}

/// An identifier returned from start_tagged.
///
/// Used as a leaf node in a tag tree for PDF/UA compliance.
/// This is an opaque handle for tracking tagged content sections.
#[pyclass(frozen)]
#[derive(Clone, PartialEq, Eq)]
pub struct Identifier {
    /// Actual Rust identifier (None for dummy identifiers)
    pub(crate) inner: Option<krilla::tagging::Identifier>,
    /// Python-side ID for equality checks and display
    id: u64,
}

#[pymethods]
impl Identifier {
    fn __repr__(&self) -> String {
        if self.id == 0 {
            "Identifier(dummy)".to_string()
        } else {
            format!("Identifier({})", self.id)
        }
    }

    fn __eq__(&self, other: &Identifier) -> bool {
        self.id == other.id
    }

    fn __hash__(&self) -> u64 {
        self.id
    }

    /// Check if this is a dummy identifier (e.g., for artifacts).
    fn is_dummy(&self) -> bool {
        self.id == 0
    }
}

impl Identifier {
    /// Create a dummy identifier (for artifacts or when tagging is disabled).
    pub fn dummy() -> Self {
        Identifier { inner: None, id: 0 }
    }

    /// Create an identifier from a krilla Identifier, preserving the actual value.
    pub fn from_inner(inner: krilla::tagging::Identifier) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Identifier {
            inner: Some(inner),
            id: COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        }
    }

    /// Convert back to krilla Identifier.
    pub fn into_inner(self) -> krilla::tagging::Identifier {
        self.inner
            .expect("Identifier should have inner value - cannot convert dummy identifier")
    }
}

// ============================================================================
// Core Tag Tree Structure Types
// ============================================================================

/// A tag tree representing the document structure for accessibility.
///
/// The tag tree encodes the logical structure of the PDF document
/// in reading order, enabling accessibility and PDF/UA compliance.
#[pyclass]
pub struct TagTree {
    pub(crate) inner: krilla::tagging::TagTree,
}

#[pymethods]
impl TagTree {
    /// Create a new empty tag tree.
    #[new]
    fn new() -> Self {
        TagTree {
            inner: krilla::tagging::TagTree::new(),
        }
    }

    /// Add a child node to the tag tree.
    ///
    /// The child can be a TagGroup, Identifier, or Node.
    fn push(&mut self, node: Bound<'_, PyAny>) -> PyResult<()> {
        let node: Node = if let Ok(n) = node.extract::<Node>() {
            n
        } else if let Ok(g) = node.extract::<TagGroup>() {
            Node::Group { group: g }
        } else if let Ok(i) = node.extract::<Identifier>() {
            Node::Leaf { identifier: i }
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "Expected Node, TagGroup, or Identifier",
            ));
        };
        self.inner.push(node.into_inner());
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("TagTree(children={})", self.inner.children.len())
    }
}

impl TagTree {
    pub fn into_inner(self) -> krilla::tagging::TagTree {
        self.inner
    }
}

/// A group node in the tag tree with a semantic tag and children.
///
/// Tag groups represent semantic elements like paragraphs, headings,
/// tables, lists, etc. Each group contains a tag that defines its
/// semantic meaning and can have child nodes.
#[pyclass]
#[derive(Clone)]
pub struct TagGroup {
    pub(crate) inner: krilla::tagging::TagGroup,
}

#[pymethods]
impl TagGroup {
    /// Create a new tag group with the specified tag.
    #[new]
    fn new(tag: TagKind) -> Self {
        TagGroup {
            inner: krilla::tagging::TagGroup::new(tag.into_inner()),
        }
    }

    /// Create a new tag group with a tag and children.
    #[staticmethod]
    fn with_children(tag: TagKind, children: Vec<Node>) -> Self {
        let rust_children: Vec<krilla::tagging::Node> =
            children.into_iter().map(|n| n.into_inner()).collect();
        TagGroup {
            inner: krilla::tagging::TagGroup::with_children(tag.into_inner(), rust_children),
        }
    }

    /// Add a child node to this tag group.
    ///
    /// The child can be a TagGroup, Identifier, or Node.
    fn push(&mut self, node: Bound<'_, PyAny>) -> PyResult<()> {
        let node: Node = if let Ok(n) = node.extract::<Node>() {
            n
        } else if let Ok(g) = node.extract::<TagGroup>() {
            Node::Group { group: g }
        } else if let Ok(i) = node.extract::<Identifier>() {
            Node::Leaf { identifier: i }
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "Expected Node, TagGroup, or Identifier",
            ));
        };
        self.inner.push(node.into_inner());
        Ok(())
    }

    /// Get the tag associated with this group.
    #[getter]
    fn tag(&self) -> TagKind {
        TagKind::from_inner(self.inner.tag.clone())
    }

    fn __repr__(&self) -> String {
        format!("TagGroup(children={})", self.inner.children.len())
    }
}

impl TagGroup {
    pub fn into_inner(self) -> krilla::tagging::TagGroup {
        self.inner
    }
}

/// A node in the tag tree - either a group or a leaf identifier.
///
/// Nodes form the tree structure of the tag tree. Group nodes contain
/// other nodes and define semantic structure, while leaf nodes (identifiers)
/// point to actual content on pages.
#[pyclass]
#[derive(Clone)]
pub enum Node {
    /// A group node with a tag and children.
    Group { group: TagGroup },
    /// A leaf node pointing to page content.
    Leaf { identifier: Identifier },
}

#[pymethods]
impl Node {
    /// Create a node from a tag group.
    #[staticmethod]
    fn from_group(group: TagGroup) -> Self {
        Node::Group { group }
    }

    /// Create a node from an identifier.
    #[staticmethod]
    fn from_identifier(identifier: Identifier) -> Self {
        Node::Leaf { identifier }
    }

    /// Check if this is a group node.
    fn is_group(&self) -> bool {
        matches!(self, Node::Group { .. })
    }

    /// Check if this is a leaf node.
    fn is_leaf(&self) -> bool {
        matches!(self, Node::Leaf { .. })
    }

    fn __repr__(&self) -> String {
        match self {
            Node::Group { .. } => "Node.Group(...)".to_string(),
            Node::Leaf { .. } => "Node.Leaf(...)".to_string(),
        }
    }
}

impl Node {
    pub fn into_inner(self) -> krilla::tagging::Node {
        match self {
            Node::Group { group } => krilla::tagging::Node::Group(group.into_inner()),
            Node::Leaf { identifier } => krilla::tagging::Node::Leaf(identifier.into_inner()),
        }
    }
}

/// A semantic tag with attributes for PDF structure.
///
/// TagKind represents a specific semantic element type (paragraph, heading,
/// table, etc.) and can have various attributes attached to customize its
/// properties for accessibility and structure.
#[pyclass]
#[derive(Clone)]
pub struct TagKind {
    pub(crate) inner: krilla::tagging::TagKind,
}

#[pymethods]
impl TagKind {
    fn __repr__(&self) -> String {
        // Simple representation for now
        "TagKind(...)".to_string()
    }

    // ========================================================================
    // Global Attributes (accessible on all tags)
    // ========================================================================

    /// Get the tag ID.
    fn id(&self) -> Option<TagId> {
        self.inner
            .as_any()
            .id()
            .map(|id| TagId::from_inner(id.clone()))
    }

    /// Set the tag ID (builder pattern).
    fn with_id(&mut self, id: Option<TagId>) -> Self {
        let mut cloned = self.inner.clone();
        cloned.as_any_mut().set_id(id.map(|i| i.into_inner()));
        TagKind::from_inner(cloned)
    }

    /// Get the language of this tag.
    fn lang(&self) -> Option<String> {
        self.inner.as_any().lang().map(|s| s.to_string())
    }

    /// Set the language of this tag (builder pattern).
    fn with_lang(&mut self, lang: Option<String>) -> Self {
        let mut cloned = self.inner.clone();
        cloned.as_any_mut().set_lang(lang);
        TagKind::from_inner(cloned)
    }

    /// Get the alternate text.
    fn alt_text(&self) -> Option<String> {
        self.inner.as_any().alt_text().map(|s| s.to_string())
    }

    /// Set the alternate text (builder pattern).
    fn with_alt_text(&mut self, alt_text: Option<String>) -> Self {
        let mut cloned = self.inner.clone();
        cloned.as_any_mut().set_alt_text(alt_text);
        TagKind::from_inner(cloned)
    }

    /// Get the expanded form of an abbreviation.
    fn expanded(&self) -> Option<String> {
        self.inner.as_any().expanded().map(|s| s.to_string())
    }

    /// Set the expanded form (builder pattern).
    fn with_expanded(&mut self, expanded: Option<String>) -> Self {
        let mut cloned = self.inner.clone();
        cloned.as_any_mut().set_expanded(expanded);
        TagKind::from_inner(cloned)
    }

    /// Get the actual text.
    fn actual_text(&self) -> Option<String> {
        self.inner.as_any().actual_text().map(|s| s.to_string())
    }

    /// Set the actual text (builder pattern).
    fn with_actual_text(&mut self, actual_text: Option<String>) -> Self {
        let mut cloned = self.inner.clone();
        cloned.as_any_mut().set_actual_text(actual_text);
        TagKind::from_inner(cloned)
    }

    /// Get the title.
    fn title(&self) -> Option<String> {
        self.inner.as_any().title().map(|s| s.to_string())
    }

    // ========================================================================
    // Struct-Specific Attributes (read-only)
    // ========================================================================

    /// Get the heading level (for Hn tags).
    fn level(&self) -> Option<u16> {
        self.inner.as_any().level().map(|l| l.get())
    }

    /// Get the list numbering (for L tags).
    fn numbering(&self) -> Option<ListNumbering> {
        self.inner
            .as_any()
            .numbering()
            .map(ListNumbering::from_inner)
    }

    /// Get the table summary (for Table tags).
    fn summary(&self) -> Option<String> {
        self.inner.as_any().summary().map(|s| s.to_string())
    }

    /// Get the table header scope (for TH tags).
    fn scope(&self) -> Option<TableHeaderScope> {
        self.inner
            .as_any()
            .scope()
            .map(TableHeaderScope::from_inner)
    }

    // ========================================================================
    // Table Attributes
    // ========================================================================

    /// Get the list of headers associated with a table cell.
    fn headers(&self) -> Option<Vec<TagId>> {
        self.inner.as_any().headers().map(|headers| {
            headers
                .iter()
                .map(|id| TagId::from_inner(id.clone()))
                .collect()
        })
    }

    /// Get the row span of this table cell.
    fn row_span(&self) -> Option<u32> {
        self.inner.as_any().row_span().map(|r| r.get())
    }

    /// Get the column span of this table cell.
    fn col_span(&self) -> Option<u32> {
        self.inner.as_any().col_span().map(|c| c.get())
    }

    // ========================================================================
    // Layout Attributes
    // ========================================================================

    /// Get the placement.
    fn placement(&self) -> Option<Placement> {
        self.inner.as_any().placement().map(Placement::from_inner)
    }

    /// Set the placement (builder pattern).
    fn with_placement(&mut self, placement: Option<Placement>) -> Self {
        let mut cloned = self.inner.clone();
        cloned
            .as_any_mut()
            .set_placement(placement.map(|p| p.into_inner()));
        TagKind::from_inner(cloned)
    }

    /// Get the writing mode.
    fn writing_mode(&self) -> Option<WritingMode> {
        self.inner
            .as_any()
            .writing_mode()
            .map(WritingMode::from_inner)
    }

    /// Set the writing mode (builder pattern).
    fn with_writing_mode(&mut self, writing_mode: Option<WritingMode>) -> Self {
        let mut cloned = self.inner.clone();
        cloned
            .as_any_mut()
            .set_writing_mode(writing_mode.map(|w| w.into_inner()));
        TagKind::from_inner(cloned)
    }

    /// Get the bounding box.
    fn bbox(&self) -> Option<BBox> {
        self.inner.as_any().bbox().map(BBox::from_inner)
    }

    /// Get the width.
    fn width(&self) -> Option<f32> {
        self.inner.as_any().width()
    }

    /// Get the height.
    fn height(&self) -> Option<f32> {
        self.inner.as_any().height()
    }

    /// Get the background color.
    fn background_color(&self) -> Option<NaiveRgbColor> {
        self.inner
            .as_any()
            .background_color()
            .map(NaiveRgbColor::from_inner)
    }

    /// Set the background color (builder pattern).
    fn with_background_color(&mut self, background_color: Option<NaiveRgbColor>) -> Self {
        let mut cloned = self.inner.clone();
        cloned
            .as_any_mut()
            .set_background_color(background_color.map(|c| c.into_inner()));
        TagKind::from_inner(cloned)
    }

    /// Get the text color.
    fn color(&self) -> Option<NaiveRgbColor> {
        self.inner.as_any().color().map(NaiveRgbColor::from_inner)
    }

    /// Set the text color (builder pattern).
    fn with_color(&mut self, color: Option<NaiveRgbColor>) -> Self {
        let mut cloned = self.inner.clone();
        cloned.as_any_mut().set_color(color.map(|c| c.into_inner()));
        TagKind::from_inner(cloned)
    }

    /// Get the padding.
    fn padding(&self) -> Option<SidesF32> {
        self.inner.as_any().padding().map(SidesF32::from_inner)
    }

    /// Set the padding (builder pattern).
    fn with_padding(&mut self, padding: Option<SidesF32>) -> Self {
        let mut cloned = self.inner.clone();
        cloned
            .as_any_mut()
            .set_padding(padding.map(|p| p.into_inner()));
        TagKind::from_inner(cloned)
    }

    /// Get text alignment.
    fn text_align(&self) -> Option<TextAlign> {
        self.inner.as_any().text_align().map(TextAlign::from_inner)
    }

    /// Get block alignment.
    fn block_align(&self) -> Option<BlockAlign> {
        self.inner
            .as_any()
            .block_align()
            .map(BlockAlign::from_inner)
    }

    /// Get inline alignment.
    fn inline_align(&self) -> Option<InlineAlign> {
        self.inner
            .as_any()
            .inline_align()
            .map(InlineAlign::from_inner)
    }
}

impl TagKind {
    pub fn from_inner(inner: krilla::tagging::TagKind) -> Self {
        TagKind { inner }
    }

    pub fn into_inner(self) -> krilla::tagging::TagKind {
        self.inner
    }
}

// ============================================================================
// Supporting Attribute Types
// ============================================================================

/// List numbering style.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ListNumbering {
    /// No numbering.
    None_ = 0,
    /// Solid circular bullets.
    Disc = 1,
    /// Open circular bullets.
    Circle = 2,
    /// Solid square bullets.
    Square = 3,
    /// Decimal numbers (1, 2, 3, ...).
    Decimal = 4,
    /// Lowercase Roman numerals (i, ii, iii, ...).
    LowerRoman = 5,
    /// Uppercase Roman numerals (I, II, III, ...).
    UpperRoman = 6,
    /// Lowercase letters (a, b, c, ...).
    LowerAlpha = 7,
    /// Uppercase letters (A, B, C, ...).
    UpperAlpha = 8,
}

impl ListNumbering {
    pub fn into_inner(self) -> krilla::tagging::ListNumbering {
        match self {
            ListNumbering::None_ => krilla::tagging::ListNumbering::None,
            ListNumbering::Disc => krilla::tagging::ListNumbering::Disc,
            ListNumbering::Circle => krilla::tagging::ListNumbering::Circle,
            ListNumbering::Square => krilla::tagging::ListNumbering::Square,
            ListNumbering::Decimal => krilla::tagging::ListNumbering::Decimal,
            ListNumbering::LowerRoman => krilla::tagging::ListNumbering::LowerRoman,
            ListNumbering::UpperRoman => krilla::tagging::ListNumbering::UpperRoman,
            ListNumbering::LowerAlpha => krilla::tagging::ListNumbering::LowerAlpha,
            ListNumbering::UpperAlpha => krilla::tagging::ListNumbering::UpperAlpha,
        }
    }

    pub fn from_inner(inner: krilla::tagging::ListNumbering) -> Self {
        match inner {
            krilla::tagging::ListNumbering::None => ListNumbering::None_,
            krilla::tagging::ListNumbering::Disc => ListNumbering::Disc,
            krilla::tagging::ListNumbering::Circle => ListNumbering::Circle,
            krilla::tagging::ListNumbering::Square => ListNumbering::Square,
            krilla::tagging::ListNumbering::Decimal => ListNumbering::Decimal,
            krilla::tagging::ListNumbering::LowerRoman => ListNumbering::LowerRoman,
            krilla::tagging::ListNumbering::UpperRoman => ListNumbering::UpperRoman,
            krilla::tagging::ListNumbering::LowerAlpha => ListNumbering::LowerAlpha,
            krilla::tagging::ListNumbering::UpperAlpha => ListNumbering::UpperAlpha,
        }
    }
}

/// Table header cell scope.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TableHeaderScope {
    /// The header cell refers to the row.
    Row = 0,
    /// The header cell refers to the column.
    Column = 1,
    /// The header cell refers to both row and column.
    Both = 2,
}

impl TableHeaderScope {
    pub fn into_inner(self) -> krilla::tagging::TableHeaderScope {
        match self {
            TableHeaderScope::Row => krilla::tagging::TableHeaderScope::Row,
            TableHeaderScope::Column => krilla::tagging::TableHeaderScope::Column,
            TableHeaderScope::Both => krilla::tagging::TableHeaderScope::Both,
        }
    }

    pub fn from_inner(inner: krilla::tagging::TableHeaderScope) -> Self {
        match inner {
            krilla::tagging::TableHeaderScope::Row => TableHeaderScope::Row,
            krilla::tagging::TableHeaderScope::Column => TableHeaderScope::Column,
            krilla::tagging::TableHeaderScope::Both => TableHeaderScope::Both,
        }
    }
}

// ============================================================================
// Tag Factory with 33 Variants
// ============================================================================

/// Factory for creating semantic tags.
///
/// Use the static methods on this class to create different types of tags
/// (paragraphs, headings, tables, lists, etc.) for building tagged PDF documents.
#[pyclass]
pub struct Tag;

// Our tags are not snake case, so that they will look like class names in Python:
// Tag.Part()
#[allow(non_snake_case)]
#[pymethods]
impl Tag {
    // Simple tags (no required parameters) - Part 1/2

    /// Create a Part tag (top-level division containing articles/sections).
    #[staticmethod]
    fn Part() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Part>::Part.into())
    }

    /// Create an Article tag (self-contained composition).
    #[staticmethod]
    fn Article() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Article>::Article.into())
    }

    /// Create a Section tag (generic container for grouping content).
    #[staticmethod]
    fn Section() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Section>::Section.into())
    }

    /// Create a Div tag (generic block-level grouping element).
    #[staticmethod]
    fn Div() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Div>::Div.into())
    }

    /// Create a BlockQuote tag (paragraph-level quote).
    #[staticmethod]
    fn BlockQuote() -> TagKind {
        TagKind::from_inner(
            krilla::tagging::Tag::<krilla::tagging::kind::BlockQuote>::BlockQuote.into(),
        )
    }

    /// Create a Caption tag (caption for a figure, table, etc.).
    ///
    /// Best practice: Should appear as a sibling after the content it describes.
    #[staticmethod]
    fn Caption() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Caption>::Caption.into())
    }

    /// Create a TOC tag (table of contents).
    ///
    /// Best practice: Should consist of TOCI items or nested TOCs.
    #[staticmethod]
    fn TOC() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::TOC>::TOC.into())
    }

    /// Create a TOCI tag (item in table of contents).
    ///
    /// Best practice: Should only appear within a TOC.
    #[staticmethod]
    fn TOCI() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::TOCI>::TOCI.into())
    }

    /// Create an Index tag (index of key terms).
    #[staticmethod]
    fn Index() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Index>::Index.into())
    }

    /// Create a P tag (paragraph).
    #[staticmethod]
    fn P() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::P>::P.into())
    }

    // Simple tags (no required parameters) - Part 2/2

    /// Create an LI tag (list item).
    ///
    /// Best practice: Should consist of Lbl and/or LBody elements.
    #[staticmethod]
    fn LI() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::LI>::LI.into())
    }

    /// Create an Lbl tag (label for a list item).
    #[staticmethod]
    fn Lbl() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Lbl>::Lbl.into())
    }

    /// Create an LBody tag (body/description of a list item).
    #[staticmethod]
    fn LBody() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::LBody>::LBody.into())
    }

    /// Create a TR tag (table row).
    #[staticmethod]
    fn TR() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::TR>::TR.into())
    }

    /// Create a TD tag (table data cell).
    #[staticmethod]
    fn TD() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::TD>::TD.into())
    }

    /// Create a THead tag (table header row group).
    #[staticmethod]
    fn THead() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::THead>::THead.into())
    }

    /// Create a TBody tag (table body row group).
    #[staticmethod]
    fn TBody() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::TBody>::TBody.into())
    }

    /// Create a TFoot tag (table footer row group).
    #[staticmethod]
    fn TFoot() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::TFoot>::TFoot.into())
    }

    /// Create a Span tag (inline-level element with no specific meaning).
    #[staticmethod]
    fn Span() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Span>::Span.into())
    }

    /// Create an InlineQuote tag (inline quotation).
    #[staticmethod]
    fn InlineQuote() -> TagKind {
        TagKind::from_inner(
            krilla::tagging::Tag::<krilla::tagging::kind::InlineQuote>::InlineQuote.into(),
        )
    }

    /// Create a Note tag (foot- or endnote).
    #[staticmethod]
    fn Note() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Note>::Note.into())
    }

    /// Create a Reference tag (reference to elsewhere in document).
    #[staticmethod]
    fn Reference() -> TagKind {
        TagKind::from_inner(
            krilla::tagging::Tag::<krilla::tagging::kind::Reference>::Reference.into(),
        )
    }

    /// Create a BibEntry tag (bibliographic entry).
    #[staticmethod]
    fn BibEntry() -> TagKind {
        TagKind::from_inner(
            krilla::tagging::Tag::<krilla::tagging::kind::BibEntry>::BibEntry.into(),
        )
    }

    /// Create a Code tag (computer code).
    #[staticmethod]
    fn Code() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Code>::Code.into())
    }

    /// Create a Link tag (hyperlink).
    ///
    /// Best practice: First child should be a link annotation, second should be associated content.
    #[staticmethod]
    fn Link() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Link>::Link.into())
    }

    /// Create an Annot tag (annotation association).
    ///
    /// Best practice: Use for all annotations except links and widgets.
    #[staticmethod]
    fn Annot() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Annot>::Annot.into())
    }

    /// Create a NonStruct tag (non-structural grouping element).
    #[staticmethod]
    fn NonStruct() -> TagKind {
        TagKind::from_inner(
            krilla::tagging::Tag::<krilla::tagging::kind::NonStruct>::NonStruct.into(),
        )
    }

    /// Create a Datetime tag (date or time).
    #[staticmethod]
    fn Datetime() -> TagKind {
        TagKind::from_inner(
            krilla::tagging::Tag::<krilla::tagging::kind::Datetime>::Datetime.into(),
        )
    }

    /// Create a Terms tag (list of terms).
    #[staticmethod]
    fn Terms() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Terms>::Terms.into())
    }

    /// Create a Title tag (title of a document or section).
    #[staticmethod]
    fn Title() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Title>::Title.into())
    }

    /// Create a Strong tag (strong importance, typically bold).
    #[staticmethod]
    fn Strong() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Strong>::Strong.into())
    }

    /// Create an Em tag (emphasized text, typically italic).
    #[staticmethod]
    fn Em() -> TagKind {
        TagKind::from_inner(krilla::tagging::Tag::<krilla::tagging::kind::Em>::Em.into())
    }

    // Tags with REQUIRED parameters

    /// Create an Hn tag (heading with level 1-6).
    ///
    /// # Arguments
    /// * `level` - Heading level (1-6), where 1 is highest level
    ///
    /// # Raises
    /// ValueError if level is 0
    #[staticmethod]
    fn Hn(level: u16) -> PyResult<TagKind> {
        let level_nz = std::num::NonZeroU16::new(level).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("Heading level must be non-zero")
        })?;
        Ok(TagKind::from_inner(
            krilla::tagging::Tag::<krilla::tagging::kind::Hn>::Hn(level_nz, None).into(),
        ))
    }

    /// Create an L tag (list with numbering style).
    ///
    /// # Arguments
    /// * `numbering` - The list numbering style (None, Disc, Circle, Square, Decimal, etc.)
    ///
    /// Best practice: Should consist of an optional caption followed by list items.
    #[staticmethod]
    fn L(numbering: ListNumbering) -> TagKind {
        TagKind::from_inner(
            krilla::tagging::Tag::<krilla::tagging::kind::L>::L(numbering.into_inner()).into(),
        )
    }

    /// Create a TH tag (table header cell with scope).
    ///
    /// # Arguments
    /// * `scope` - The header scope (Row, Column, or Both)
    #[staticmethod]
    fn TH(scope: TableHeaderScope) -> TagKind {
        TagKind::from_inner(
            krilla::tagging::Tag::<krilla::tagging::kind::TH>::TH(scope.into_inner()).into(),
        )
    }

    // Tags with OPTIONAL parameters

    /// Create a Table tag with optional summary.
    ///
    /// # Arguments
    /// * `summary` - Optional description of table purpose and structure
    ///
    /// Best practice: Should consist of optional header, one or more body elements, optional footer.
    #[staticmethod]
    #[pyo3(signature = (summary=None))]
    fn Table(summary: Option<String>) -> TagKind {
        // Table has a const, but we need to set summary. Use with_summary if available,
        // or just use the const and set the attribute later (Phase 4).
        // For now, just return the const - summary will be settable via .with_summary() in Phase 4
        let mut tag = krilla::tagging::Tag::<krilla::tagging::kind::Table>::Table;
        tag.set_summary(summary);
        TagKind::from_inner(tag.into())
    }

    /// Create a Figure tag with optional alt text.
    ///
    /// # Arguments
    /// * `alt_text` - Alternate text describing the figure (required for PDF/UA-1)
    #[staticmethod]
    #[pyo3(signature = (alt_text=None))]
    fn Figure(alt_text: Option<String>) -> TagKind {
        TagKind::from_inner(
            krilla::tagging::Tag::<krilla::tagging::kind::Figure>::Figure(alt_text).into(),
        )
    }

    /// Create a Formula tag with optional alt text.
    ///
    /// # Arguments
    /// * `alt_text` - Alternate text describing the formula (required for PDF/UA-1)
    #[staticmethod]
    #[pyo3(signature = (alt_text=None))]
    fn Formula(alt_text: Option<String>) -> TagKind {
        TagKind::from_inner(
            krilla::tagging::Tag::<krilla::tagging::kind::Formula>::Formula(alt_text).into(),
        )
    }
}

// ============================================================================
// Attribute Enums and Complex Types
// ============================================================================

/// The positioning of the element with respect to the enclosing reference area
/// and other content.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Placement {
    Block = 0,
    Inline = 1,
    Before = 2,
    Start = 3,
    End = 4,
}

impl Placement {
    pub fn into_inner(self) -> krilla::tagging::Placement {
        match self {
            Placement::Block => krilla::tagging::Placement::Block,
            Placement::Inline => krilla::tagging::Placement::Inline,
            Placement::Before => krilla::tagging::Placement::Before,
            Placement::Start => krilla::tagging::Placement::Start,
            Placement::End => krilla::tagging::Placement::End,
        }
    }

    pub fn from_inner(inner: krilla::tagging::Placement) -> Self {
        match inner {
            krilla::tagging::Placement::Block => Placement::Block,
            krilla::tagging::Placement::Inline => Placement::Inline,
            krilla::tagging::Placement::Before => Placement::Before,
            krilla::tagging::Placement::Start => Placement::Start,
            krilla::tagging::Placement::End => Placement::End,
        }
    }
}

/// The directions of layout progression for packing of ILSEs (inline progression)
/// and stacking of BLSEs (block progression).
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WritingMode {
    LrTb = 0,
    RlTb = 1,
    TbRl = 2,
}

impl WritingMode {
    pub fn into_inner(self) -> krilla::tagging::WritingMode {
        match self {
            WritingMode::LrTb => krilla::tagging::WritingMode::LrTb,
            WritingMode::RlTb => krilla::tagging::WritingMode::RlTb,
            WritingMode::TbRl => krilla::tagging::WritingMode::TbRl,
        }
    }

    pub fn from_inner(inner: krilla::tagging::WritingMode) -> Self {
        match inner {
            krilla::tagging::WritingMode::LrTb => WritingMode::LrTb,
            krilla::tagging::WritingMode::RlTb => WritingMode::RlTb,
            krilla::tagging::WritingMode::TbRl => WritingMode::TbRl,
        }
    }
}

/// The border style of an element.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BorderStyle {
    None = 0,
    Hidden = 1,
    Solid = 2,
    Dashed = 3,
    Dotted = 4,
    Double = 5,
    Groove = 6,
    Ridge = 7,
    Inset = 8,
    Outset = 9,
}

impl BorderStyle {
    #[allow(dead_code)]
    pub fn into_inner(self) -> krilla::tagging::BorderStyle {
        match self {
            BorderStyle::None => krilla::tagging::BorderStyle::None,
            BorderStyle::Hidden => krilla::tagging::BorderStyle::Hidden,
            BorderStyle::Solid => krilla::tagging::BorderStyle::Solid,
            BorderStyle::Dashed => krilla::tagging::BorderStyle::Dashed,
            BorderStyle::Dotted => krilla::tagging::BorderStyle::Dotted,
            BorderStyle::Double => krilla::tagging::BorderStyle::Double,
            BorderStyle::Groove => krilla::tagging::BorderStyle::Groove,
            BorderStyle::Ridge => krilla::tagging::BorderStyle::Ridge,
            BorderStyle::Inset => krilla::tagging::BorderStyle::Inset,
            BorderStyle::Outset => krilla::tagging::BorderStyle::Outset,
        }
    }

    #[allow(dead_code)]
    pub fn from_inner(inner: krilla::tagging::BorderStyle) -> Self {
        match inner {
            krilla::tagging::BorderStyle::None => BorderStyle::None,
            krilla::tagging::BorderStyle::Hidden => BorderStyle::Hidden,
            krilla::tagging::BorderStyle::Solid => BorderStyle::Solid,
            krilla::tagging::BorderStyle::Dashed => BorderStyle::Dashed,
            krilla::tagging::BorderStyle::Dotted => BorderStyle::Dotted,
            krilla::tagging::BorderStyle::Double => BorderStyle::Double,
            krilla::tagging::BorderStyle::Groove => BorderStyle::Groove,
            krilla::tagging::BorderStyle::Ridge => BorderStyle::Ridge,
            krilla::tagging::BorderStyle::Inset => BorderStyle::Inset,
            krilla::tagging::BorderStyle::Outset => BorderStyle::Outset,
        }
    }
}

/// The text alignment.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextAlign {
    Start = 0,
    Center = 1,
    End = 2,
    Justify = 3,
}

impl TextAlign {
    #[allow(dead_code)]
    pub fn into_inner(self) -> krilla::tagging::TextAlign {
        match self {
            TextAlign::Start => krilla::tagging::TextAlign::Start,
            TextAlign::Center => krilla::tagging::TextAlign::Center,
            TextAlign::End => krilla::tagging::TextAlign::End,
            TextAlign::Justify => krilla::tagging::TextAlign::Justify,
        }
    }

    #[allow(dead_code)]
    pub fn from_inner(inner: krilla::tagging::TextAlign) -> Self {
        match inner {
            krilla::tagging::TextAlign::Start => TextAlign::Start,
            krilla::tagging::TextAlign::Center => TextAlign::Center,
            krilla::tagging::TextAlign::End => TextAlign::End,
            krilla::tagging::TextAlign::Justify => TextAlign::Justify,
        }
    }
}

/// The block alignment.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockAlign {
    Begin = 0,
    Middle = 1,
    After = 2,
    Justify = 3,
}

impl BlockAlign {
    #[allow(dead_code)]
    pub fn into_inner(self) -> krilla::tagging::BlockAlign {
        match self {
            BlockAlign::Begin => krilla::tagging::BlockAlign::Begin,
            BlockAlign::Middle => krilla::tagging::BlockAlign::Middle,
            BlockAlign::After => krilla::tagging::BlockAlign::After,
            BlockAlign::Justify => krilla::tagging::BlockAlign::Justify,
        }
    }

    pub fn from_inner(inner: krilla::tagging::BlockAlign) -> Self {
        match inner {
            krilla::tagging::BlockAlign::Begin => BlockAlign::Begin,
            krilla::tagging::BlockAlign::Middle => BlockAlign::Middle,
            krilla::tagging::BlockAlign::After => BlockAlign::After,
            krilla::tagging::BlockAlign::Justify => BlockAlign::Justify,
        }
    }
}

/// The inline alignment.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InlineAlign {
    Start = 0,
    Center = 1,
    End = 2,
}

impl InlineAlign {
    #[allow(dead_code)]
    pub fn into_inner(self) -> krilla::tagging::InlineAlign {
        match self {
            InlineAlign::Start => krilla::tagging::InlineAlign::Start,
            InlineAlign::Center => krilla::tagging::InlineAlign::Center,
            InlineAlign::End => krilla::tagging::InlineAlign::End,
        }
    }

    pub fn from_inner(inner: krilla::tagging::InlineAlign) -> Self {
        match inner {
            krilla::tagging::InlineAlign::Start => InlineAlign::Start,
            krilla::tagging::InlineAlign::Center => InlineAlign::Center,
            krilla::tagging::InlineAlign::End => InlineAlign::End,
        }
    }
}

/// The text decoration type (over- and underlines).
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextDecorationType {
    None = 0,
    Underline = 1,
    Overline = 2,
    LineThrough = 3,
}

impl TextDecorationType {
    #[allow(dead_code)]
    pub fn into_inner(self) -> krilla::tagging::TextDecorationType {
        match self {
            TextDecorationType::None => krilla::tagging::TextDecorationType::None,
            TextDecorationType::Underline => krilla::tagging::TextDecorationType::Underline,
            TextDecorationType::Overline => krilla::tagging::TextDecorationType::Overline,
            TextDecorationType::LineThrough => krilla::tagging::TextDecorationType::LineThrough,
        }
    }

    #[allow(dead_code)]
    pub fn from_inner(inner: krilla::tagging::TextDecorationType) -> Self {
        match inner {
            krilla::tagging::TextDecorationType::None => TextDecorationType::None,
            krilla::tagging::TextDecorationType::Underline => TextDecorationType::Underline,
            krilla::tagging::TextDecorationType::Overline => TextDecorationType::Overline,
            krilla::tagging::TextDecorationType::LineThrough => TextDecorationType::LineThrough,
        }
    }
}

/// The rotation of glyphs in vertical writing modes.
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GlyphOrientationVertical {
    Auto = 0,
    None = 1,
    Clockwise90 = 2,
    CounterClockwise90 = 3,
    Clockwise180 = 4,
    CounterClockwise180 = 5,
    Clockwise270 = 6,
}

impl GlyphOrientationVertical {
    #[allow(dead_code)]
    pub fn into_inner(self) -> krilla::tagging::GlyphOrientationVertical {
        match self {
            GlyphOrientationVertical::Auto => krilla::tagging::GlyphOrientationVertical::Auto,
            GlyphOrientationVertical::None => krilla::tagging::GlyphOrientationVertical::None,
            GlyphOrientationVertical::Clockwise90 => {
                krilla::tagging::GlyphOrientationVertical::Clockwise90
            }
            GlyphOrientationVertical::CounterClockwise90 => {
                krilla::tagging::GlyphOrientationVertical::CounterClockwise90
            }
            GlyphOrientationVertical::Clockwise180 => {
                krilla::tagging::GlyphOrientationVertical::Clockwise180
            }
            GlyphOrientationVertical::CounterClockwise180 => {
                krilla::tagging::GlyphOrientationVertical::CounterClockwise180
            }
            GlyphOrientationVertical::Clockwise270 => {
                krilla::tagging::GlyphOrientationVertical::Clockwise270
            }
        }
    }

    #[allow(dead_code)]
    pub fn from_inner(inner: krilla::tagging::GlyphOrientationVertical) -> Self {
        match inner {
            krilla::tagging::GlyphOrientationVertical::Auto => GlyphOrientationVertical::Auto,
            krilla::tagging::GlyphOrientationVertical::None => GlyphOrientationVertical::None,
            krilla::tagging::GlyphOrientationVertical::Clockwise90 => {
                GlyphOrientationVertical::Clockwise90
            }
            krilla::tagging::GlyphOrientationVertical::CounterClockwise90 => {
                GlyphOrientationVertical::CounterClockwise90
            }
            krilla::tagging::GlyphOrientationVertical::Clockwise180 => {
                GlyphOrientationVertical::Clockwise180
            }
            krilla::tagging::GlyphOrientationVertical::CounterClockwise180 => {
                GlyphOrientationVertical::CounterClockwise180
            }
            krilla::tagging::GlyphOrientationVertical::Clockwise270 => {
                GlyphOrientationVertical::Clockwise270
            }
        }
    }
}

/// The height of a line.
#[pyclass]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LineHeight {
    Normal(),
    Auto(),
    Custom { value: f32 },
}

#[pymethods]
impl LineHeight {
    #[staticmethod]
    fn normal() -> Self {
        LineHeight::Normal()
    }

    #[staticmethod]
    fn auto() -> Self {
        LineHeight::Auto()
    }

    #[staticmethod]
    fn custom(value: f32) -> Self {
        LineHeight::Custom { value }
    }
}

impl LineHeight {
    pub fn into_inner(self) -> krilla::tagging::LineHeight {
        match self {
            LineHeight::Normal() => krilla::tagging::LineHeight::Normal,
            LineHeight::Auto() => krilla::tagging::LineHeight::Auto,
            LineHeight::Custom { value } => krilla::tagging::LineHeight::Custom(value),
        }
    }

    pub fn from_inner(inner: krilla::tagging::LineHeight) -> Self {
        match inner {
            krilla::tagging::LineHeight::Normal => LineHeight::Normal(),
            krilla::tagging::LineHeight::Auto => LineHeight::Auto(),
            krilla::tagging::LineHeight::Custom(value) => LineHeight::Custom { value },
        }
    }
}

// ============================================================================
// Complex Attribute Types
// ============================================================================

/// A unique identifier for tags in the tag tree.
#[pyclass]
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TagId {
    pub(crate) inner: krilla::tagging::TagId,
}

#[pymethods]
impl TagId {
    /// Create a new TagId from bytes.
    #[new]
    fn new(bytes: Vec<u8>) -> Self {
        TagId {
            inner: krilla::tagging::TagId::from(bytes),
        }
    }

    /// Create a TagId from a string.
    #[staticmethod]
    fn from_str(s: &str) -> Self {
        TagId {
            inner: krilla::tagging::TagId::from(s.as_bytes().to_vec()),
        }
    }

    fn __eq__(&self, other: &TagId) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }

    fn __repr__(&self) -> String {
        format!("TagId({:?})", self.inner.as_bytes())
    }
}

impl TagId {
    pub fn into_inner(self) -> krilla::tagging::TagId {
        self.inner
    }

    pub fn from_inner(inner: krilla::tagging::TagId) -> Self {
        TagId { inner }
    }
}

/// Bounding box for tag content.
#[pyclass]
#[derive(Clone, Copy)]
pub struct BBox {
    /// The page index of the bounding box.
    #[pyo3(get, set)]
    pub page_idx: usize,
    /// The rectangle that encloses the content.
    #[pyo3(get, set)]
    pub rect: crate::geometry::Rect,
}

#[pymethods]
impl BBox {
    #[new]
    fn new(page_idx: usize, rect: crate::geometry::Rect) -> Self {
        BBox { page_idx, rect }
    }

    fn __repr__(&self) -> String {
        format!("BBox(page_idx={}, rect=...)", self.page_idx)
    }
}

impl BBox {
    pub fn into_inner(self) -> krilla::tagging::BBox {
        krilla::tagging::BBox::new(self.page_idx, self.rect.into_inner())
    }

    pub fn from_inner(inner: krilla::tagging::BBox) -> Self {
        BBox {
            page_idx: inner.page_idx,
            rect: crate::geometry::Rect::from_inner(inner.rect),
        }
    }
}

/// RGB color (8-bit per channel) for tag attributes.
#[pyclass]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct NaiveRgbColor {
    /// Red component (0-255).
    #[pyo3(get, set)]
    pub red: u8,
    /// Green component (0-255).
    #[pyo3(get, set)]
    pub green: u8,
    /// Blue component (0-255).
    #[pyo3(get, set)]
    pub blue: u8,
}

#[pymethods]
impl NaiveRgbColor {
    #[new]
    fn new(red: u8, green: u8, blue: u8) -> Self {
        NaiveRgbColor { red, green, blue }
    }

    /// Create from normalized float values (0.0-1.0).
    #[staticmethod]
    fn new_f32(red: f32, green: f32, blue: f32) -> PyResult<Self> {
        if !(0.0..=1.0).contains(&red)
            || !(0.0..=1.0).contains(&green)
            || !(0.0..=1.0).contains(&blue)
        {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "RGB color components must be in the range [0.0, 1.0]",
            ));
        }
        Ok(NaiveRgbColor {
            red: (255.0 * red).round() as u8,
            green: (255.0 * green).round() as u8,
            blue: (255.0 * blue).round() as u8,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "NaiveRgbColor(red={}, green={}, blue={})",
            self.red, self.green, self.blue
        )
    }
}

impl NaiveRgbColor {
    pub fn into_inner(self) -> krilla::tagging::NaiveRgbColor {
        krilla::tagging::NaiveRgbColor::new(self.red, self.green, self.blue)
    }

    pub fn from_inner(inner: krilla::tagging::NaiveRgbColor) -> Self {
        NaiveRgbColor {
            red: inner.red,
            green: inner.green,
            blue: inner.blue,
        }
    }
}

/// Four-sided value for padding, borders, etc.
#[pyclass]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SidesF32 {
    /// The start of the element on the block axis.
    #[pyo3(get, set)]
    pub before: f32,
    /// The end of the element on the block axis.
    #[pyo3(get, set)]
    pub after: f32,
    /// The start of the element on the inline axis.
    #[pyo3(get, set)]
    pub start: f32,
    /// The end of the element on the inline axis.
    #[pyo3(get, set)]
    pub end: f32,
}

#[pymethods]
impl SidesF32 {
    /// Create with specific values for each side.
    #[new]
    fn new(before: f32, after: f32, start: f32, end: f32) -> Self {
        SidesF32 {
            before,
            after,
            start,
            end,
        }
    }

    /// Create with the same value for all sides.
    #[staticmethod]
    fn uniform(value: f32) -> Self {
        SidesF32 {
            before: value,
            after: value,
            start: value,
            end: value,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "SidesF32(before={}, after={}, start={}, end={})",
            self.before, self.after, self.start, self.end
        )
    }
}

impl SidesF32 {
    pub fn into_inner(self) -> krilla::tagging::Sides<f32> {
        krilla::tagging::Sides::new(self.before, self.after, self.start, self.end)
    }

    pub fn from_inner(inner: krilla::tagging::Sides<f32>) -> Self {
        SidesF32 {
            before: inner.before,
            after: inner.after,
            start: inner.start,
            end: inner.end,
        }
    }
}

/// Column widths - either uniform or specific per column.
#[pyclass]
#[derive(Clone, PartialEq, Debug)]
pub enum ColumnDimensions {
    All { value: f32 },
    Specific { values: Vec<f32> },
}

#[pymethods]
impl ColumnDimensions {
    /// Create with the same value for all columns.
    #[staticmethod]
    fn all(value: f32) -> Self {
        ColumnDimensions::All { value }
    }

    /// Create with specific values for each column.
    #[staticmethod]
    fn specific(values: Vec<f32>) -> Self {
        ColumnDimensions::Specific { values }
    }
}

impl ColumnDimensions {
    pub fn into_inner(self) -> krilla::tagging::ColumnDimensions {
        match self {
            ColumnDimensions::All { value } => krilla::tagging::ColumnDimensions::All(value),
            ColumnDimensions::Specific { values } => {
                krilla::tagging::ColumnDimensions::Specific(values)
            }
        }
    }

    pub fn from_inner(inner: krilla::tagging::ColumnDimensions) -> Self {
        match inner {
            krilla::tagging::ColumnDimensions::All(value) => ColumnDimensions::All { value },
            krilla::tagging::ColumnDimensions::Specific(values) => {
                ColumnDimensions::Specific { values }
            }
        }
    }
}
