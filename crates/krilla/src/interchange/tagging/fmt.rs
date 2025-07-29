//! Format the tag tree in a human readable way, in form of a YAML document.
//! YAML is a good fit to represent the hierarchy of the tag tree, and many
//! editors allow folding the sub-trees.

use std::fmt::{Debug, Display};

use crate::tagging::{
    Attr, BBox, BlockAlign, BorderStyle, ColumnDimensions, GlyphOrientationVertical, Identifier,
    IdentifierInner, IdentifierType, InlineAlign, LayoutAttr, LineHeight, ListAttr, ListNumbering,
    NaiveRgbColor, Node, Placement, Sides, StructAttr, TableAttr, TableHeaderScope, TagGroup,
    TagId, TagKind, TagTree, TextAlign, TextDecorationType, WritingMode,
};

/// Helper trait for indented output.
pub trait Output {
    /// Wrapper around [`Output::output`] with a zero indent;
    fn output(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        self.output_indent(f, Indent(0))
    }

    /// Output data with an indent.
    fn output_indent(&self, f: &mut impl std::fmt::Write, indent: Indent) -> std::fmt::Result;

    /// Create a [`Display`]able wrapper.
    fn display<'a>(&'a self) -> Wrapper<'a, Self> {
        Wrapper {
            inner: self,
            indent: Indent(0),
        }
    }

    /// Create a [`Display`]able wrapper with a specific indent.
    fn display_indent<'a>(&'a self, indent: Indent) -> Wrapper<'a, Self> {
        Wrapper {
            inner: self,
            indent,
        }
    }
}

/// A [`Display`]able wrapper struct around an [`Output`].
pub struct Wrapper<'a, T: Output + ?Sized> {
    inner: &'a T,
    indent: Indent,
}

impl<T: Output> Display for Wrapper<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.output_indent(f, self.indent)
    }
}

/// A [`Display`]able indentation.
#[derive(Clone, Copy)]
pub struct Indent(pub usize);

impl Indent {
    fn inc(self) -> Self {
        Self(self.0 + 1)
    }
}

impl Display for Indent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:indent$}", "", indent = 2 * self.0)
    }
}

impl Output for TagTree {
    fn output_indent(&self, f: &mut impl std::fmt::Write, indent: Indent) -> std::fmt::Result {
        for child in self.children.iter() {
            child.output_indent(f, indent)?;
        }
        Ok(())
    }
}

impl Output for Node {
    fn output_indent(&self, f: &mut impl std::fmt::Write, indent: Indent) -> std::fmt::Result {
        match self {
            Node::Group(group) => group.output_indent(f, indent),
            Node::Leaf(Identifier(IdentifierInner::Real(IdentifierType::PageIdentifier(pi)))) => {
                writeln!(
                    f,
                    "{indent}- Content: page={} mcid={}",
                    pi.page_index, pi.mcid
                )
            }
            Node::Leaf(Identifier(IdentifierInner::Real(
                IdentifierType::AnnotationIdentifier(ai),
            ))) => {
                writeln!(
                    f,
                    "{indent}- Annotation: page={} index={}",
                    ai.page_index, ai.annot_index
                )
            }
            Node::Leaf(Identifier(IdentifierInner::Dummy)) => writeln!(f, "{indent}- Artifact"),
        }
    }
}

impl Output for TagGroup {
    fn output_indent(&self, f: &mut impl std::fmt::Write, indent: Indent) -> std::fmt::Result {
        self.tag.output_indent(f, indent)?;
        let indent = indent.inc();
        if !self.children.is_empty() {
            writeln!(f, "{indent}/K:")?;
            let indent = indent.inc();
            for node in self.children.iter() {
                node.output_indent(f, indent)?;
            }
        }
        Ok(())
    }
}

impl Output for TagKind {
    fn output_indent(&self, f: &mut impl std::fmt::Write, indent: Indent) -> std::fmt::Result {
        write!(f, "{indent}- Tag: ")?;
        match self {
            TagKind::Part(_) => write!(f, "Part"),
            TagKind::Article(_) => write!(f, "Article"),
            TagKind::Section(_) => write!(f, "Section"),
            TagKind::Div(_) => write!(f, "Div"),
            TagKind::BlockQuote(_) => write!(f, "BlockQuote"),
            TagKind::Caption(_) => write!(f, "Caption"),
            TagKind::TOC(_) => write!(f, "TOC"),
            TagKind::TOCI(_) => write!(f, "TOCI"),
            TagKind::Index(_) => write!(f, "Index"),
            TagKind::P(_) => write!(f, "P"),
            TagKind::Hn(tag) => write!(f, "H{}", tag.level().get()),
            TagKind::L(_) => write!(f, "L"),
            TagKind::LI(_) => write!(f, "LI"),
            TagKind::Lbl(_) => write!(f, "Lbl"),
            TagKind::LBody(_) => write!(f, "LBody"),
            TagKind::Table(_) => write!(f, "Table"),
            TagKind::TR(_) => write!(f, "TR"),
            TagKind::TH(_) => write!(f, "TH"),
            TagKind::TD(_) => write!(f, "TD"),
            TagKind::THead(_) => write!(f, "THead"),
            TagKind::TBody(_) => write!(f, "TBody"),
            TagKind::TFoot(_) => write!(f, "TFoot"),
            TagKind::Span(_) => write!(f, "Span"),
            TagKind::InlineQuote(_) => write!(f, "InlineQuote"),
            TagKind::Note(_) => write!(f, "Note"),
            TagKind::Reference(_) => write!(f, "Reference"),
            TagKind::BibEntry(_) => write!(f, "BibEntry"),
            TagKind::Code(_) => write!(f, "Code"),
            TagKind::Link(_) => write!(f, "Link"),
            TagKind::Annot(_) => write!(f, "Annot"),
            TagKind::Figure(_) => write!(f, "Figure"),
            TagKind::Formula(_) => write!(f, "Formula"),
            TagKind::NonStruct(_) => write!(f, "NonStruct"),
            TagKind::Datetime(_) => write!(f, "Datetime"),
            TagKind::Terms(_) => write!(f, "Terms"),
            TagKind::Title(_) => write!(f, "Title"),
        }?;
        writeln!(f)?;

        let indent = indent.inc();
        for attr in self.as_any().attrs.iter() {
            attr.output_indent(f, indent)?;
        }

        Ok(())
    }
}

impl Output for Attr {
    fn output_indent(&self, f: &mut impl std::fmt::Write, indent: Indent) -> std::fmt::Result {
        use LayoutAttr::*;
        use ListAttr::*;
        use StructAttr::*;
        use TableAttr::*;

        write!(f, "{indent}")?;
        match self {
            Attr::Struct(struct_attr) => match struct_attr {
                Id(id) => writeln!(f, "/Id: {}", id.display()),
                Lang(lang) => writeln!(f, "/Lang: {lang}"),
                AltText(alt) => writeln!(f, "/Alt: {alt}"),
                Expanded(e) => writeln!(f, "/E: {e}"),
                ActualText(actual) => writeln!(f, "/ActualText: {actual}"),
                Title(title) => writeln!(f, "/T: {title}"),

                // Not a real attribute, is already displayed in tag kind.
                HeadingLevel(_) => Ok(()),
            },
            Attr::List(list_attr) => match list_attr {
                Numbering(n) => writeln!(f, "/Numbering: {}", n.display()),
            },
            Attr::Table(table_attr) => match table_attr {
                Summary(summary) => writeln!(f, "/Summary: {summary}"),
                HeaderScope(scope) => writeln!(f, "/Scope: {}", scope.display()),
                CellHeaders(headers) => {
                    write!(f, "/Headers: [")?;
                    if let Some((first, remainder)) = headers.split_first() {
                        first.output(f)?;
                        for id in remainder.iter() {
                            write!(f, ", {}", id.display())?;
                        }
                    }
                    writeln!(f, "]")
                }
                RowSpan(rowspan) => writeln!(f, "/RowSpan: {}", rowspan.get()),
                ColSpan(colspan) => writeln!(f, "/ColSpan: {}", colspan.get()),
            },
            Attr::Layout(layout_attr) => match layout_attr {
                Placement(placement) => writeln!(f, "/Placement: {}", placement.display()),
                WritingMode(mode) => writeln!(f, "/WritingMode: {}", mode.display()),
                BBox(bbox) => writeln!(f, "/BBox: {}", bbox.display_indent(indent.inc())),
                Width(width) => writeln!(f, "/Width: {}", width.display()),
                Height(height) => writeln!(f, "/Height: {}", height.display()),
                BackgroundColor(color) => writeln!(f, "/BackgroundColor: {}", color.display()),
                BorderColor(sides) => {
                    writeln!(f, "/BorderColor: {}", sides.display_indent(indent.inc()))
                }
                BorderStyle(sides) => {
                    writeln!(f, "/BorderStyle: {}", sides.display_indent(indent.inc()))
                }
                BorderThickness(sides) => {
                    writeln!(
                        f,
                        "/BorderThickness: {}",
                        sides.display_indent(indent.inc())
                    )
                }
                Padding(sides) => {
                    writeln!(f, "/Padding: {}", sides.display_indent(indent.inc()))
                }
                Color(color) => writeln!(f, "/Color: {}", color.display()),
                SpaceBefore(space) => writeln!(f, "/SpaceBefore: {}", space.display()),
                SpaceAfter(space) => writeln!(f, "/SpaceAfter: {}", space.display()),
                StartIndent(indent) => writeln!(f, "/StartIndent: {}", indent.display()),
                EndIndent(indent) => writeln!(f, "/EndIndent: {}", indent.display()),
                TextIndent(indent) => writeln!(f, "/TextIndent: {}", indent.display()),
                TextAlign(text_align) => writeln!(f, "/TextAlign: {}", text_align.display()),
                BlockAlign(block_align) => writeln!(f, "/BlockAlign: {}", block_align.display()),
                InlineAlign(inline_align) => {
                    writeln!(f, "/InlineAlign: {}", inline_align.display())
                }
                TableBorderStyle(sides) => writeln!(
                    f,
                    "/TableBorderStyle: {}",
                    sides.display_indent(indent.inc())
                ),
                TablePadding(sides) => {
                    writeln!(f, "/TablePadding: {}", sides.display_indent(indent.inc()))
                }
                BaselineShift(shift) => writeln!(f, "/BaselineShift: {}", shift.display()),
                LineHeight(line_height) => writeln!(
                    f,
                    "/LineHeight: {}",
                    line_height.display_indent(indent.inc())
                ),
                TextDecorationColor(color) => {
                    writeln!(f, "/TextDecorationColor: {}", color.display())
                }
                TextDecorationThickness(thickness) => {
                    writeln!(f, "/TextDecorationThickness: {}", thickness.display())
                }
                TextDecorationType(deco_type) => {
                    writeln!(f, "/TextDecorationType: {}", deco_type.display())
                }
                GlyphOrientationVertical(orientation) => {
                    writeln!(f, "/GlyphOrientationVertical: {}", orientation.display())
                }
                ColumnCount(column_count) => writeln!(f, "/ColumnCount: {}", column_count.get()),
                ColumnGap(column_gap) => {
                    writeln!(f, "/ColumnGap: {}", column_gap.display_indent(indent.inc()))
                }
                ColumnWidths(column_width) => {
                    writeln!(
                        f,
                        "/ColumnWidths: {}",
                        column_width.display_indent(indent.inc())
                    )
                }
            },
        }
    }
}

impl Output for TagId {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        if self
            .as_bytes()
            .iter()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z'))
        {
            let str = std::str::from_utf8(self.as_bytes()).unwrap();
            write!(f, "'{str}'")?;
        } else {
            for b in self.as_bytes() {
                write!(f, "0x{b:02x}")?;
            }
        }
        Ok(())
    }
}

impl Output for ListNumbering {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        match self {
            ListNumbering::None => write!(f, "None"),
            ListNumbering::Disc => write!(f, "Disc"),
            ListNumbering::Circle => write!(f, "Circle"),
            ListNumbering::Square => write!(f, "Square"),
            ListNumbering::Decimal => write!(f, "Decimal"),
            ListNumbering::LowerRoman => write!(f, "LowerRoman"),
            ListNumbering::UpperRoman => write!(f, "UpperRoman"),
            ListNumbering::LowerAlpha => write!(f, "LowerAlpha"),
            ListNumbering::UpperAlpha => write!(f, "UpperAlpha"),
        }
    }
}

impl Output for TableHeaderScope {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        match self {
            TableHeaderScope::Row => write!(f, "Row"),
            TableHeaderScope::Column => write!(f, "Column"),
            TableHeaderScope::Both => write!(f, "Both"),
        }
    }
}

impl Output for Placement {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        match self {
            Placement::Block => write!(f, "Block"),
            Placement::Inline => write!(f, "Inline"),
            Placement::Before => write!(f, "Before"),
            Placement::Start => write!(f, "Start"),
            Placement::End => write!(f, "End"),
        }
    }
}

impl Output for WritingMode {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        match self {
            WritingMode::LrTb => write!(f, "LrTb"),
            WritingMode::RlTb => write!(f, "RlTb"),
            WritingMode::TbRl => write!(f, "TbRl"),
        }
    }
}

impl Output for BBox {
    fn output_indent(&self, f: &mut impl std::fmt::Write, indent: Indent) -> std::fmt::Result {
        writeln!(f)?;
        writeln!(f, "{indent}page: {}", self.page_idx)?;
        writeln!(f, "{indent}left:   {}", self.rect.left().display())?;
        writeln!(f, "{indent}top:    {}", self.rect.top().display())?;
        writeln!(f, "{indent}right:  {}", self.rect.right().display())?;
        write!(f, "{indent}bottom: {}", self.rect.bottom().display())?;
        Ok(())
    }
}

impl<T: Output + Debug + Copy + PartialEq> Output for Sides<T> {
    fn output_indent(&self, f: &mut impl std::fmt::Write, indent: Indent) -> std::fmt::Result {
        match self {
            Sides::All(all) => {
                all.output_indent(f, indent.inc())?;
            }
            Sides::Specific {
                before,
                after,
                start,
                end,
            } => {
                writeln!(f)?;
                writeln!(f, "{indent}before: {}", before.display_indent(indent.inc()))?;
                writeln!(f, "{indent}after:  {}", after.display_indent(indent.inc()))?;
                writeln!(f, "{indent}start:  {}", start.display_indent(indent.inc()))?;
                write!(f, "{indent}end:    {}", end.display_indent(indent.inc()))?;
            }
        }
        Ok(())
    }
}

impl Output for NaiveRgbColor {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        let r = (255.0 * self.red).round() as u8;
        let g = (255.0 * self.green).round() as u8;
        let b = (255.0 * self.blue).round() as u8;
        write!(f, "#{r:02x}{g:02x}{b:02x}")
    }
}

impl Output for BorderStyle {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        match self {
            BorderStyle::None => write!(f, "None"),
            BorderStyle::Hidden => write!(f, "Hidden"),
            BorderStyle::Solid => write!(f, "Solid"),
            BorderStyle::Dashed => write!(f, "Dashed"),
            BorderStyle::Dotted => write!(f, "Dotted"),
            BorderStyle::Double => write!(f, "Double"),
            BorderStyle::Groove => write!(f, "Groove"),
            BorderStyle::Ridge => write!(f, "Ridge"),
            BorderStyle::Inset => write!(f, "Inset"),
            BorderStyle::Outset => write!(f, "Outset"),
        }
    }
}

impl Output for TextAlign {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        match self {
            TextAlign::Start => write!(f, "Start"),
            TextAlign::Center => write!(f, "Center"),
            TextAlign::End => write!(f, "End"),
            TextAlign::Justify => write!(f, "Justify"),
        }
    }
}

impl Output for BlockAlign {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        match self {
            BlockAlign::Begin => write!(f, "Begin"),
            BlockAlign::Middle => write!(f, "Middle"),
            BlockAlign::After => write!(f, "After"),
            BlockAlign::Justify => write!(f, "Justify"),
        }
    }
}

impl Output for InlineAlign {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        match self {
            InlineAlign::Start => write!(f, "Start"),
            InlineAlign::Center => write!(f, "Center"),
            InlineAlign::End => write!(f, "End"),
        }
    }
}

impl Output for LineHeight {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        match self {
            LineHeight::Normal => write!(f, "Normal"),
            LineHeight::Auto => write!(f, "Auto"),
            LineHeight::Custom(custom) => custom.output(f),
        }
    }
}

impl Output for TextDecorationType {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        match self {
            TextDecorationType::None => write!(f, "None"),
            TextDecorationType::Underline => write!(f, "Underline"),
            TextDecorationType::Overline => write!(f, "Overline"),
            TextDecorationType::LineThrough => write!(f, "LineThrough"),
        }
    }
}

impl Output for GlyphOrientationVertical {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        match self {
            GlyphOrientationVertical::Auto => write!(f, "Auto"),
            GlyphOrientationVertical::None => write!(f, "0"),
            GlyphOrientationVertical::Clockwise90 => write!(f, "90"),
            GlyphOrientationVertical::CounterClockwise90 => write!(f, "-90"),
            GlyphOrientationVertical::Clockwise180 => write!(f, "180"),
            GlyphOrientationVertical::CounterClockwise180 => write!(f, "-180"),
            GlyphOrientationVertical::Clockwise270 => write!(f, "270"),
        }
    }
}

impl Output for ColumnDimensions {
    fn output_indent(&self, f: &mut impl std::fmt::Write, indent: Indent) -> std::fmt::Result {
        match self {
            ColumnDimensions::All(all) => all.output(f),
            ColumnDimensions::Specific(list) => {
                let Some((last, remainder)) = list.split_last() else {
                    return Ok(());
                };
                writeln!(f)?;
                for dim in remainder.iter() {
                    writeln!(f, "{indent}- {}", dim.display())?;
                }
                write!(f, "{indent}- {}", last.display())
            }
        }
    }
}

impl Output for f32 {
    fn output_indent(&self, f: &mut impl std::fmt::Write, _: Indent) -> std::fmt::Result {
        write!(f, "{self:7.3}")
    }
}

#[cfg(test)]
mod tests {
    use crate::action::{Action, LinkAction};
    use crate::annotation::{LinkAnnotation, Target};
    use crate::geom::Rect;
    use crate::tagging::fmt::Output;
    use crate::tagging::{
        BBox, ColumnDimensions, LineHeight, NaiveRgbColor, Sides, Tag, TagGroup, TagTree,
    };
    use crate::Document;

    #[test]
    fn display_empty_tag_tree() {
        assert_eq!("", TagTree::new().display().to_string());
    }

    #[test]
    fn display_tag_tree() {
        let mut document = Document::new();
        let mut page = document.start_page();

        let mut tree = TagTree::new();

        let sec = Tag::Section
            .with_lang(Some("de".into()))
            .with_column_widths(Some(ColumnDimensions::Specific(vec![17.0, 23.0, 34.0])))
            .with_column_gap(Some(ColumnDimensions::Specific(vec![3.0, 4.0])));
        let mut sec = TagGroup::new(sec);

        let figure_rect = Rect::from_ltrb(12.1, 12.342, 24.789877, 32.0).unwrap();
        let figure = Tag::Figure(Some("figure alt text".into()))
            .with_actual_text(Some("THE ACTUAL TEXT".into()))
            .with_bbox(Some(BBox::new(0, figure_rect)))
            .with_line_height(Some(LineHeight::Normal));
        let mut figure = TagGroup::new(figure);

        let link_rect = Rect::from_ltrb(12.0, 12.0, 24.0, 32.32).unwrap();
        let link_target =
            Target::Action(Action::Link(LinkAction::new("https://github.com".into())));
        let link_id =
            page.add_tagged_annotation(LinkAnnotation::new(link_rect, link_target).into());
        figure.push(link_id);
        sec.push(figure);

        let border_color = Sides::specific(
            NaiveRgbColor::new(0.1, 0.4, 1.0),
            NaiveRgbColor::new(0.3, 0.5, 0.2),
            NaiveRgbColor::new(0.3, 0.4, 0.3),
            NaiveRgbColor::new(0.0, 0.7, 0.2),
        );
        let table = Tag::Table
            .with_border_color(Some(border_color))
            .with_line_height(Some(LineHeight::Custom(23.0)));
        let table = TagGroup::new(table);
        sec.push(table);

        tree.push(sec);

        let yaml = tree.display().to_string();
        let expected = "\
- Tag: Section
  /Lang: de
  /ColumnGap:\x20
    -   3.000
    -   4.000
  /ColumnWidths:\x20
    -  17.000
    -  23.000
    -  34.000
  /K:
    - Tag: Figure
      /Alt: figure alt text
      /ActualText: THE ACTUAL TEXT
      /BBox:\x20
        page: 0
        left:    12.100
        top:     12.342
        right:   24.790
        bottom:  32.000
      /LineHeight: Normal
      /K:
        - Annotation: page=0 index=0
    - Tag: Table
      /BorderColor:\x20
        before: #1a66ff
        after:  #4d8033
        start:  #4d664d
        end:    #00b333
      /LineHeight:  23.000
";
        assert_eq!(expected, yaml)
    }
}
