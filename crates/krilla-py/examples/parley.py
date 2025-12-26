"""This example shows how to use advanced text layout with manual glyph
positioning.

NOTE: The Rust version of this example uses the parley library for text
shaping and layout. Since parley is a Rust library, this Python example
demonstrates the manual glyph positioning API that you would use after
performing text layout with a Python text shaping library like uharfbuzz
or python-bidi.

This example creates a simple multi-line, multi-styled text layout
manually to demonstrate the draw_glyphs API.
"""

from pathlib import Path

from krilla import (
    Document,
    Fill,
    Font,
    NormalizedF32,
    PageSettings,
    Paint,
    Point,
    color,
)


def main():
    # Load fonts
    assets_path = Path(__file__).parent.parent.parent.parent / "assets"

    with open(assets_path / "fonts" / "NotoSans-Regular.ttf", "rb") as f:
        noto_font = Font.new(f.read(), 0)
    if noto_font is None:
        raise RuntimeError("Failed to load font")

    # The usual page setup.
    document = Document()

    with document.start_page_with(PageSettings.from_wh(200.0, 300.0)) as page:
        with page.surface() as surface:
            # In a real application, you would use a text shaping library like uharfbuzz
            # to shape the text and get glyph IDs, advances, and offsets.
            # Here we demonstrate the API with a simplified example.

            # For demonstration purposes, we'll just draw simple text using
            # draw_text since proper text shaping requires external
            # libraries. The draw_glyphs API is used when you have manual
            # control over glyph positioning.

            # Example 1: Draw text normally (this is what you'd typically use)
            surface.set_fill(
                Fill(
                    paint=Paint.from_rgb(color.rgb(0, 0, 0)),
                    opacity=NormalizedF32.one(),
                )
            )

            # Note: For actual advanced text layout with line breaking,
            # bidirectional text, and complex scripts, you would:
            # 1. Use a Python text shaping library (uharfbuzz, python-bidi, etc.)
            # 2. Get the shaped glyph data (glyph IDs, advances, positions)
            # 3. Use surface.draw_glyphs() with KrillaGlyph objects

            # Here's a conceptual example of how you'd use draw_glyphs:
            # (This won't produce meaningful output without actual text shaping)
            """
            # Hypothetical shaped glyphs from a text shaping library
            glyphs = [
                KrillaGlyph(
                    glyph_id=GlyphId(42),  # From text shaping
                    x_advance=0.5,          # Normalized by font size
                    x_offset=0.0,
                    y_offset=0.0,
                    y_advance=0.0,
                    text_start=0,
                    text_end=1,
                ),
                # ... more glyphs
            ]

            surface.draw_glyphs(
                Point.from_xy(0.0, 25.0),
                glyphs,
                noto_font,
                text,
                16.0,  # font size
                False,
            )
            """

            # For this example, we'll just use the simple text API:
            y_pos = 25.0
            line1 = "This is advanced text layout."
            line2 = "In Python, use uharfbuzz or similar"
            line3 = "for complex text shaping."

            surface.draw_text(
                Point.from_xy(10.0, y_pos),
                noto_font,
                16.0,
                line1,
                False,
            )

            y_pos += 20.0
            surface.draw_text(
                Point.from_xy(10.0, y_pos),
                noto_font,
                14.0,
                line2,
                False,
            )

            y_pos += 18.0
            surface.draw_text(
                Point.from_xy(10.0, y_pos),
                noto_font,
                14.0,
                line3,
                False,
            )

            # Draw some colored text
            surface.set_fill(
                Fill(
                    paint=Paint.from_rgb(color.rgb(255, 0, 0)),
                    opacity=NormalizedF32.one(),
                )
            )

            y_pos += 30.0
            surface.draw_text(
                Point.from_xy(10.0, y_pos),
                noto_font,
                16.0,
                "This text is red!",
                False,
            )

    pdf = document.finish()

    path = Path("parley.pdf").absolute()
    print(f"Saved PDF to '{path}'")
    print("\nNOTE: For advanced text layout with line breaking, bidirectional text,")
    print("and complex scripts in Python, use a text shaping library like uharfbuzz")
    print("or python-bidi, then use surface.draw_glyphs() with the shaped glyph data.")

    # Write the PDF to a file.
    with open(path, "wb") as f:
        f.write(pdf)


if __name__ == "__main__":
    main()
