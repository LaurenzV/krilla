"""Tests for Pythonic text API (Glyph class and conversion utilities)."""

import pytest

from krilla import Glyph, GlyphId
from krilla._krilla import char_to_byte_offset, char_range_to_bytes


class TestCharToByteConversion:
    """Test character-to-byte index conversion functions."""

    def test_ascii_text(self):
        """Test conversion with ASCII text (character == byte)."""
        text = "Hello"
        assert char_to_byte_offset(text, 0) == 0
        assert char_to_byte_offset(text, 1) == 1
        assert char_to_byte_offset(text, 4) == 4

    def test_unicode_accented(self):
        """Test conversion with accented characters (é is 2 bytes in UTF-8)."""
        text = "café"  # 4 characters, 5 bytes
        assert char_to_byte_offset(text, 0) == 0  # 'c'
        assert char_to_byte_offset(text, 1) == 1  # 'a'
        assert char_to_byte_offset(text, 2) == 2  # 'f'
        assert char_to_byte_offset(text, 3) == 3  # 'é' starts at byte 3
        # Character 4 doesn't exist, would raise ValueError

    def test_unicode_emoji(self):
        """Test conversion with emoji (4 bytes in UTF-8)."""
        text = "Hi👋"  # 3 characters, 6 bytes (👋 is 4 bytes)
        assert char_to_byte_offset(text, 0) == 0  # 'H'
        assert char_to_byte_offset(text, 1) == 1  # 'i'
        assert char_to_byte_offset(text, 2) == 2  # '👋' starts at byte 2

    def test_unicode_arabic(self):
        """Test conversion with Arabic text (multi-byte characters)."""
        text = "مرحبا"  # Arabic "marhaban"
        # Each Arabic character is typically 2 bytes in UTF-8
        assert char_to_byte_offset(text, 0) == 0
        assert char_to_byte_offset(text, 1) == 2
        assert char_to_byte_offset(text, 2) == 4

    def test_out_of_range(self):
        """Test that out-of-range indices raise ValueError."""
        text = "Hello"
        with pytest.raises(ValueError, match="out of range"):
            char_to_byte_offset(text, 10)

    def test_char_range_to_bytes_ascii(self):
        """Test range conversion with ASCII text."""
        text = "Hello"
        start, end = char_range_to_bytes(text, 0, 5)
        assert start == 0
        assert end == 5

    def test_char_range_to_bytes_unicode(self):
        """Test range conversion with Unicode text."""
        text = "café"  # 4 characters, 5 bytes
        start, end = char_range_to_bytes(text, 3, 4)
        assert start == 3  # 'é' starts at byte 3
        assert end == 5  # 'é' ends at byte 5 (2 bytes)

    def test_char_range_to_bytes_full_text(self):
        """Test range conversion for full text."""
        text = "Hello, café!"
        start, end = char_range_to_bytes(text, 0, len(text))
        assert start == 0
        assert end == len(text.encode("utf-8"))

    def test_char_range_to_bytes_out_of_range(self):
        """Test that out-of-range indices raise ValueError."""
        text = "Hello"
        with pytest.raises(ValueError, match="out of range"):
            char_range_to_bytes(text, 0, 10)


class TestGlyph:
    """Test the Pythonic Glyph class."""

    def test_from_shaper_ascii(self):
        """Test creating a Glyph from ASCII text."""
        glyph = Glyph.from_shaper(
            text="Hello",
            char_start=0,
            char_end=1,
            glyph_id=GlyphId(42),
            x_advance=0.5,
        )
        assert glyph.text == "H"
        assert glyph.glyph_id.to_u32() == 42
        assert glyph.x_advance == 0.5
        assert glyph.x_offset == 0.0
        assert glyph.y_offset == 0.0
        assert glyph.y_advance == 0.0

    def test_from_shaper_unicode(self):
        """Test creating a Glyph from Unicode text."""
        glyph = Glyph.from_shaper(
            text="café",
            char_start=3,
            char_end=4,
            glyph_id=GlyphId(100),
            x_advance=0.6,
        )
        assert glyph.text == "é"
        assert glyph.glyph_id.to_u32() == 100

    def test_from_shaper_ligature(self):
        """Test creating a Glyph for a ligature (multiple characters, one glyph)."""
        glyph = Glyph.from_shaper(
            text="office",
            char_start=2,
            char_end=4,  # "fi" ligature
            glyph_id=GlyphId(200),
            x_advance=0.8,
        )
        assert glyph.text == "fi"
        assert len(glyph.text) == 2  # Two characters

    def test_from_shaper_emoji(self):
        """Test creating a Glyph for an emoji."""
        glyph = Glyph.from_shaper(
            text="Hi👋",
            char_start=2,
            char_end=3,
            glyph_id=GlyphId(300),
            x_advance=1.0,
        )
        assert glyph.text == "👋"

    def test_from_shaper_with_positioning(self):
        """Test creating a Glyph with all positioning parameters."""
        glyph = Glyph.from_shaper(
            text="Test",
            char_start=0,
            char_end=1,
            glyph_id=GlyphId(1),
            x_advance=0.5,
            x_offset=0.1,
            y_offset=-0.2,
            y_advance=0.0,
        )
        assert glyph.x_advance == 0.5
        assert glyph.x_offset == 0.1
        assert glyph.y_offset == -0.2
        assert glyph.y_advance == 0.0

    def test_to_krilla_glyph_ascii(self):
        """Test conversion to _KrillaGlyph with ASCII text."""
        glyph = Glyph.from_shaper(
            text="Hello",
            char_start=0,
            char_end=1,
            glyph_id=GlyphId(1),
            x_advance=0.5,
        )

        krilla_glyph = glyph._to_krilla_glyph()
        assert krilla_glyph.text_start == 0
        assert krilla_glyph.text_end == 1
        assert krilla_glyph.glyph_id.to_u32() == 1

    def test_to_krilla_glyph_unicode(self):
        """Test conversion to _KrillaGlyph with Unicode text (byte conversion)."""
        glyph = Glyph.from_shaper(
            text="café",
            char_start=3,
            char_end=4,
            glyph_id=GlyphId(1),
            x_advance=0.5,
        )

        krilla_glyph = glyph._to_krilla_glyph()
        # 'é' is at character 3, byte 3, and is 2 bytes long
        assert krilla_glyph.text_start == 3
        assert krilla_glyph.text_end == 5  # Not 4! é is 2 bytes

    def test_to_krilla_glyph_caching(self):
        """Test that _KrillaGlyph conversion is cached."""
        glyph = Glyph.from_shaper(
            text="Test",
            char_start=0,
            char_end=1,
            glyph_id=GlyphId(1),
            x_advance=0.5,
        )

        krilla_glyph1 = glyph._to_krilla_glyph()
        krilla_glyph2 = glyph._to_krilla_glyph()

        # Should be the same object (cached)
        assert krilla_glyph1 is krilla_glyph2

    def test_batch_from_shaper(self):
        """Test creating multiple glyphs at once."""
        text = "Hello"
        glyph_data = [
            {
                "char_start": 0,
                "char_end": 1,
                "glyph_id": GlyphId(1),
                "x_advance": 0.5,
            },
            {
                "char_start": 1,
                "char_end": 2,
                "glyph_id": GlyphId(2),
                "x_advance": 0.6,
            },
            {
                "char_start": 2,
                "char_end": 3,
                "glyph_id": GlyphId(3),
                "x_advance": 0.5,
            },
        ]

        glyphs = Glyph.batch_from_shaper(text, glyph_data)
        assert len(glyphs) == 3
        assert glyphs[0].text == "H"
        assert glyphs[1].text == "e"
        assert glyphs[2].text == "l"

    def test_repr(self):
        """Test string representation of Glyph."""
        glyph = Glyph.from_shaper(
            text="Test",
            char_start=0,
            char_end=1,
            glyph_id=GlyphId(42),
            x_advance=0.5,
        )
        repr_str = repr(glyph)
        assert "Glyph" in repr_str
        assert "42" in repr_str  # glyph ID
        assert "'T'" in repr_str  # text
        assert "0.500" in repr_str  # x_advance


class TestGlyphsToText:
    """Test the glyphs_to_text helper function."""

    def test_simple_text(self):
        """Test reconstructing simple text from glyphs."""
        from krilla import glyphs_to_text

        text = "Hello"
        glyphs = [
            Glyph.from_shaper(
                text=text,
                char_start=i,
                char_end=i + 1,
                glyph_id=GlyphId(i),
                x_advance=0.5,
            )
            for i in range(len(text))
        ]

        reconstructed = glyphs_to_text(glyphs)
        assert reconstructed == "Hello"

    def test_unicode_text(self):
        """Test reconstructing Unicode text from glyphs."""
        from krilla import glyphs_to_text

        text = "café"
        glyphs = [
            Glyph.from_shaper(
                text=text,
                char_start=i,
                char_end=i + 1,
                glyph_id=GlyphId(i),
                x_advance=0.5,
            )
            for i in range(len(text))
        ]

        reconstructed = glyphs_to_text(glyphs)
        assert reconstructed == "café"

    def test_empty_glyphs(self):
        """Test with empty glyph list."""
        from krilla import glyphs_to_text

        assert glyphs_to_text([]) == ""

    def test_ligature(self):
        """Test with ligatures (multiple characters in one glyph)."""
        from krilla import glyphs_to_text

        glyphs = [
            Glyph.from_shaper(
                text="office",
                char_start=0,
                char_end=2,
                glyph_id=GlyphId(1),
                x_advance=0.5,
            ),
            Glyph.from_shaper(
                text="office",
                char_start=2,
                char_end=4,  # "fi" ligature
                glyph_id=GlyphId(2),
                x_advance=0.8,
            ),
            Glyph.from_shaper(
                text="office",
                char_start=4,
                char_end=6,
                glyph_id=GlyphId(3),
                x_advance=0.5,
            ),
        ]

        reconstructed = glyphs_to_text(glyphs)
        assert reconstructed == "office"
