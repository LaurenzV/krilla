"""Advanced text layout with uharfbuzz - multi-line with styling.

This example demonstrates sophisticated text rendering capabilities similar to
the Rust parley.rs example, using uharfbuzz for text shaping:

1. Multi-line text with simple line breaking
2. Multi-styled text (bold/regular, different colors)
3. Style-per-glyph rendering (batching glyphs by style)
4. Proper glyph positioning and text range mapping

This serves as a foundation for text layout engines like those needed in
OCRmyPDF or other document processing applications.
"""

from pathlib import Path

import uharfbuzz as hb
from krilla import (
    Document,
    Fill,
    Font,
    GlyphId,
    KrillaGlyph,
    NormalizedF32,
    PageSettings,
    Paint,
    Point,
    color,
)


class StyleRange:
    """Represents a style applied to a range of text."""

    def __init__(
        self,
        start: int,
        end: int,
        color_rgb: tuple[int, int, int] | None = None,
        bold: bool = False,
    ):
        self.start = start
        self.end = end
        self.color_rgb = color_rgb if color_rgb else (0, 0, 0)
        self.bold = bold

    def contains(self, pos: int) -> bool:
        """Check if a position is within this style range."""
        return self.start <= pos < self.end

    def get_paint(self) -> Paint:
        """Get the Paint for this style."""
        return Paint.from_rgb(color.rgb(*self.color_rgb))


def simple_line_break(
    words: list[str], word_widths: list[float], max_width: float
) -> list[list[int]]:
    """Simple line breaking algorithm - wrap at word boundaries.

    Returns: List of lines, where each line is a list of word indices.
    """
    lines = []
    current_line = []
    current_width = 0.0
    space_width = word_widths[0] * 0.25  # Approximate space width

    for i, (word, width) in enumerate(zip(words, word_widths)):
        word_width = width + (space_width if current_line else 0)

        if current_line and current_width + word_width > max_width:
            # Start new line
            lines.append(current_line)
            current_line = [i]
            current_width = width
        else:
            current_line.append(i)
            current_width += word_width

    if current_line:
        lines.append(current_line)

    return lines


def main():
    # Load fonts
    assets_path = Path(__file__).parent.parent.parent.parent / "assets"

    # Regular font
    font_path = assets_path / "fonts" / "NotoSans-Regular.ttf"
    font_data = font_path.read_bytes()
    krilla_font = Font.new(font_data, 0)
    if krilla_font is None:
        raise RuntimeError("Failed to load font")

    # Bold font (for this example, we'll just use regular and simulate with style)
    # In a real application, you'd load NotoSans-Bold.ttf

    # Create uharfbuzz font
    hb_face = hb.Face(font_data)
    hb_font = hb.Font(hb_face)

    # Text with styling (similar to parley.rs example)
    # Characters 0-4 are bold, characters 2-12 are red
    text = (
        "This is a long text. We want it to not be wider than 200pt, "
        "so that it fits on the page."
    )

    font_size = 16.0
    max_width = 200.0
    line_height = font_size * 1.3  # 1.3x line spacing

    # Define style ranges
    styles = [
        StyleRange(0, 4, bold=True),  # "This" is bold
        StyleRange(2, 12, color_rgb=(255, 0, 0)),  # "is is a lo" is red
    ]

    # Split text into words for line breaking
    words = text.split()

    # Shape each word and calculate widths
    word_glyphs = []
    word_widths = []
    word_byte_offsets = []  # Track where each word starts in the original text

    byte_offset = 0
    for word in words:
        buf = hb.Buffer()
        buf.add_str(word)
        buf.guess_segment_properties()
        hb.shape(hb_font, buf)

        infos = buf.glyph_infos
        positions = buf.glyph_positions

        # Calculate word width
        width = (
            sum(pos.x_advance for pos in positions)
            / krilla_font.units_per_em()
            * font_size
        )
        word_widths.append(width)
        word_byte_offsets.append(byte_offset)

        # Store glyph info for this word
        word_glyphs.append((infos, positions))

        byte_offset += len(word.encode("utf-8")) + 1  # +1 for space

    # Break into lines
    lines = simple_line_break(words, word_widths, max_width)

    # Calculate line metrics using font metrics
    ascent = krilla_font.ascent()
    units_per_em = krilla_font.units_per_em()

    # Normalize to font size
    baseline_offset = (ascent / units_per_em) * font_size

    # Create PDF
    doc = Document()
    page_height = len(lines) * line_height + 50

    with doc.start_page_with(PageSettings.from_wh(220.0, page_height)) as page:
        with page.surface() as surface:
            y = 20.0  # Starting y position

            for line_words in lines:
                x = 10.0  # Starting x position for each line

                # Process each word in the line
                for word_idx in line_words:
                    word = words[word_idx]
                    infos, positions = word_glyphs[word_idx]
                    word_byte_offset = word_byte_offsets[word_idx]

                    # Convert to KrillaGlyphs
                    krilla_glyphs = []
                    for i, (info, pos) in enumerate(zip(infos, positions)):
                        text_start = word_byte_offset + info.cluster

                        # Find text_end
                        text_end = word_byte_offset + len(word.encode("utf-8"))
                        for j in range(i + 1, len(infos)):
                            if infos[j].cluster > info.cluster:
                                text_end = word_byte_offset + infos[j].cluster
                                break

                        krilla_glyphs.append(
                            KrillaGlyph(
                                glyph_id=GlyphId(info.codepoint),
                                x_advance=pos.x_advance / units_per_em,
                                text_start=text_start,
                                text_end=text_end,
                                x_offset=pos.x_offset / units_per_em,
                                y_offset=-pos.y_offset / units_per_em,
                            )
                        )

                # Group glyphs by style and render in batches
                # This mimics the parley.rs pattern of flushing when style changes
                current_style = None
                glyph_batch = []
                batch_start_x = x

                for glyph in krilla_glyphs:
                    # Find which style applies to this glyph
                    glyph_style = None
                    for style in styles:
                        if style.contains(glyph.text_start):
                            glyph_style = style
                            break

                    # If style changed, flush the batch
                    if glyph_style != current_style:
                        if glyph_batch:
                            # Render previous batch
                            surface.set_fill(
                                Fill(
                                    paint=current_style.get_paint()
                                    if current_style
                                    else Paint.from_rgb(color.rgb(0, 0, 0)),
                                    opacity=NormalizedF32.one(),
                                )
                            )
                            surface.draw_glyphs(
                                Point.from_xy(batch_start_x, y + baseline_offset),
                                glyph_batch,
                                krilla_font,
                                text,
                                font_size,
                                False,
                            )
                            # Advance x by the width of rendered glyphs
                            for g in glyph_batch:
                                batch_start_x += g.x_advance * font_size

                        # Start new batch
                        glyph_batch = []
                        current_style = glyph_style

                    glyph_batch.append(glyph)

                # Flush remaining batch for this word
                if glyph_batch:
                    surface.set_fill(
                        Fill(
                            paint=current_style.get_paint()
                            if current_style
                            else Paint.from_rgb(color.rgb(0, 0, 0)),
                            opacity=NormalizedF32.one(),
                        )
                    )
                    surface.draw_glyphs(
                        Point.from_xy(batch_start_x, y + baseline_offset),
                        glyph_batch,
                        krilla_font,
                        text,
                        font_size,
                        False,
                    )

                # Advance x for next word (word width + space)
                x += word_widths[word_idx] + (font_size * 0.25)

            # Move to next line
            y += line_height

    pdf = doc.finish()

    # Save PDF
    output_path = Path("uharfbuzz_layout.pdf").absolute()
    output_path.write_bytes(pdf)
    print(f"Saved PDF to '{output_path}'")
    print(f"Rendered {len(lines)} lines")
    print(f"Text: {text[:50]}...")


if __name__ == "__main__":
    main()
