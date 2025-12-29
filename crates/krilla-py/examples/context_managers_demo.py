"""Demonstration of Surface context managers for graphics state operations.

This example shows how to use the new context manager API for transforms,
blend modes, opacity, and other graphics state operations.
"""

from krilla import (
    BlendMode,
    Document,
    Fill,
    NormalizedF32,
    PageSettings,
    Paint,
    PathBuilder,
    Rect,
    Transform,
    color,
)


def main():
    doc = Document()

    with (
        doc.start_page_with(PageSettings.from_wh(400, 400)) as page,
        page.surface() as surface,
    ):
        # Set fill for drawing
        surface.set_fill(
            Fill(
                paint=Paint.from_rgb(color.rgb(255, 0, 0)),
                opacity=NormalizedF32.one(),
            )
        )

        # Old style - manual push/pop
        print("Drawing with manual push/pop style...")
        surface.push_transform(Transform.from_translate(50, 50))
        path1 = PathBuilder()
        path1.push_rect(Rect.from_xywh(0, 0, 50, 50))
        surface.draw_path(path1.finish())
        surface.pop()

        # New style - context manager (automatic pop)
        print("Drawing with context manager style...")
        with surface.transform(Transform.from_translate(150, 50)):
            path2 = PathBuilder()
            path2.push_rect(Rect.from_xywh(0, 0, 50, 50))
            surface.draw_path(path2.finish())
        # Transform automatically popped here!

        # Nested context managers (coalesced syntax)
        print("Drawing with nested context managers...")
        with (
            surface.transform(Transform.from_translate(250, 50)),
            surface.blend_mode(BlendMode.Multiply),
            surface.opacity(NormalizedF32(0.7)),
        ):
            surface.set_fill(
                Fill(
                    paint=Paint.from_rgb(color.rgb(0, 255, 0)),
                    opacity=NormalizedF32.one(),
                )
            )
            path3 = PathBuilder()
            path3.push_rect(Rect.from_xywh(0, 0, 50, 50))
            surface.draw_path(path3.finish())
        # All state automatically restored!

        # Complex nesting with rotation and scale (coalesced syntax)
        print("Drawing complex nested transformations...")
        with (
            surface.transform(Transform.from_translate(50, 150)),
            surface.transform(Transform.from_rotate(45)),
            surface.transform(Transform.from_scale(1.5, 1.5)),
        ):
            surface.set_fill(
                Fill(
                    paint=Paint.from_rgb(color.rgb(0, 0, 255)),
                    opacity=NormalizedF32.one(),
                )
            )
            path4 = PathBuilder()
            path4.push_rect(Rect.from_xywh(-25, -25, 50, 50))
            surface.draw_path(path4.finish())

        # Using isolated transparency groups (coalesced syntax)
        print("Drawing with isolated transparency group...")
        with (
            surface.transform(Transform.from_translate(200, 200)),
            surface.isolated(),
        ):
            # Everything in this group composites separately
            surface.set_fill(
                Fill(
                    paint=Paint.from_rgb(color.rgb(255, 255, 0)),
                    opacity=NormalizedF32(0.5),
                )
            )
            path5 = PathBuilder()
            path5.push_rect(Rect.from_xywh(0, 0, 60, 60))
            surface.draw_path(path5.finish())

            surface.set_fill(
                Fill(
                    paint=Paint.from_rgb(color.rgb(0, 255, 255)),
                    opacity=NormalizedF32(0.5),
                )
            )
            path6 = PathBuilder()
            path6.push_rect(Rect.from_xywh(30, 30, 60, 60))
            surface.draw_path(path6.finish())

    # Save the PDF
    pdf_bytes = doc.finish()
    with open("context_managers_demo.pdf", "wb") as f:
        f.write(pdf_bytes)

    print("\nPDF saved as context_managers_demo.pdf")
    print("The context manager API provides:")
    print("  ✓ Automatic cleanup (pop on exit)")
    print("  ✓ Exception safety (pop even on errors)")
    print("  ✓ Clean nested syntax")
    print("  ✓ Works with all graphics state operations")
    print("  ✓ Can mix with manual push/pop if needed")


if __name__ == "__main__":
    main()
