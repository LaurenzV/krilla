# Quick Start Guide

Get started with krilla for PDF generation in Python.

## Installation

Install krilla using pip:

```bash
pip install krilla
```

## Basic Usage

### Creating Your First PDF

```python
from krilla import Document, PageSettings, PathBuilder, Fill
from krilla.color import rgb

# Create a document
doc = Document()

# Add a page with custom dimensions (200x200 points)
with doc.start_page(PageSettings.from_wh(200, 200)) as page:
    with page.surface() as surface:
        # Create a triangular path
        pb = PathBuilder()
        pb.move_to(100, 20)   # Top point
        pb.line_to(160, 160)  # Bottom right
        pb.line_to(40, 160)   # Bottom left
        pb.close()            # Close the path
        path = pb.finish()

        # Set fill color to red and draw
        surface.set_fill(Fill(paint=rgb(255, 0, 0)))
        surface.draw_path(path)

# Finish and save the PDF
pdf_bytes = doc.finish()
with open("triangle.pdf", "wb") as f:
    f.write(pdf_bytes)
```

## Key Concepts

### Document → Page → Surface Hierarchy

Krilla uses a strict ownership chain:

- **Document** - The top-level container for the entire PDF
- **Page** - Individual pages within the document
- **Surface** - Drawing surface for graphics operations on a page

All three support Python context managers (`with` statements) for automatic resource cleanup.

### Context Managers

Krilla makes heavy use of context managers for safety:

```python
# Document context
with doc.start_page(PageSettings.from_wh(200, 200)) as page:
    # Page context - automatically finished when exiting
    with page.surface() as surface:
        # Surface context - automatically finished when exiting
        surface.draw_path(path)
    # Surface is finished here
# Page is finished here
```

### Graphics State

The surface provides methods for setting graphics state:

```python
with page.surface() as surface:
    # Set fill and stroke
    surface.set_fill(Fill(paint=rgb(255, 0, 0)))
    surface.set_stroke(Stroke(paint=rgb(0, 0, 255), width=2.0))

    # Apply transforms
    with surface.transform(Transform.from_translate(10, 20)):
        surface.draw_path(path)
    # Transform automatically popped

    # Set blend mode
    with surface.blend_mode(BlendMode.Multiply):
        surface.draw_path(path)
```

## Common Tasks

### Working with Colors

```python
from krilla.color import rgb, luma, cmyk

# RGB color (0-255)
red = rgb(255, 0, 0)

# Grayscale (0-255)
gray = luma(128)

# CMYK for print (0-255)
cyan = cmyk(255, 0, 0, 0)
```

### Creating Gradients

```python
from krilla import LinearGradient, RadialGradient, Stop, Paint

# Linear gradient
gradient = LinearGradient(
    start=Point.from_xy(0, 0),
    end=Point.from_xy(100, 100),
    stops=[
        Stop(offset=0.0, color=rgb(255, 0, 0)),
        Stop(offset=1.0, color=rgb(0, 0, 255))
    ]
)

surface.set_fill(Fill(paint=Paint.from_linear_gradient(gradient)))
```

### Drawing Text

```python
from krilla import Font

# Load a font
font = Font.from_file("path/to/font.ttf")

# Draw text (requires simple-text feature)
surface.draw_simple_text("Hello, World!", font, 12.0, rgb(0, 0, 0))
```

### Embedding Images

```python
from krilla import Image

# Load an image (requires raster-images feature)
image = Image.from_file("photo.jpg")

# Draw at position with size
surface.draw_image(image, Rect.from_xywh(10, 10, 100, 100))
```

## Next Steps

- Explore the {doc}`../api/index` for detailed API documentation
- Check out {doc}`../examples/index` for more complex use cases
- Read {doc}`architecture` to understand the design
