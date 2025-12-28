"""This example shows how to use `simple_text` API to draw simple text in a
single line. It also demonstrates how you can use non-default variation
coordinates.

Note that simple text in this case does not mean that complex scripts
aren't supported (they are, including RTL text!), but the text itself must
not contain mixed scripts. And the font must contain all necessary glyphs,
otherwise the `.notdef` glyph will be emitted instead of font fallback.
"""

from pathlib import Path

from krilla import (
    Document,
    Fill,
    Font,
    LinearGradient,
    NormalizedF32,
    PageSettings,
    Paint,
    Point,
    SpreadMethod,
    Stop,
    Stroke,
    TextDirection,
    color,
)


def main():
    # The usual page setup.
    document = Document()

    # Get asset paths
    assets_path = Path(__file__).parent.parent.parent.parent / "assets"

    with (
        document.start_page_with(PageSettings.from_wh(600.0, 280.0)) as page,
        page.surface() as surface,
    ):
        # Load font
        noto_font_path = assets_path / "fonts" / "NotoSans-Regular.ttf"
        with open(noto_font_path, "rb") as f:
            noto_font = Font.new(f.read(), 0)
        if noto_font is None:
            raise RuntimeError("Failed to load Noto Sans font")

        gradient = LinearGradient(
            x1=30.0,
            y1=0.0,
            x2=50.0,
            y2=0.0,
            stops=[
                Stop(
                    offset=NormalizedF32(0.2),
                    color=color.Color.from_rgb(color.rgb(255, 0, 0)),
                ),
                Stop(
                    offset=NormalizedF32(0.8),
                    color=color.Color.from_rgb(color.rgb(255, 255, 0)),
                ),
            ],
            spread_method=SpreadMethod.Reflect,
            anti_alias=True,
        )

        surface.set_fill(
            Fill(
                paint=Paint.from_linear_gradient(gradient),
                opacity=NormalizedF32(0.5),
            )
        )

        # Let's first write some red-colored text with some English text.
        surface.draw_text(
            Point.from_xy(0.0, 25.0),
            noto_font,
            25.0,
            "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔͆̏̓͢",
            False,
            TextDirection.Auto,
        )

        surface.set_fill(None)
        surface.set_stroke(
            Stroke(
                paint=Paint.from_rgb(color.rgb(0, 255, 0)),
            )
        )
        # Instead of applying fills, we can also apply strokes!
        surface.draw_text(
            Point.from_xy(0.0, 50.0),
            noto_font,
            25.0,
            "This text is stroked green!",
            False,
            TextDirection.Auto,
        )

        # Load Arabic font
        arabic_font_path = assets_path / "fonts" / "NotoSansArabic-Regular.ttf"
        with open(arabic_font_path, "rb") as f:
            noto_arabic_font = Font.new(f.read(), 0)
        if noto_arabic_font is None:
            raise RuntimeError("Failed to load Noto Sans Arabic font")

        surface.set_fill(None)
        # As mentioned above, complex scripts are supported, you just
        # can't mix them in one run.
        surface.draw_text(
            Point.from_xy(0.0, 75.0),
            noto_arabic_font,
            25.0,
            "هذا هو السطر الثاني من النص.",
            False,
            TextDirection.Auto,
        )

    pdf = document.finish()

    path = Path("simple_text.pdf").absolute()
    print(f"Saved PDF to '{path}'")

    # Write the PDF to a file.
    with open(path, "wb") as f:
        f.write(pdf)


if __name__ == "__main__":
    main()
