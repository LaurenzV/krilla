# Krilla Python Examples

This directory contains Python examples demonstrating how to use the krilla-py library for PDF creation.

## Examples

### empty_document.py
The most basic example - creates a PDF document with a single empty page.

**Run:**
```bash
uv run python examples/empty_document.py
```

### basic.py
Demonstrates fundamental PDF creation features including:
- Loading and using fonts
- Drawing text with different sizes and colors
- Creating shapes with PathBuilder (triangle)
- Using linear gradients
- Working with multiple pages

**Run:**
```bash
uv run python examples/basic.py
```

### simple_text.py
Shows how to use the simple text API for single-line text rendering:
- Drawing text with fills and strokes
- Using gradients as paint for text
- Rendering complex scripts (including RTL text like Arabic)
- Working with combining characters (zalgo text)

**Run:**
```bash
uv run python examples/simple_text.py
```

### stream_builder.py
Introduces the StreamBuilder API for creating reusable graphics patterns:
- Creating custom patterns with StreamBuilder
- Using patterns as fills
- Applying transformations to surfaces
- Combining fills and strokes

**Run:**
```bash
uv run python examples/stream_builder.py
```

### uharfbuzz_simple.py
A working example of using uharfbuzz for text shaping with krilla-py:
- Single-line text rendering
- Demonstrates the complete workflow: load font → shape with uharfbuzz → convert to KrillaGlyphs → render with draw_glyphs()
- Shows how to handle glyph positioning and text ranges
- Foundation for more sophisticated text layout

**Run:**
```bash
uv pip install uharfbuzz  # Install dependency first
uv run python examples/uharfbuzz_simple.py
```

### uharfbuzz_layout.py
Advanced text layout example replicating parley.rs capabilities using uharfbuzz:
- Multi-line text with simple line breaking
- Multi-styled text (bold characters 0-4, red characters 2-12)
- Style-per-glyph rendering with batching (mimics parley.rs pattern)
- Proper use of font metrics (ascent, descent) for line spacing
- Demonstrates everything needed for sophisticated text rendering in OCRmyPDF or similar applications

**Features demonstrated:**
- Word-by-word text shaping
- Simple line breaking algorithm
- Style range tracking and application
- Glyph batching (flush when style changes)
- Font metrics usage for proper baseline positioning

**Run:**
```bash
uv pip install uharfbuzz  # Install dependency first
uv run python examples/uharfbuzz_layout.py
```

## Prerequisites

Before running the examples, make sure you have:

1. Built the Python extension:
   ```bash
   uv run maturin develop
   ```

2. The font assets are available in the repository at `../../assets/fonts/`

## Output

Each example will create a PDF file in the current directory with the same name as the example (e.g., `basic.pdf`, `simple_text.pdf`, etc.).
