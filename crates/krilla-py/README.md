# krilla-py

Python bindings for the [krilla](https://github.com/LaurenzV/krilla) PDF library.

## Installation

```bash
pip install krilla
```

## Usage

```python
from krilla import Document, PageSettings, PathBuilder, Fill
from krilla.color import rgb

# Create a document
doc = Document()

# Add a page
with doc.start_page(PageSettings.from_wh(200, 200)) as page:
    with page.surface() as surface:
        # Create a path
        pb = PathBuilder()
        pb.move_to(100, 20)
        pb.line_to(160, 160)
        pb.line_to(40, 160)
        pb.close()
        path = pb.finish()

        # Set fill color and draw
        surface.set_fill(Fill(paint=rgb(255, 0, 0)))
        surface.draw_path(path)

# Export PDF
pdf_bytes = doc.finish()
with open("output.pdf", "wb") as f:
    f.write(pdf_bytes)
```

## Features

- High-level PDF generation API
- Context manager support for pages and surfaces
- Full color support (RGB, Grayscale, CMYK)
- Path drawing with fills and strokes
- Linear, radial, and sweep gradients
- Text rendering (with `simple-text` feature)
- Image embedding (with `raster-images` feature)
- PDF/A and PDF/UA validation support

## License

MIT OR Apache-2.0
