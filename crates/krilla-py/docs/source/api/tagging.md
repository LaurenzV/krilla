# Tagging & Accessibility

Types for creating tagged PDF (PDF/UA accessibility support).

## Overview

Tagged PDFs preserve semantic structure and enable accessibility features. Without tagging, PDF documents are just low-level drawing instructions. Tags encode the logical structure (headings, paragraphs, tables, lists, etc.) and reading order, making PDFs accessible to screen readers and other assistive technologies.

## Basic Tagging Types

### Location

```{eval-rst}
.. autoclass:: krilla.Location
   :members:
   :undoc-members:
   :show-inheritance:
```

Represents a location in the logical document structure for tagged PDF.

### ArtifactType

```{eval-rst}
.. autoclass:: krilla.ArtifactType
   :members:
   :undoc-members:
   :show-inheritance:
```

Types of artifacts (decorative elements) in tagged PDF.

### ContentTag

```{eval-rst}
.. autoclass:: krilla.ContentTag
   :members:
   :undoc-members:
   :show-inheritance:
```

Tags for document content elements.

### SpanTag

```{eval-rst}
.. autoclass:: krilla.SpanTag
   :members:
   :undoc-members:
   :show-inheritance:
```

Tags for inline text spans and formatting.

### Identifier

```{eval-rst}
.. autoclass:: krilla.Identifier
   :members:
   :undoc-members:
   :show-inheritance:
```

Unique identifier for structure elements in tagged PDF.

## Tag Tree Core

### TagTree

```{eval-rst}
.. autoclass:: krilla.TagTree
   :members:
   :undoc-members:
   :show-inheritance:
```

Root container for the PDF tag tree (accessibility structure).

### TagGroup

```{eval-rst}
.. autoclass:: krilla.TagGroup
   :members:
   :undoc-members:
   :show-inheritance:
```

Group node in tag tree with semantic tag and children.

### Node

```{eval-rst}
.. autoclass:: krilla.Node
   :members:
   :undoc-members:
   :show-inheritance:
```

Tag tree node - either a group or leaf identifier.

## Creating Tags

### Tag Factory

```{eval-rst}
.. autoclass:: krilla.Tag
   :members:
   :undoc-members:
   :show-inheritance:
```

Factory class for creating semantic tags. Provides 33 tag type variants.

### TagKind

```{eval-rst}
.. autoclass:: krilla.TagKind
   :members:
   :undoc-members:
   :show-inheritance:
```

Semantic tag with attributes for PDF structure.

## Tag Attributes

### TagId

```{eval-rst}
.. autoclass:: krilla.TagId
   :members:
   :undoc-members:
   :show-inheritance:
```

Unique identifier for tags in the tag tree.

### BBox

```{eval-rst}
.. autoclass:: krilla.BBox
   :members:
   :undoc-members:
   :show-inheritance:
```

Bounding box for tag content.

### NaiveRgbColor

```{eval-rst}
.. autoclass:: krilla.NaiveRgbColor
   :members:
   :undoc-members:
   :show-inheritance:
```

RGB color (8-bit per channel).

### SidesF32

```{eval-rst}
.. autoclass:: krilla.SidesF32
   :members:
   :undoc-members:
   :show-inheritance:
```

Four-sided values for padding, spacing, etc.

### ColumnDimensions

```{eval-rst}
.. autoclass:: krilla.ColumnDimensions
   :members:
   :undoc-members:
   :show-inheritance:
```

Column width specifications for tables.

### LineHeight

```{eval-rst}
.. autoclass:: krilla.LineHeight
   :members:
   :undoc-members:
   :show-inheritance:
```

Line height specification.

## Tag Attribute Enumerations

### ListNumbering

```{eval-rst}
.. autoclass:: krilla.ListNumbering
   :members:
   :undoc-members:
   :show-inheritance:
```

List numbering style.

### TableHeaderScope

```{eval-rst}
.. autoclass:: krilla.TableHeaderScope
   :members:
   :undoc-members:
   :show-inheritance:
```

Table header cell scope.

### Placement

```{eval-rst}
.. autoclass:: krilla.Placement
   :members:
   :undoc-members:
   :show-inheritance:
```

Element placement type.

### WritingMode

```{eval-rst}
.. autoclass:: krilla.WritingMode
   :members:
   :undoc-members:
   :show-inheritance:
```

Text writing direction.

### BorderStyle

```{eval-rst}
.. autoclass:: krilla.BorderStyle
   :members:
   :undoc-members:
   :show-inheritance:
```

Border rendering style.

### TextAlign

```{eval-rst}
.. autoclass:: krilla.TextAlign
   :members:
   :undoc-members:
   :show-inheritance:
```

Text alignment within block.

### BlockAlign

```{eval-rst}
.. autoclass:: krilla.BlockAlign
   :members:
   :undoc-members:
   :show-inheritance:
```

Block-level alignment.

### InlineAlign

```{eval-rst}
.. autoclass:: krilla.InlineAlign
   :members:
   :undoc-members:
   :show-inheritance:
```

Inline element alignment.

### TextDecorationType

```{eval-rst}
.. autoclass:: krilla.TextDecorationType
   :members:
   :undoc-members:
   :show-inheritance:
```

Text decoration style.

### GlyphOrientationVertical

```{eval-rst}
.. autoclass:: krilla.GlyphOrientationVertical
   :members:
   :undoc-members:
   :show-inheritance:
```

Glyph orientation in vertical text.

## Document Methods

The {py:class}`krilla.Document` class provides this tagging method:

- {py:meth}`~krilla.Document.set_tag_tree` - Set the tag tree for PDF/UA accessibility

The {py:class}`krilla.Surface` class provides these tagging methods:

- {py:meth}`~krilla.Surface.start_tagged` - Begin a tagged content section
- {py:meth}`~krilla.Surface.end_tagged` - End the current tagged section

## Tag Types

The Tag factory class provides 33 semantic tag types:

**Structure Tags:**
- `Part()` - Top-level document part
- `Article()` - Self-contained composition
- `Section()` - Generic section
- `Div()` - Generic block-level division

**Grouping Tags:**
- `BlockQuote()` - Block quotation
- `Caption()` - Caption for figure/table
- `TOC()` - Table of contents
- `TOCI()` - Table of contents item
- `Index()` - Index section

**Paragraph Tags:**
- `P()` - Paragraph
- `Hn(level)` - Heading with level 1-6

**List Tags:**
- `L(numbering)` - List with numbering style
- `LI()` - List item
- `Lbl()` - List item label (bullet/number)
- `LBody()` - List item body

**Table Tags:**
- `Table(summary=None)` - Table with optional summary
- `TR()` - Table row
- `TH(scope)` - Table header cell with scope
- `TD()` - Table data cell
- `THead()` - Table header section
- `TBody()` - Table body section
- `TFoot()` - Table footer section

**Inline Tags:**
- `Span()` - Generic inline element
- `InlineQuote()` - Inline quotation
- `Note()` - Footnote or endnote
- `Reference()` - Reference to external resource
- `BibEntry()` - Bibliography entry
- `Code()` - Code fragment
- `Link()` - Hyperlink
- `Annot()` - Annotation reference

**Illustration Tags:**
- `Figure(alt_text=None)` - Figure/image
- `Formula(alt_text=None)` - Mathematical formula

**Other Tags:**
- `NonStruct()` - Non-structural content
- `Datetime()` - Date/time value
- `Terms()` - Terms and definitions
- `Title()` - Title or headline
- `Strong()` - Strong emphasis
- `Em()` - Emphasis

## Example Usage

### Basic Tagged Document

```python
import krilla

# Create document
doc = krilla.Document()

# Build tag tree
tree = krilla.TagTree()

# Create article with heading and paragraph
article = krilla.TagGroup(krilla.Tag.Article())
heading = krilla.TagGroup(
    krilla.Tag.Hn(1).with_title("Chapter 1")
)
paragraph = krilla.TagGroup(krilla.Tag.P())

# Add tagged content to page
page = doc.start_page()
surface = page.surface()

# Heading content
h_tag = krilla.ContentTag.span(krilla.SpanTag(lang="en"))
h_id = surface.start_tagged(h_tag)
# Draw heading text...
surface.end_tagged()

# Paragraph content
p_tag = krilla.ContentTag.span(krilla.SpanTag(lang="en"))
p_id = surface.start_tagged(p_tag)
# Draw paragraph text...
surface.end_tagged()

surface.finish()
page.finish()

# Build tree structure
heading.push(h_id)
paragraph.push(p_id)
article.push(heading)
article.push(paragraph)
tree.push(article)

# Attach to document
doc.set_tag_tree(tree)

# Serialize with tagging enabled
settings = krilla.SerializeSettings().enable_tagging(True)
pdf = doc.finish(settings)
```

### Table with Headers

```python
import krilla

# Create table structure
table = krilla.TagGroup(krilla.Tag.Table(summary="Sales data"))

# Header row
thead = krilla.TagGroup(krilla.Tag.THead())
header_row = krilla.TagGroup(krilla.Tag.TR())

# Header cells with scope and IDs
th_month = krilla.TagGroup(
    krilla.Tag.TH(krilla.TableHeaderScope.Column)
    .with_id(krilla.TagId.from_str("col-month"))
)
th_sales = krilla.TagGroup(
    krilla.Tag.TH(krilla.TableHeaderScope.Column)
    .with_id(krilla.TagId.from_str("col-sales"))
)

header_row.push(th_month)
header_row.push(th_sales)
thead.push(header_row)
table.push(thead)

# Data rows
tbody = krilla.TagGroup(krilla.Tag.TBody())
data_row = krilla.TagGroup(krilla.Tag.TR())

# Data cells reference headers for accessibility
td_month = krilla.TagGroup(
    krilla.Tag.TD()
    .with_headers([krilla.TagId.from_str("col-month")])
)
td_sales = krilla.TagGroup(
    krilla.Tag.TD()
    .with_headers([krilla.TagId.from_str("col-sales")])
)

data_row.push(td_month)
data_row.push(td_sales)
tbody.push(data_row)
table.push(tbody)

# Add content to cells and include in tag tree...
```

### Ordered List

```python
import krilla

# Create ordered list
list_group = krilla.TagGroup(
    krilla.Tag.L(krilla.ListNumbering.Decimal)
)

# List items with labels and bodies
item1 = krilla.TagGroup(krilla.Tag.LI())
label1 = krilla.TagGroup(krilla.Tag.Lbl())  # "1."
body1 = krilla.TagGroup(krilla.Tag.LBody()) # Item content

item1.push(label1)
item1.push(body1)
list_group.push(item1)

# Repeat for more items...
# Add content to label and body groups...
```

### Figure with Alt Text

```python
import krilla

# Create figure with alternative text for accessibility
figure = krilla.TagGroup(
    krilla.Tag.Figure(alt_text="Company logo - a blue circle with white text")
)

# Add image content to figure...
# Include in document tag tree...
```

### Tag with Layout Attributes

```python
import krilla

# Create paragraph with layout styling
para = (krilla.Tag.P()
    .with_id(krilla.TagId.from_str("intro"))
    .with_lang("en-US")
    .with_placement(krilla.Placement.Block)
    .with_writing_mode(krilla.WritingMode.LrTb)
    .with_color(krilla.NaiveRgbColor(0, 0, 0))
    .with_background_color(krilla.NaiveRgbColor(255, 255, 240))
    .with_padding(krilla.SidesF32.uniform(10.0))
    .with_text_align(krilla.TextAlign.Justify)
)

para_group = krilla.TagGroup(para)
# Add content and include in tag tree...
```

## Best Practices

### Accessibility

1. **Always provide alt text** for figures and formulas:
   ```python
   figure = krilla.Tag.Figure(alt_text="Detailed description of the image")
   ```

2. **Use proper heading hierarchy** (don't skip levels):
   ```python
   h1 = krilla.Tag.Hn(1)  # Main title
   h2 = krilla.Tag.Hn(2)  # Subsection
   h3 = krilla.Tag.Hn(3)  # Sub-subsection
   ```

3. **Link table headers to data cells**:
   ```python
   th = krilla.Tag.TH(scope).with_id(krilla.TagId.from_str("header-id"))
   td = krilla.Tag.TD().with_headers([krilla.TagId.from_str("header-id")])
   ```

4. **Set document language** for screen readers:
   ```python
   tag = krilla.Tag.P().with_lang("en-US")
   ```

### Document Structure

1. **Use semantic tags** that match content meaning:
   ```python
   # Good: Article for self-contained content
   article = krilla.TagGroup(krilla.Tag.Article())
   ```

2. **Maintain logical reading order** in the tag tree:
   ```python
   # Tree structure matches visual/reading order
   article.push(heading)   # First
   article.push(intro)     # Second
   article.push(body)      # Third
   ```

3. **Group related content**:
   ```python
   section = krilla.TagGroup(krilla.Tag.Section())
   section.push(section_heading)
   section.push(section_content)
   ```
