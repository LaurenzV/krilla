"""A basic example of a PDF file created with krilla."""

from pathlib import Path

from krilla import (
    Document,
    Fill,
    FillRule,
    Font,
    LinearGradient,
    NormalizedF32,
    PageSettings,
    Paint,
    PathBuilder,
    Point,
    SpreadMethod,
    Stop,
    TextDirection,
    color,
)


def main():
    # Create a new document.
    document = Document()
    # Load a font.
    font_path = (
        Path(__file__).parent.parent.parent.parent
        / "assets"
        / "fonts"
        / "NotoSans-Regular.ttf"
    )
    with open(font_path, "rb") as f:
        font_data = f.read()
    font = Font.new(font_data, 0)
    if font is None:
        raise RuntimeError("Failed to load font")

    # Add a new page with dimensions 200x200.
    with document.start_page_with(PageSettings.from_wh(200.0, 200.0)) as page:
        # Get the surface of the page.
        with page.surface() as surface:
            # Draw some text.
            surface.draw_text(
                Point.from_xy(0.0, 25.0),
                font,
                14.0,
                "This text has font size 14!",
                False,
                TextDirection.Auto,
            )

            surface.set_fill(
                Fill(
                    paint=Paint.from_rgb(color.rgb(255, 0, 0)),
                    opacity=NormalizedF32(0.5),
                )
            )
            # Draw some more text, in a different color with an opacity and
            # bigger font size.
            surface.draw_text(
                Point.from_xy(0.0, 50.0),
                font,
                16.0,
                "This text has font size 16!",
                False,
                TextDirection.Auto,
            )

    # Start a new page.
    with document.start_page_with(PageSettings.from_wh(200.0, 200.0)) as page:
        # Create the triangle.
        pb = PathBuilder()
        pb.move_to(100.0, 20.0)
        pb.line_to(160.0, 160.0)
        pb.line_to(40.0, 160.0)
        pb.close()
        triangle = pb.finish()

        # Create the linear gradient.
        lg = LinearGradient(
            x1=60.0,
            y1=0.0,
            x2=140.0,
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
            spread_method=SpreadMethod.Repeat,
            anti_alias=False,
        )

        with page.surface() as surface:
            # Set the fill.
            surface.set_fill(
                Fill(
                    paint=Paint.from_linear_gradient(lg),
                    rule=FillRule.EvenOdd,
                    opacity=NormalizedF32.one(),
                )
            )

            # Fill the path.
            surface.draw_path(triangle)

    # Finish up and write the resulting PDF.
    pdf = document.finish()
    path = Path("basic.pdf").absolute()
    print(f"Saved PDF to '{path}'")

    # Write the PDF to a file.
    with open(path, "wb") as f:
        f.write(pdf)


if __name__ == "__main__":
    main()
