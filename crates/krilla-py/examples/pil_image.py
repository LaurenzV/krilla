"""Convert PIL/Pillow images to krilla Images.

This example demonstrates how to use the Image.from_pil() method to convert
Python Pillow (PIL) images into krilla images that can be embedded in PDFs.
"""

from pathlib import Path

from PIL import Image as PILImage
from PIL import ImageDraw

from krilla import Document, Image, PageSettings, Size


def main():
    # Create a PIL image with some content
    pil_img = PILImage.new("RGB", (400, 200), color=(255, 255, 255))
    draw = ImageDraw.Draw(pil_img)

    # Draw some shapes
    draw.rectangle([10, 10, 190, 90], fill=(255, 0, 0), outline=(0, 0, 0), width=2)
    draw.ellipse([210, 10, 390, 90], fill=(0, 255, 0), outline=(0, 0, 0), width=2)
    draw.polygon(
        [(100, 110), (200, 110), (150, 190)], fill=(0, 0, 255), outline=(0, 0, 0)
    )

    # Add text (using default font since we don't have a specific font file)
    draw.text((10, 100), "Hello from PIL!", fill=(0, 0, 0))
    draw.text((210, 100), "Converted to PDF!", fill=(0, 0, 0))

    # Convert PIL image to krilla Image
    krilla_img = Image.from_pil(pil_img, interpolate=True)

    print(f"Created krilla image: {krilla_img}")
    print(f"Size: {krilla_img.width} x {krilla_img.height} pixels")

    # Create a PDF and embed the image
    doc = Document()

    with doc.start_page_with(PageSettings.from_wh(450, 250)) as page:
        with page.surface() as surface:
            # Draw the image at its full size
            surface.draw_image(krilla_img, Size.from_wh(400, 200))

    pdf = doc.finish()

    # Save the PDF
    output_path = Path("pil_image.pdf").absolute()
    output_path.write_bytes(pdf)
    print(f"Saved PDF to '{output_path}'")


if __name__ == "__main__":
    main()
