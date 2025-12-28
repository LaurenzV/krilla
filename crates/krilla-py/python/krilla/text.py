"""High-level Pythonic text handling for krilla.

This module provides the `Glyph` class, which offers a character-based API
for working with text glyphs. Unlike the low-level `_KrillaGlyph` class which
requires byte offsets, `Glyph` works with natural Python string indexing.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from collections.abc import Sequence

from krilla._krilla import _KrillaGlyph, GlyphId, char_range_to_bytes


@dataclass
class Glyph:
    """A glyph with positioning information (Pythonic API).

    This class provides a high-level, character-based interface for working
    with glyphs. Character indices are used instead of byte offsets, making
    it natural to work with Python strings and text shapers like HarfBuzz.

    Attributes:
        glyph_id: The glyph identifier from the font.
        text: The text substring this glyph represents (e.g., "é", "fi" for ligatures).
        x_advance: Horizontal advance (normalized by font units_per_em).
        x_offset: Horizontal offset (normalized by font units_per_em).
        y_offset: Vertical offset (normalized by font units_per_em).
        y_advance: Vertical advance (normalized by font units_per_em).

    Example:
        >>> # From HarfBuzz shaping output
        >>> info = buf.glyph_infos[0]
        >>> pos = buf.glyph_positions[0]
        >>> glyph = Glyph.from_shaper(
        ...     text="Hello",
        ...     char_start=info.cluster,
        ...     char_end=info.cluster + 1,
        ...     glyph_id=GlyphId(info.codepoint),
        ...     x_advance=pos.x_advance / font.units_per_em(),
        ... )
        >>> print(glyph.text)  # "H"
    """

    glyph_id: GlyphId
    text: str
    x_advance: float
    x_offset: float = 0.0
    y_offset: float = 0.0
    y_advance: float = 0.0

    # Internal fields for byte offset conversion
    _source_text: str | None = field(default=None, repr=False, compare=False)
    _char_start: int | None = field(default=None, repr=False, compare=False)
    _char_end: int | None = field(default=None, repr=False, compare=False)
    _cached_krilla_glyph: _KrillaGlyph | None = field(
        default=None, repr=False, compare=False
    )

    @classmethod
    def from_shaper(
        cls,
        text: str,
        char_start: int,
        char_end: int,
        glyph_id: GlyphId,
        x_advance: float,
        x_offset: float = 0.0,
        y_offset: float = 0.0,
        y_advance: float = 0.0,
    ) -> Glyph:
        """Create a Glyph from text shaper output (character indices).

        This is the primary method for creating glyphs when using text shaping
        libraries like HarfBuzz, which return character-based cluster indices.

        Args:
            text: The full text string being shaped.
            char_start: Character index where this glyph's text starts (inclusive).
            char_end: Character index where this glyph's text ends (exclusive).
            glyph_id: The glyph ID from the font.
            x_advance: Horizontal advance (normalized by font units_per_em).
            x_offset: Horizontal offset (normalized by font units_per_em).
            y_offset: Vertical offset (normalized by font units_per_em).
            y_advance: Vertical advance (normalized by font units_per_em).

        Returns:
            A new Glyph instance with the text substring and positioning.

        Example:
            >>> # HarfBuzz returns character indices
            >>> text = "café"
            >>> glyph = Glyph.from_shaper(
            ...     text=text,
            ...     char_start=3,  # Character 3 (the 'é')
            ...     char_end=4,    # Character 4 (end)
            ...     glyph_id=GlyphId(42),
            ...     x_advance=0.5,
            ... )
            >>> glyph.text  # "é"

        Note:
            Handles ligatures correctly (e.g., "fi" → single glyph with text="fi").
            Also handles combining characters (e.g., "e" + combining accent → "é").
        """
        # Extract the substring this glyph represents
        glyph_text = text[char_start:char_end]

        return cls(
            glyph_id=glyph_id,
            text=glyph_text,
            x_advance=x_advance,
            x_offset=x_offset,
            y_offset=y_offset,
            y_advance=y_advance,
            _source_text=text,
            _char_start=char_start,
            _char_end=char_end,
        )

    @classmethod
    def batch_from_shaper(
        cls,
        text: str,
        glyph_data: list[dict],
    ) -> list[Glyph]:
        """Create multiple glyphs from shaper output.

        Convenience method for creating many glyphs at once from a list of
        shaper output data.

        Args:
            text: The full text string being shaped.
            glyph_data: List of dictionaries, each containing:
                - char_start: int - Starting character index
                - char_end: int - Ending character index
                - glyph_id: GlyphId - The glyph identifier
                - x_advance: float - Horizontal advance
                - x_offset: float (optional) - Horizontal offset
                - y_offset: float (optional) - Vertical offset
                - y_advance: float (optional) - Vertical advance

        Returns:
            List of Glyph instances.

        Example:
            >>> glyphs = Glyph.batch_from_shaper(
            ...     text="Hello",
            ...     glyph_data=[
            ...         {
            ...             "char_start": 0,
            ...             "char_end": 1,
            ...             "glyph_id": GlyphId(72),
            ...             "x_advance": 0.5,
            ...         },
            ...         # ... more glyphs
            ...     ],
            ... )
        """
        return [cls.from_shaper(text=text, **data) for data in glyph_data]

    def _to_krilla_glyph(self) -> _KrillaGlyph:
        """Convert to internal _KrillaGlyph with byte offsets.

        This method is called internally by krilla when drawing glyphs.
        It converts the character-based indices to byte offsets required
        by the Rust PDF library.

        The result is cached to avoid recomputation.

        Returns:
            A _KrillaGlyph with byte-based text_start and text_end.

        Raises:
            ValueError: If this Glyph wasn't created via from_shaper() and
                lacks the required source text information.
        """
        if self._cached_krilla_glyph is not None:
            return self._cached_krilla_glyph

        # Need source text and indices to compute byte offsets
        if (
            self._source_text is None
            or self._char_start is None
            or self._char_end is None
        ):
            raise ValueError(
                "Cannot convert to _KrillaGlyph: missing source text information. "
                "Use Glyph.from_shaper() to create Glyphs with proper "
                "conversion support."
            )

        # Convert character indices to byte offsets
        byte_start, byte_end = char_range_to_bytes(
            self._source_text,
            self._char_start,
            self._char_end,
        )

        # Create and cache the internal glyph
        self._cached_krilla_glyph = _KrillaGlyph(
            glyph_id=self.glyph_id,
            x_advance=self.x_advance,
            text_start=byte_start,
            text_end=byte_end,
            x_offset=self.x_offset,
            y_offset=self.y_offset,
            y_advance=self.y_advance,
        )

        return self._cached_krilla_glyph

    def __repr__(self) -> str:
        """Return a string representation of this Glyph."""
        return (
            f"Glyph(id={self.glyph_id.to_u32()}, "
            f"text={self.text!r}, "
            f"x_advance={self.x_advance:.3f})"
        )


def glyphs_to_text(glyphs: Sequence[Glyph]) -> str:
    """Reconstruct text from a sequence of glyphs.

    Concatenates the text substrings from each glyph to reconstruct
    the original text.

    Args:
        glyphs: Sequence of Glyph objects.

    Returns:
        The reconstructed text string.

    Note:
        This may not perfectly reconstruct the original text if glyphs
        are out of order or have gaps between them.

    Example:
        >>> glyphs = [...]  # Shaped from "Hello"
        >>> glyphs_to_text(glyphs)
        "Hello"
    """
    return "".join(g.text for g in glyphs)
