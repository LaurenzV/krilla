# Examples

A collection of examples demonstrating various krilla features.

## Empty Document

The simplest possible PDF - an empty document with one blank page.

```{literalinclude} ../../../examples/empty_document.py
:language: python
:linenos:
```

## Basic Drawing

Creating a PDF with text and a gradient-filled triangle.

```{literalinclude} ../../../examples/basic.py
:language: python
:linenos:
```

## Simple Text

Text rendering using the simple-text feature for easy text drawing.

```{literalinclude} ../../../examples/simple_text.py
:language: python
:linenos:
```

## Stream Builder

Advanced graphics using StreamBuilder for creating patterns and masks.

```{literalinclude} ../../../examples/stream_builder.py
:language: python
:linenos:
```

## PIL Image Embedding

Embedding images from PIL/Pillow (requires raster-images feature).

```{literalinclude} ../../../examples/pil_image.py
:language: python
:linenos:
```

## Context Managers

Demonstrating Pythonic context managers for graphics state operations like transforms, blend modes, clipping, and masking.

```{literalinclude} ../../../examples/context_managers_demo.py
:language: python
:linenos:
```

## HarfBuzz Simple

Basic text shaping with HarfBuzz for advanced typography.

```{literalinclude} ../../../examples/uharfbuzz_simple.py
:language: python
:linenos:
```

## HarfBuzz Multi-line Layout

Advanced multi-line text layout using HarfBuzz with line breaking and alignment.

```{literalinclude} ../../../examples/uharfbuzz_layout.py
:language: python
:linenos:
```
