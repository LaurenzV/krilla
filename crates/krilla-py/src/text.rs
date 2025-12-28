//! Text and font types for krilla Python bindings.

use pyo3::prelude::*;
use std::ops::Range;

/// A font for rendering text.
///
/// Fonts are loaded from font data (TTF, OTF, etc.) and are cheap to clone.
#[pyclass]
#[derive(Clone)]
pub struct Font {
    pub(crate) inner: krilla::text::Font,
}

#[pymethods]
impl Font {
    /// Load a font from bytes.
    ///
    /// Args:
    ///     data: Font file contents (TTF, OTF, TTC, etc.)
    ///     index: Font index in the file (0 for single-font files)
    ///
    /// Returns:
    ///     A Font object, or None if the font could not be loaded.
    #[staticmethod]
    fn new(data: &[u8], index: u32) -> Option<Self> {
        krilla::text::Font::new(data.to_vec().into(), index).map(|f| Font { inner: f })
    }

    /// Load a variable font with specific variation coordinates.
    ///
    /// Args:
    ///     data: Font file contents
    ///     index: Font index in the file
    ///     variations: List of (tag, value) tuples for variation axes
    ///
    /// Returns:
    ///     A Font object, or None if the font could not be loaded.
    #[staticmethod]
    fn new_variable(data: &[u8], index: u32, variations: Vec<(String, f32)>) -> Option<Self> {
        let coords: Vec<(krilla::text::Tag, f32)> = variations
            .iter()
            .filter_map(|(tag, value)| krilla::text::Tag::try_from_str(tag).map(|t| (t, *value)))
            .collect();

        krilla::text::Font::new_variable(data.to_vec().into(), index, &coords).map(|f| Font { inner: f })
    }

    /// Get the units per em of the font.
    fn units_per_em(&self) -> f32 {
        self.inner.units_per_em()
    }

    /// Get the font ascent (above baseline) in font units.
    fn ascent(&self) -> f32 {
        self.inner.ascent()
    }

    /// Get the font descent (below baseline) in font units.
    /// Note: This value is typically negative.
    fn descent(&self) -> f32 {
        self.inner.descent()
    }

    /// Get the cap height of the font in font units, if available.
    fn cap_height(&self) -> Option<f32> {
        self.inner.cap_height()
    }

    fn __repr__(&self) -> String {
        "Font(...)".to_string()
    }
}

impl Font {
    pub fn into_inner(self) -> krilla::text::Font {
        self.inner
    }
}

/// A glyph identifier.
#[pyclass(frozen)]
#[derive(Clone, Copy)]
pub struct GlyphId {
    inner: krilla::text::GlyphId,
}

#[pymethods]
impl GlyphId {
    /// Create a new glyph ID.
    #[new]
    fn new(id: u32) -> Self {
        GlyphId {
            inner: krilla::text::GlyphId::new(id),
        }
    }

    /// Get the numeric value of the glyph ID.
    fn to_u32(&self) -> u32 {
        self.inner.to_u32()
    }

    fn __repr__(&self) -> String {
        format!("GlyphId({})", self.inner.to_u32())
    }

    fn __eq__(&self, other: &GlyphId) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

impl GlyphId {
    pub fn into_inner(self) -> krilla::text::GlyphId {
        self.inner
    }
}

/// A glyph with positioning information for low-level text rendering.
///
/// This is an internal type. Python users should use the `Glyph` class instead,
/// which provides a more Pythonic API with character-based indexing.
#[pyclass(name = "_KrillaGlyph")]
#[derive(Clone)]
pub struct _KrillaGlyph {
    /// The glyph ID.
    #[pyo3(get, set)]
    pub glyph_id: GlyphId,
    /// Start of text range this glyph represents.
    #[pyo3(get, set)]
    pub text_start: usize,
    /// End of text range this glyph represents.
    #[pyo3(get, set)]
    pub text_end: usize,
    /// Horizontal advance (normalized by units_per_em).
    #[pyo3(get, set)]
    pub x_advance: f32,
    /// Horizontal offset (normalized by units_per_em).
    #[pyo3(get, set)]
    pub x_offset: f32,
    /// Vertical offset (normalized by units_per_em).
    #[pyo3(get, set)]
    pub y_offset: f32,
    /// Vertical advance (normalized by units_per_em).
    #[pyo3(get, set)]
    pub y_advance: f32,
}

#[pymethods]
impl _KrillaGlyph {
    /// Create a new glyph with positioning.
    #[new]
    #[pyo3(signature = (glyph_id, x_advance, text_start, text_end, x_offset=0.0, y_offset=0.0, y_advance=0.0))]
    fn new(
        glyph_id: GlyphId,
        x_advance: f32,
        text_start: usize,
        text_end: usize,
        x_offset: f32,
        y_offset: f32,
        y_advance: f32,
    ) -> Self {
        _KrillaGlyph {
            glyph_id,
            text_start,
            text_end,
            x_advance,
            x_offset,
            y_offset,
            y_advance,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "_KrillaGlyph(id={}, x_advance={})",
            self.glyph_id.to_u32(),
            self.x_advance
        )
    }
}

/// Wrapper to implement the Glyph trait for Python glyphs.
pub struct GlyphWrapper {
    pub glyph_id: krilla::text::GlyphId,
    pub text_range: Range<usize>,
    pub x_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub y_advance: f32,
}

impl krilla::text::Glyph for GlyphWrapper {
    fn glyph_id(&self) -> krilla::text::GlyphId {
        self.glyph_id
    }

    fn text_range(&self) -> Range<usize> {
        self.text_range.clone()
    }

    fn x_advance(&self, size: f32) -> f32 {
        self.x_advance * size
    }

    fn x_offset(&self, size: f32) -> f32 {
        self.x_offset * size
    }

    fn y_offset(&self, size: f32) -> f32 {
        self.y_offset * size
    }

    fn y_advance(&self, size: f32) -> f32 {
        self.y_advance * size
    }

    fn location(&self) -> Option<krilla::surface::Location> {
        None
    }
}

impl From<&_KrillaGlyph> for GlyphWrapper {
    fn from(g: &_KrillaGlyph) -> Self {
        GlyphWrapper {
            glyph_id: g.glyph_id.into_inner(),
            text_range: g.text_start..g.text_end,
            x_advance: g.x_advance,
            x_offset: g.x_offset,
            y_offset: g.y_offset,
            y_advance: g.y_advance,
        }
    }
}

/// Text direction for text layout.
#[cfg(feature = "simple-text")]
#[pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    /// Auto-detect text direction.
    Auto,
    /// Left-to-right.
    LeftToRight,
    /// Right-to-left.
    RightToLeft,
}

#[cfg(feature = "simple-text")]
impl TextDirection {
    pub fn into_inner(self) -> krilla::text::TextDirection {
        match self {
            TextDirection::Auto => krilla::text::TextDirection::Auto,
            TextDirection::LeftToRight => krilla::text::TextDirection::LeftToRight,
            TextDirection::RightToLeft => krilla::text::TextDirection::RightToLeft,
        }
    }
}

/// Convert a character index to a byte offset in a UTF-8 string.
///
/// Args:
///     text: The UTF-8 string
///     char_index: The character index (0-based)
///
/// Returns:
///     The byte offset where the character starts
///
/// Raises:
///     ValueError: If char_index is out of range
#[pyfunction]
pub fn char_to_byte_offset(text: &str, char_index: usize) -> PyResult<usize> {
    text.char_indices()
        .nth(char_index)
        .map(|(byte_idx, _)| byte_idx)
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Character index {} out of range for text with {} characters",
                char_index,
                text.chars().count()
            ))
        })
}

/// Convert a character range to a byte range in a UTF-8 string.
///
/// This is the primary conversion function used when creating glyphs from
/// text shaper output (like HarfBuzz), which returns character-based clusters.
///
/// Args:
///     text: The UTF-8 string
///     char_start: The starting character index (0-based)
///     char_end: The ending character index (exclusive)
///
/// Returns:
///     A tuple of (byte_start, byte_end) offsets
///
/// Raises:
///     ValueError: If indices are out of range
///
/// Example:
///     >>> char_range_to_bytes("café", 3, 4)
///     (3, 5)  # 'é' is at byte 3-5 (2 bytes in UTF-8)
#[pyfunction]
pub fn char_range_to_bytes(
    text: &str,
    char_start: usize,
    char_end: usize,
) -> PyResult<(usize, usize)> {
    let char_count = text.chars().count();

    // Validate indices
    if char_start > char_count {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "char_start {} out of range for text with {} characters",
            char_start, char_count
        )));
    }
    if char_end > char_count {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "char_end {} out of range for text with {} characters",
            char_end, char_count
        )));
    }

    let byte_start = if char_start == 0 {
        0
    } else {
        char_to_byte_offset(text, char_start)?
    };

    let byte_end = if char_end >= char_count {
        text.len()
    } else {
        char_to_byte_offset(text, char_end)?
    };

    Ok((byte_start, byte_end))
}
