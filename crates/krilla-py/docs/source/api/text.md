# Text

Text rendering and font handling.

## Font

```{eval-rst}
.. autoclass:: krilla.Font
   :members:
   :undoc-members:
   :show-inheritance:
```

Represents an OpenType font (TTF or CFF). Supports color fonts (COLR, SVG, sbix, CBDT).

## Glyph

```{eval-rst}
.. autoclass:: krilla.Glyph
   :members:
   :undoc-members:
   :show-inheritance:
```

High-level Python class for working with glyphs using character-based indexing (as opposed to byte-based).

## GlyphId

```{eval-rst}
.. autoclass:: krilla.GlyphId
   :members:
   :undoc-members:
   :show-inheritance:
```

Represents a glyph identifier in a font.

## Utility Functions

```{eval-rst}
.. autofunction:: krilla.glyphs_to_text
```

Helper function to convert a list of glyphs back to text.
