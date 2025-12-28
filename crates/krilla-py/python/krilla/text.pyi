"""Type stubs for krilla.text module."""

from __future__ import annotations

from collections.abc import Sequence

from krilla._krilla import GlyphId, _KrillaGlyph

class Glyph:
    """A glyph with positioning information (Pythonic API)."""

    glyph_id: GlyphId
    text: str
    x_advance: float
    x_offset: float
    y_offset: float
    y_advance: float
    # Private attributes for internal use
    _source_text: str | None
    _char_start: int | None
    _char_end: int | None

    def __init__(
        self,
        glyph_id: GlyphId,
        text: str,
        x_advance: float,
        x_offset: float = 0.0,
        y_offset: float = 0.0,
        y_advance: float = 0.0,
    ) -> None: ...
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
    ) -> Glyph: ...
    @classmethod
    def batch_from_shaper(
        cls,
        text: str,
        glyph_data: list[dict],
    ) -> list[Glyph]: ...
    def _to_krilla_glyph(self) -> _KrillaGlyph: ...

def glyphs_to_text(glyphs: Sequence[Glyph]) -> str: ...
