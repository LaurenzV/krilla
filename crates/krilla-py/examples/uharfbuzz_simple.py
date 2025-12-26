"""Simple single-line text rendering using uharfbuzz for shaping.

This example demonstrates the basic workflow for using uharfbuzz with krilla-py:
1. Load a font with both uharfbuzz and krilla
2. Shape text using uharfbuzz to get glyph IDs and positions
3. Convert uharfbuzz output to KrillaGlyph objects
4. Render with krilla's draw_glyphs API

This is the foundation for more sophisticated text layout with line breaking
and styling.
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


def main():
    # Load font
    assets_path = Path(__file__).parent.parent.parent.parent / "assets"
    font_path = assets_path / "fonts" / "NotoSans-Regular.ttf"
    font_data = font_path.read_bytes()

    # Create krilla Font
    krilla_font = Font.new(font_data, 0)
    if krilla_font is None:
        raise RuntimeError("Failed to load font")

    # Create uharfbuzz Font
    hb_face = hb.Face(font_data)
    hb_font = hb.Font(hb_face)

    # Text to render (start with simple ASCII to verify basic workflow)
    text = "Hello, uharfbuzz!"
    font_size = 16.0

    # Shape text with uharfbuzz
    buf = hb.Buffer()
    buf.add_str(text)
    buf.guess_segment_properties()
    hb.shape(hb_font, buf)

    # Convert uharfbuzz glyphs to KrillaGlyphs
    infos = buf.glyph_infos
    positions = buf.glyph_positions

    # Build a mapping of clusters to find text ranges
    # Clusters in uharfbuzz are byte offsets, but we need character indices
    krilla_glyphs = []

    for i, (info, pos) in enumerate(zip(infos, positions)):
        # Calculate text range using byte indices (uharfbuzz clusters)
        text_start = info.cluster

        # Find the end by looking at the next different cluster
        text_end = len(text)
        for j in range(i + 1, len(infos)):
            if infos[j].cluster > text_start:
                text_end = infos[j].cluster
                break

        krilla_glyphs.append(
            KrillaGlyph(
                glyph_id=GlyphId(info.codepoint),
                x_advance=pos.x_advance / krilla_font.units_per_em(),
                text_start=text_start,
                text_end=text_end,
                x_offset=pos.x_offset / krilla_font.units_per_em(),
                y_offset=-pos.y_offset / krilla_font.units_per_em(),
                # Negative because PDF y-axis is flipped
            )
        )

    # Create PDF
    doc = Document()
    with doc.start_page_with(PageSettings.from_wh(300.0, 100.0)) as page:
        with page.surface() as surface:
            # Set fill color
            surface.set_fill(
                Fill(
                    paint=Paint.from_rgb(color.rgb(0, 0, 0)),
                    opacity=NormalizedF32.one(),
                )
            )

            # Draw the shaped text
            surface.draw_glyphs(
                Point.from_xy(10.0, 30.0),
                krilla_glyphs,
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
    print(f"Rendered {len(krilla_glyphs)} glyphs for text: {text}")


if __name__ == "__main__":
    main()
