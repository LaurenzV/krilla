# krilla - Python PDF Generation

Krilla is a high-level library for programmatic PDF creation, written mostly in Rust. This documentation describes Krilla's Python bindings, which are implemented in the krilla-py Rust crate.

Krilla provides powerful graphics primitives like fills, strokes, gradients, glyphs, and images while abstracting away PDF format complexity.

## Quick Start

```python
from krilla import Document, PageSettings, PathBuilder, Fill
from krilla.color import rgb

# Create a document
doc = Document()

# Add a page
with (
    doc.start_page(PageSettings.from_wh(200, 200)) as page,
    page.surface() as surface,
):
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

- **High-level API** - Document → Page → Surface ownership chain with automatic state management
- **Context managers** - Pythonic `with` statements for proper resource cleanup
- **Full color support** - RGB, Grayscale, CMYK color spaces
- **Path graphics** - Fills, strokes, gradients (linear, radial, sweep)
- **Text rendering** - OpenType fonts with simple-text feature
- **Image embedding** - PNG, JPEG, GIF, WebP support with raster-images feature
- **Standards compliance** - PDF/A and PDF/UA validation support

## Installation

```bash
uv add krilla
```

or

```bash
pip install krilla
```

## Documentation

```{toctree}
:maxdepth: 2
:caption: User Guide

guides/quickstart
guides/architecture
```

```{toctree}
:maxdepth: 2
:caption: API Reference

api/index
```

```{toctree}
:maxdepth: 1
:caption: Examples

examples/index
```

## Indices

* {ref}`genindex`
* {ref}`modindex`

## License

MIT OR Apache-2.0
