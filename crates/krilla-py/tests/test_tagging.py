"""Tests for PDF tag tree functionality (PDF/UA accessibility)."""

import krilla
import pytest


def test_empty_tag_tree():
    """Test creating document with empty tag tree."""
    doc = krilla.Document()
    tag_tree = krilla.TagTree()
    doc.set_tag_tree(tag_tree)

    # Should be able to finish without errors
    pdf = doc.finish()
    assert len(pdf) > 0


def test_tag_tree_basic():
    """Test basic tag tree structure."""
    tree = krilla.TagTree()

    # Create a simple article with paragraph
    article = krilla.TagGroup(krilla.Tag.Article())
    paragraph = krilla.TagGroup(krilla.Tag.P())

    article.push(paragraph)
    tree.push(article)

    # Verify structure
    assert tree is not None


def test_all_simple_tags():
    """Test all 33 tag type variants can be created."""
    # Simple tags (25 variants)
    assert krilla.Tag.Part() is not None
    assert krilla.Tag.Article() is not None
    assert krilla.Tag.Section() is not None
    assert krilla.Tag.Div() is not None
    assert krilla.Tag.BlockQuote() is not None
    assert krilla.Tag.Caption() is not None
    assert krilla.Tag.TOC() is not None
    assert krilla.Tag.TOCI() is not None
    assert krilla.Tag.Index() is not None
    assert krilla.Tag.P() is not None
    assert krilla.Tag.LI() is not None
    assert krilla.Tag.Lbl() is not None
    assert krilla.Tag.LBody() is not None
    assert krilla.Tag.TR() is not None
    assert krilla.Tag.TD() is not None
    assert krilla.Tag.THead() is not None
    assert krilla.Tag.TBody() is not None
    assert krilla.Tag.TFoot() is not None
    assert krilla.Tag.Span() is not None
    assert krilla.Tag.InlineQuote() is not None
    assert krilla.Tag.Note() is not None
    assert krilla.Tag.Reference() is not None
    assert krilla.Tag.BibEntry() is not None
    assert krilla.Tag.Code() is not None
    assert krilla.Tag.Link() is not None
    assert krilla.Tag.Annot() is not None
    assert krilla.Tag.NonStruct() is not None
    assert krilla.Tag.Datetime() is not None
    assert krilla.Tag.Terms() is not None
    assert krilla.Tag.Title() is not None
    assert krilla.Tag.Strong() is not None
    assert krilla.Tag.Em() is not None


def test_heading_with_level():
    """Test heading tag with level parameter."""
    h1 = krilla.Tag.Hn(1)
    h2 = krilla.Tag.Hn(2)
    h6 = krilla.Tag.Hn(6)

    assert h1 is not None
    assert h2 is not None
    assert h6 is not None


def test_heading_invalid_level():
    """Test heading with invalid level raises error."""
    with pytest.raises(ValueError):
        krilla.Tag.Hn(0)  # Level must be non-zero


def test_list_with_numbering():
    """Test list tag with numbering styles."""
    list_none = krilla.Tag.L(krilla.ListNumbering.None_)
    list_disc = krilla.Tag.L(krilla.ListNumbering.Disc)
    list_decimal = krilla.Tag.L(krilla.ListNumbering.Decimal)
    list_roman = krilla.Tag.L(krilla.ListNumbering.LowerRoman)

    assert list_none is not None
    assert list_disc is not None
    assert list_decimal is not None
    assert list_roman is not None


def test_table_with_summary():
    """Test table tag with optional summary."""
    table_no_summary = krilla.Tag.Table()
    table_with_summary = krilla.Tag.Table(summary="Sales data for Q4")

    assert table_no_summary is not None
    assert table_with_summary is not None


def test_table_header_scope():
    """Test table header cell with scope."""
    th_row = krilla.Tag.TH(krilla.TableHeaderScope.Row)
    th_col = krilla.Tag.TH(krilla.TableHeaderScope.Column)
    th_both = krilla.Tag.TH(krilla.TableHeaderScope.Both)

    assert th_row is not None
    assert th_col is not None
    assert th_both is not None


def test_figure_with_alt_text():
    """Test figure tag with alternate text."""
    figure_no_alt = krilla.Tag.Figure()
    figure_with_alt = krilla.Tag.Figure(alt_text="Company logo")

    assert figure_no_alt is not None
    assert figure_with_alt is not None


def test_formula_with_alt_text():
    """Test formula tag with alternate text."""
    formula_no_alt = krilla.Tag.Formula()
    formula_with_alt = krilla.Tag.Formula(alt_text="E equals mc squared")

    assert formula_no_alt is not None
    assert formula_with_alt is not None


def test_tag_with_global_attributes():
    """Test tag with global attributes (id, lang, alt_text, etc.)."""
    tag = krilla.Tag.P()

    # Set attributes using builder pattern
    tag_with_id = tag.with_id(krilla.TagId.from_str("para-1"))
    tag_with_lang = tag_with_id.with_lang("en-US")
    tag_with_alt = tag_with_lang.with_alt_text("Paragraph content")

    # Verify attributes can be retrieved
    assert tag_with_alt.id() is not None
    assert tag_with_alt.lang() == "en-US"
    assert tag_with_alt.alt_text() == "Paragraph content"


def test_tag_id_creation():
    """Test TagId creation methods."""
    # From string
    id1 = krilla.TagId.from_str("my-tag-id")
    id2 = krilla.TagId.from_str("my-tag-id")
    id3 = krilla.TagId.from_str("different-id")

    # IDs with same string should be equal
    assert id1 == id2
    assert id1 != id3

    # Should be hashable
    id_set = {id1, id2, id3}
    assert len(id_set) == 2  # id1 and id2 are same


def test_bbox_attribute():
    """Test bounding box attribute."""
    rect = krilla.Rect.from_xywh(0, 0, 100, 50)
    bbox = krilla.BBox(0, rect)

    assert bbox.page_idx == 0
    assert bbox.rect is not None

    # Use bbox in tag
    tag = krilla.Tag.Figure().with_id(krilla.TagId.from_str("fig-1"))
    # Note: bbox is read-only, set via layout during content creation


def test_naive_rgb_color():
    """Test NaiveRgbColor creation."""
    # From 8-bit values
    color1 = krilla.NaiveRgbColor(255, 128, 64)
    assert color1.red == 255
    assert color1.green == 128
    assert color1.blue == 64

    # From normalized floats
    color2 = krilla.NaiveRgbColor.new_f32(1.0, 0.5, 0.25)
    assert color2.red == 255
    assert abs(color2.green - 128) <= 1  # Rounding tolerance
    assert abs(color2.blue - 64) <= 1


def test_naive_rgb_color_invalid():
    """Test NaiveRgbColor with invalid float values."""
    with pytest.raises(ValueError):
        krilla.NaiveRgbColor.new_f32(1.5, 0.5, 0.5)  # > 1.0

    with pytest.raises(ValueError):
        krilla.NaiveRgbColor.new_f32(-0.1, 0.5, 0.5)  # < 0.0


def test_sides_f32():
    """Test SidesF32 for padding/border values."""
    # Specific values for each side
    sides1 = krilla.SidesF32(10.0, 20.0, 15.0, 25.0)
    assert sides1.before == 10.0
    assert sides1.after == 20.0
    assert sides1.start == 15.0
    assert sides1.end == 25.0

    # Uniform value
    sides2 = krilla.SidesF32.uniform(10.0)
    assert sides2.before == 10.0
    assert sides2.after == 10.0
    assert sides2.start == 10.0
    assert sides2.end == 10.0


def test_column_dimensions():
    """Test ColumnDimensions for table columns."""
    # All columns same width
    cols1 = krilla.ColumnDimensions.all(100.0)
    assert cols1 is not None

    # Specific widths for each column
    cols2 = krilla.ColumnDimensions.specific([80.0, 120.0, 100.0])
    assert cols2 is not None


def test_line_height():
    """Test LineHeight variants."""
    lh_normal = krilla.LineHeight.normal()
    lh_auto = krilla.LineHeight.auto()
    lh_custom = krilla.LineHeight.custom(1.5)

    assert lh_normal is not None
    assert lh_auto is not None
    assert lh_custom is not None


def test_tag_with_layout_attributes():
    """Test tag with layout attributes."""
    tag = (
        krilla.Tag.P()
        .with_placement(krilla.Placement.Block)
        .with_writing_mode(krilla.WritingMode.LrTb)
        .with_color(krilla.NaiveRgbColor(0, 0, 0))
        .with_background_color(krilla.NaiveRgbColor(255, 255, 255))
        .with_padding(krilla.SidesF32.uniform(10.0))
    )

    # Verify attributes
    assert tag.placement() == krilla.Placement.Block
    assert tag.writing_mode() == krilla.WritingMode.LrTb
    assert tag.color() is not None
    assert tag.background_color() is not None
    assert tag.padding() is not None


def test_nested_tag_structure():
    """Test deeply nested tag tree."""
    tree = krilla.TagTree()

    # Article > Section > Div > Paragraph
    article = krilla.TagGroup(krilla.Tag.Article())
    section = krilla.TagGroup(krilla.Tag.Section())
    div = krilla.TagGroup(krilla.Tag.Div())
    para = krilla.TagGroup(krilla.Tag.P())

    div.push(para)
    section.push(div)
    article.push(section)
    tree.push(article)

    assert tree is not None


def test_table_structure():
    """Test complete table structure with headers."""
    tree = krilla.TagTree()

    # Create table
    table = krilla.TagGroup(krilla.Tag.Table(summary="Employee data"))

    # Header row
    thead = krilla.TagGroup(krilla.Tag.THead())
    header_row = krilla.TagGroup(krilla.Tag.TR())

    th1 = krilla.TagGroup(
        krilla.Tag.TH(krilla.TableHeaderScope.Column).with_id(
            krilla.TagId.from_str("col-name")
        )
    )
    th2 = krilla.TagGroup(
        krilla.Tag.TH(krilla.TableHeaderScope.Column).with_id(
            krilla.TagId.from_str("col-dept")
        )
    )

    header_row.push(th1)
    header_row.push(th2)
    thead.push(header_row)
    table.push(thead)

    # Body with data row
    tbody = krilla.TagGroup(krilla.Tag.TBody())
    data_row = krilla.TagGroup(krilla.Tag.TR())

    td1 = krilla.TagGroup(krilla.Tag.TD())
    td2 = krilla.TagGroup(krilla.Tag.TD())

    data_row.push(td1)
    data_row.push(td2)
    tbody.push(data_row)
    table.push(tbody)

    tree.push(table)
    assert tree is not None


def test_list_structure():
    """Test list structure with items."""
    tree = krilla.TagTree()

    # Ordered list
    list_group = krilla.TagGroup(krilla.Tag.L(krilla.ListNumbering.Decimal))

    # List items
    item1 = krilla.TagGroup(krilla.Tag.LI())
    label1 = krilla.TagGroup(krilla.Tag.Lbl())
    body1 = krilla.TagGroup(krilla.Tag.LBody())

    item1.push(label1)
    item1.push(body1)
    list_group.push(item1)

    tree.push(list_group)
    assert tree is not None


def test_node_type_checking():
    """Test Node type checking methods."""
    group = krilla.TagGroup(krilla.Tag.P())
    node_group = krilla.Node.from_group(group)

    assert node_group.is_group()
    assert not node_group.is_leaf()

    # Note: Leaf nodes require actual Identifier from surface.start_tagged()
    # Can't test is_leaf() without creating actual document content


def test_tag_group_with_children():
    """Test TagGroup.with_children static method."""
    para1 = krilla.TagGroup(krilla.Tag.P())
    para2 = krilla.TagGroup(krilla.Tag.P())

    # Create nodes
    node1 = krilla.Node.from_group(para1)
    node2 = krilla.Node.from_group(para2)

    # Create section with children
    section = krilla.TagGroup.with_children(krilla.Tag.Section(), [node1, node2])

    assert section is not None


def test_tag_attribute_enums():
    """Test all attribute enum types."""
    # ListNumbering
    assert krilla.ListNumbering.Decimal is not None
    assert krilla.ListNumbering.LowerRoman is not None

    # TableHeaderScope
    assert krilla.TableHeaderScope.Row is not None
    assert krilla.TableHeaderScope.Column is not None

    # Placement
    assert krilla.Placement.Block is not None
    assert krilla.Placement.Inline is not None

    # WritingMode
    assert krilla.WritingMode.LrTb is not None
    assert krilla.WritingMode.RlTb is not None

    # BorderStyle
    assert krilla.BorderStyle.Solid is not None
    assert krilla.BorderStyle.Dashed is not None

    # TextAlign
    assert krilla.TextAlign.Start is not None
    assert krilla.TextAlign.Center is not None

    # BlockAlign
    assert krilla.BlockAlign.Begin is not None
    assert krilla.BlockAlign.Middle is not None

    # InlineAlign
    assert krilla.InlineAlign.Start is not None
    assert krilla.InlineAlign.Center is not None

    # TextDecorationType
    assert krilla.TextDecorationType.Underline is not None
    assert krilla.TextDecorationType.Overline is not None

    # GlyphOrientationVertical
    assert krilla.GlyphOrientationVertical.Auto is not None
    assert krilla.GlyphOrientationVertical.Clockwise90 is not None


def test_document_set_tag_tree_integration():
    """Test Document.set_tag_tree integration."""
    doc = krilla.Document()

    # Create a simple tag tree
    tree = krilla.TagTree()
    article = krilla.TagGroup(krilla.Tag.Article())
    tree.push(article)

    # Should not raise
    doc.set_tag_tree(tree)

    # Finish document
    pdf = doc.finish()
    assert len(pdf) > 0


def test_tag_attributes_readonly():
    """Test read-only tag attributes."""
    # Level is read-only (set via Hn constructor)
    tag = krilla.Tag.Hn(3)
    assert tag.level() == 3

    # Numbering is read-only (set via L constructor)
    tag2 = krilla.Tag.L(krilla.ListNumbering.Decimal)
    assert tag2.numbering() == krilla.ListNumbering.Decimal

    # Scope is read-only (set via TH constructor)
    tag3 = krilla.Tag.TH(krilla.TableHeaderScope.Row)
    assert tag3.scope() == krilla.TableHeaderScope.Row


def test_multiple_tag_trees():
    """Test that we can create multiple independent tag trees."""
    tree1 = krilla.TagTree()
    tree1.push(krilla.TagGroup(krilla.Tag.Article()))

    tree2 = krilla.TagTree()
    tree2.push(krilla.TagGroup(krilla.Tag.Section()))

    # Both trees should be independent
    assert tree1 is not None
    assert tree2 is not None
    assert tree1 is not tree2
