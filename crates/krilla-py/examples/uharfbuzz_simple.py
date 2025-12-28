"""Simple text shaping example with uharfbuzz.

This example demonstrates the Pythonic pattern for using uharfbuzz with krilla:
1. Shape text with uharfbuzz to get glyphs and positioning
2. Create Glyph objects with character indices (natural for Python)
3. Render glyphs to PDF

The character-to-byte conversion is handled automatically by the Glyph class.
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


def main():
    # Use text with accented characters to demonstrate Unicode handling
    # Character-to-byte conversion is now handled automatically!
    text = "Hello, café!"

    # Load font
    assets_path = Path(__file__).parent.parent.parent.parent / "assets"
    font_path = assets_path / "fonts" / "NotoSans-Regular.ttf"
    font_data = font_path.read_bytes()

    # Create krilla font
    krilla_font = Font.new(font_data, 0)
    if krilla_font is None:
        raise RuntimeError("Failed to load font")

    # Create uharfbuzz font
    hb_face = hb.Face(font_data)
    hb_font = hb.Font(hb_face)

    # Shape the text with uharfbuzz
    buf = hb.Buffer()
    buf.add_str(text)
    buf.guess_segment_properties()
    hb.shape(hb_font, buf)

    # Get shaping results
    infos = buf.glyph_infos
    positions = buf.glyph_positions

    # Convert to Glyphs with character-based indices (natural for Python!)
    glyphs = []
    units_per_em = krilla_font.units_per_em()

    # Collect all unique cluster values to determine text ranges
    clusters = sorted(set(info.cluster for info in infos))
    clusters.append(len(text))  # Add end position (character count)

    for info, pos in zip(infos, positions):
        # info.cluster is a CHARACTER index - use it directly!
        char_start = info.cluster

        # Find the next cluster to determine the range
        cluster_idx = clusters.index(char_start)
        char_end = clusters[cluster_idx + 1]

        # Create Glyph with character indices - byte conversion is automatic!
        glyph = Glyph.from_shaper(
            text=text,
            char_start=char_start,  # Character index (natural!)
            char_end=char_end,      # Character index
            glyph_id=GlyphId(info.codepoint),
            x_advance=pos.x_advance / units_per_em,
            x_offset=pos.x_offset / units_per_em,
            y_offset=-pos.y_offset / units_per_em,
        )
        glyphs.append(glyph)

    # Create PDF and render the text
    font_size = 24.0
    doc = Document()

    with (
        doc.start_page_with(PageSettings.from_wh(200, 100)) as page,
        page.surface() as surface,
    ):
        # Set fill color
        surface.set_fill(
            Fill(
                paint=Paint.from_rgb(color.rgb(0, 0, 0)),
                opacity=NormalizedF32.one(),
            )
        )

        # Calculate baseline position
        baseline = (krilla_font.ascent() / units_per_em) * font_size

        # Draw all glyphs at once
        surface.draw_glyphs(
            Point.from_xy(10.0, 20.0 + baseline),
            glyphs,
            krilla_font,
            text,
            font_size,
            False,
        )

    pdf = doc.finish()

    # Save PDF
    output_path = Path("uharfbuzz_simple.pdf").absolute()
    output_path.write_bytes(pdf)
    print(f"Saved PDF to '{output_path}'")
    print(f"Text: '{text}'")
    print(
        f"Characters: {len(text)}, Bytes: {len(text.encode('utf-8'))}, "
        f"Glyphs: {len(glyphs)}"
    )


if __name__ == "__main__":
    main()
