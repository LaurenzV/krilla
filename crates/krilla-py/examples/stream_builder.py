"""This example introduces the concept of a `StreamBuilder`, which allows
you to define graphics content on a separate context than the main page
surface. This is necessary when defining patterns or mask.
"""

from pathlib import Path

from krilla import (
    Document,
    Fill,
    PageSettings,
    Paint,
    PathBuilder,
    Pattern,
    Rect,
    Size,
    StreamBuilder,
    Stroke,
    Transform,
    color,
)


def rect_to_path(x1: float, y1: float, x2: float, y2: float):
    """A simple convenience function that allow us to generate rectangle paths."""
    builder = PathBuilder()
    builder.push_rect(Rect.from_ltrb(x1, y1, x2, y2))
    return builder.finish()


def main():
    document = Document()

    with document.start_page_with(PageSettings.from_wh(200.0, 200.0)) as page:
        with page.surface() as surface:
            # We want to define a pattern with a red rectangle on the top-left and a
            # blue rectangle on the bottom-right. Then we want to apply this pattern to
            # a rectangle that is rotated by 45 degrees in the center.

            # First, let's define the pattern by creating a new stream builder.
            stream_builder = StreamBuilder(Size.from_wh(20.0, 20.0))

            with stream_builder.surface() as pattern_surface:
                pattern_surface.set_fill(
                    Fill(
                        paint=Paint.from_rgb(color.rgb(255, 0, 0)),
                    )
                )
                # Draw the top-left rectangle.
                pattern_surface.draw_path(rect_to_path(0.0, 0.0, 10.0, 10.0))

                pattern_surface.set_fill(
                    Fill(
                        paint=Paint.from_rgb(color.rgb(0, 0, 255)),
                    )
                )
                # Draw the bottom-right rectangle.
                pattern_surface.draw_path(rect_to_path(10.0, 10.0, 20.0, 20.0))

            # Get the pattern stream
            pattern_stream = stream_builder.finish()

            # Define the actual pattern
            pattern = Pattern(stream=pattern_stream, width=20.0, height=20.0)

            # Now we draw the actual transformed rectangle.
            # First, push a transform so that the rectangle will be rotated.
            surface.push_transform(Transform.from_rotate_at(45.0, 100.0, 100.0))

            rect_path = rect_to_path(30.0, 30.0, 170.0, 170.0)

            surface.set_fill(
                Fill(
                    paint=Paint.from_pattern(pattern),
                )
            )
            # Draw the rectangle.
            surface.draw_path(rect_path)

            surface.set_fill(None)
            surface.set_stroke(
                Stroke(
                    paint=Paint.from_rgb(color.RgbColor.black()),
                )
            )
            # Let's also add a stroke, makes it look a bit nicer.
            surface.draw_path(rect_path)

            # Don't forget to pop! Each `push_` method must have a corresponding pop.
            surface.pop()

    pdf = document.finish()

    path = Path("stream_builder.pdf").absolute()
    print(f"Saved PDF to '{path}'")

    # Write the PDF to a file.
    with open(path, "wb") as f:
        f.write(pdf)


if __name__ == "__main__":
    main()
