"""Advanced text layout with uharfbuzz - multi-line with styling and RTL.

This example demonstrates sophisticated text rendering capabilities:
1. Multi-line text with word-based line breaking
2. Multi-styled text (different colors and weights)
3. Style-per-glyph rendering (batching glyphs by style)
4. Automatic character-to-byte index conversion (handled by Glyph class)
5. Both LTR (English) and RTL (Arabic) text using unified logic

Key Concepts:
- HarfBuzz cluster indices are CHARACTER positions (0, 1, 2, ...)
- Glyph class handles byte offset conversion automatically
- RTL reordering happens automatically via guess_segment_properties()
- Glyphs include spaces - no manual space handling needed
- Line breaking works directly on glyph arrays
"""

from pathlib import Path

import uharfbuzz as hb
from krilla import (
    Document,
    Fill,
    Font,
    Glyph,  # Pythonic high-level API
    GlyphId,
    NormalizedF32,
    PageSettings,
    Paint,
    Point,
    color,
)


class StyleRange:
    """Represents a style applied to a range of text (character-based)."""

    def __init__(
        self,
        start: int,
        end: int,
        color_rgb: tuple[int, int, int] | None = None,
        bold: bool = False,
    ):
        self.start = start  # Character index
        self.end = end  # Character index
        self.color_rgb = color_rgb if color_rgb else (0, 0, 0)
        self.bold = bold

    def contains(self, pos: int) -> bool:
        """Check if a character position is within this style range."""
        return self.start <= pos < self.end

    def get_paint(self) -> Paint:
        """Get the Paint for this style."""
        return Paint.from_rgb(color.rgb(*self.color_rgb))


def shape_text(text: str, hb_font: hb.Font, krilla_font: Font) -> list[Glyph]:
    """Shape text with uharfbuzz and convert to Glyphs.

    Character-to-byte conversion is handled automatically by the Glyph class.
    """
    # Shape the text
    buf = hb.Buffer()
    buf.add_str(text)
    buf.guess_segment_properties()  # Auto-detects script and direction (LTR/RTL)
    hb.shape(hb_font, buf)

    infos = buf.glyph_infos
    positions = buf.glyph_positions

    # Convert to Glyphs with character-based indices
    glyphs = []
    units_per_em = krilla_font.units_per_em()

    # Collect all unique cluster values to determine text ranges
    clusters = sorted(set(info.cluster for info in infos))
    clusters.append(len(text))  # Add end position (character count)

    for info, pos in zip(infos, positions, strict=True):
        # info.cluster is a CHARACTER index - use it directly!
        char_start = info.cluster

        # Find the next cluster to determine the range
        cluster_idx = clusters.index(char_start)
        char_end = clusters[cluster_idx + 1]

        # Create Glyph with character indices - byte conversion is automatic!
        glyph = Glyph.from_shaper(
            text=text,
            char_start=char_start,  # Character index (natural!)
            char_end=char_end,  # Character index
            glyph_id=GlyphId(info.codepoint),
            x_advance=pos.x_advance / units_per_em,
            x_offset=pos.x_offset / units_per_em,
            y_offset=-pos.y_offset / units_per_em,
        )
        glyphs.append(glyph)

    return glyphs


def find_word_boundaries(glyphs: list[Glyph], text: str) -> list[int]:
    """Find glyph indices where words start and end.

    Returns list of indices in the glyph array marking word boundaries.
    """
    if not glyphs:
        return [0]

    boundaries = [0]  # Start of first word

    for i in range(1, len(glyphs)):
        # Check if we're transitioning from space to non-space
        prev_glyph = glyphs[i - 1]
        curr_glyph = glyphs[i]

        # Get the actual characters these glyphs represent (from .text attribute)
        prev_char = prev_glyph.text
        curr_char = curr_glyph.text

        # Word boundary: transitioning from space to non-space
        if prev_char.strip() == "" and curr_char.strip() != "":
            boundaries.append(i)

    boundaries.append(len(glyphs))  # End of last word
    return boundaries


def break_into_lines(
    glyphs: list[Glyph],
    boundaries: list[int],
    max_width: float,
    font_size: float,
) -> list[tuple[int, int]]:
    """Break glyphs into lines based on word boundaries.

    Returns list of (start_idx, end_idx) tuples for each line in the glyph array.
    """
    if not boundaries or len(boundaries) < 2:
        return [(0, len(glyphs))]

    lines = []
    line_start = 0
    current_width = 0.0

    for i in range(1, len(boundaries)):
        word_start = boundaries[i - 1]
        word_end = boundaries[i]

        # Calculate word width
        word_width = sum(g.x_advance for g in glyphs[word_start:word_end]) * font_size

        # Try to add word to current line
        if current_width + word_width > max_width and line_start < word_start:
            # Word doesn't fit - start new line
            lines.append((line_start, word_start))
            line_start = word_start
            current_width = word_width
        else:
            # Word fits
            current_width += word_width

    # Add final line
    if line_start < len(glyphs):
        lines.append((line_start, len(glyphs)))

    return lines if lines else [(0, len(glyphs))]


def render_line_with_styles(
    surface,
    glyphs: list[Glyph],
    styles: list[StyleRange],
    text: str,
    font: Font,
    font_size: float,
    x_start: float,
    y_pos: float,
):
    """Render a line of glyphs, batching by style.

    This works for both LTR and RTL text - just pass appropriate x_start.
    """
    x = x_start
    cur_x = x_start
    current_style = None
    glyph_batch = []

    for glyph in glyphs:
        # Find style for this glyph's character position
        glyph_style = None
        for style in styles:
            # Use the internal character position (available via _char_start)
            if glyph._char_start is not None and style.contains(glyph._char_start):
                glyph_style = style
                break

        # Flush batch if style changes
        if current_style is not None and glyph_style != current_style:
            surface.set_fill(
                Fill(
                    paint=(
                        current_style.get_paint()
                        if current_style
                        else Paint.from_rgb(color.rgb(0, 0, 0))
                    ),
                    opacity=NormalizedF32.one(),
                )
            )
            surface.draw_glyphs(
                Point.from_xy(cur_x, y_pos),
                glyph_batch,
                font,
                text,
                font_size,
                False,
            )
            glyph_batch = []
            cur_x = x

        current_style = glyph_style
        glyph_batch.append(glyph)
        x += glyph.x_advance * font_size

    # Flush remaining batch
    if glyph_batch:
        surface.set_fill(
            Fill(
                paint=(
                    current_style.get_paint()
                    if current_style
                    else Paint.from_rgb(color.rgb(0, 0, 0))
                ),
                opacity=NormalizedF32.one(),
            )
        )
        surface.draw_glyphs(
            Point.from_xy(cur_x, y_pos),
            glyph_batch,
            font,
            text,
            font_size,
            False,
        )


def main():
    # Load fonts
    assets_path = Path(__file__).parent.parent.parent.parent / "assets"

    # Regular Latin font
    font_path = assets_path / "fonts" / "NotoSans-Regular.ttf"
    font_data = font_path.read_bytes()
    krilla_font = Font.new(font_data, 0)
    if krilla_font is None:
        raise RuntimeError("Failed to load font")

    hb_face = hb.Face(font_data)
    hb_font = hb.Font(hb_face)

    # Arabic font
    arabic_font_path = assets_path / "fonts" / "NotoSansArabic-Regular.ttf"
    arabic_font_data = arabic_font_path.read_bytes()
    krilla_arabic_font = Font.new(arabic_font_data, 0)
    if krilla_arabic_font is None:
        raise RuntimeError("Failed to load Arabic font")

    hb_arabic_face = hb.Face(arabic_font_data)
    hb_arabic_font = hb.Font(hb_arabic_face)

    # Text with styling (similar to parley.rs example)
    text_english = (
        "This is a long text. We want it to not be wider than 200pt, "
        "so that it fits on the page."
    )

    # Arabic text: "مرحبا بالعالم" means "Hello World"
    text_arabic = "مرحبا بالعالم"

    font_size = 16.0
    max_width = 200.0
    line_height = font_size * 1.3

    # Define style ranges for English text (character-based indices)
    styles_english = [
        StyleRange(0, 4, bold=True),  # "This" is bold (characters 0-4)
        # "is a lo" is red (characters 2-12)
        StyleRange(2, 12, color_rgb=(255, 0, 0)),
    ]

    # Style for Arabic text (character-based indices)
    styles_arabic = [
        StyleRange(0, len(text_arabic), color_rgb=(0, 0, 255)),  # All text is blue
    ]

    # ==================== Shape text (unified for LTR and RTL) ====================
    english_glyphs = shape_text(text_english, hb_font, krilla_font)
    arabic_glyphs = shape_text(text_arabic, hb_arabic_font, krilla_arabic_font)

    # ==================== Line breaking for English ====================
    boundaries = find_word_boundaries(english_glyphs, text_english)
    lines = break_into_lines(english_glyphs, boundaries, max_width, font_size)

    # ==================== Calculate baseline offsets ====================
    units_per_em = krilla_font.units_per_em()
    arabic_units_per_em = krilla_arabic_font.units_per_em()

    baseline_offset = (krilla_font.ascent() / units_per_em) * font_size
    arabic_baseline_offset = (
        krilla_arabic_font.ascent() / arabic_units_per_em
    ) * font_size

    # ==================== Create PDF ====================
    doc = Document()
    page_height = (len(lines) + 2) * line_height + 50

    with (
        doc.start_page_with(PageSettings.from_wh(220.0, page_height)) as page,
        page.surface() as surface,
    ):
        y = 20.0

        # ==================== Render English text ====================
        for start_idx, end_idx in lines:
            line_glyphs = english_glyphs[start_idx:end_idx]
            render_line_with_styles(
                surface,
                line_glyphs,
                styles_english,
                text_english,
                krilla_font,
                font_size,
                10.0,  # LTR: start from left
                y + baseline_offset,
            )
            y += line_height

        # ==================== Render Arabic text (RTL) ====================
        y += line_height * 0.5  # Extra spacing

        # For RTL: calculate total width and right-align
        # Glyphs are already in visual (RTL) order from harfbuzz
        arabic_width = sum(g.x_advance for g in arabic_glyphs) * font_size
        arabic_x_start = max_width + 10.0 - arabic_width  # Right-align

        render_line_with_styles(
            surface,
            arabic_glyphs,
            styles_arabic,
            text_arabic,
            krilla_arabic_font,
            font_size,
            arabic_x_start,  # RTL: start from right
            y + arabic_baseline_offset,
        )

    pdf = doc.finish()

    # Save PDF
    output_path = Path("uharfbuzz_layout.pdf").absolute()
    output_path.write_bytes(pdf)
    print(f"Saved PDF to '{output_path}'")
    print(f"English: {len(lines)} lines - {text_english[:40]}...")
    print(f"Arabic: 1 line - {text_arabic}")


if __name__ == "__main__":
    main()
