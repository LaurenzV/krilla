"""
Krilla - Python bindings for the krilla PDF library.

A high-level library for creating PDF files with support for:
- Filling and stroking paths
- Affine transformations
- Alpha and luminosity masks
- Clip paths and blend modes
- Text rendering with OpenType fonts
- Linear, radial and sweep gradients
- Embedding bitmap images
"""

from krilla._krilla import (
    ArtifactType,
    # Enums
    BlendMode,
    # Configuration
    Configuration,
    ContentTag,
    # Core types
    Document,
    # Graphics
    Fill,
    FillRule,
    # Text (low-level)
    Font,
    FontError,
    GlyphId,
    Identifier,
    ImageError,
    # Exceptions
    KrillaError,
    # Gradients
    LinearGradient,
    LineCap,
    LineJoin,
    # Accessibility/Tagging
    Location,
    Mask,
    MaskType,
    # Numeric
    NormalizedF32,
    Page,
    PageSettings,
    Paint,
    Path,
    PathBuilder,
    Pattern,
    PdfVersion,
    # Geometry
    Point,
    RadialGradient,
    Rect,
    SerializeSettings,
    Size,
    SpanTag,
    SpreadMethod,
    Stop,
    # Streams and masks
    Stream,
    StreamBuilder,
    StreamSurface,
    Stroke,
    StrokeDash,
    Surface,
    SweepGradient,
    Transform,
    ValidationError,
    Validator,
    # Feature detection
    has_image_support,
    has_text_support,
)

# High-level Pythonic text API
from krilla.text import Glyph, glyphs_to_text

# Conditionally import feature-gated types
try:
    from krilla._krilla import Image
except ImportError:
    Image = None  # type: ignore

try:
    from krilla._krilla import TextDirection
except ImportError:
    TextDirection = None  # type: ignore

# Re-export color module
from krilla._krilla import color

__all__ = [
    # Core types
    "Document",
    "Page",
    "Surface",
    "PageSettings",
    # Geometry
    "Point",
    "Size",
    "Rect",
    "Transform",
    "Path",
    "PathBuilder",
    # Graphics
    "Fill",
    "Stroke",
    "Paint",
    "StrokeDash",
    # Gradients
    "LinearGradient",
    "RadialGradient",
    "SweepGradient",
    "Stop",
    # Enums
    "BlendMode",
    "FillRule",
    "LineCap",
    "LineJoin",
    "SpreadMethod",
    "MaskType",
    # Text
    "Font",
    "GlyphId",
    "Glyph",  # Pythonic high-level API
    "glyphs_to_text",  # Helper function
    "TextDirection",
    # Image
    "Image",
    # Streams and masks
    "Stream",
    "StreamBuilder",
    "StreamSurface",
    "Mask",
    "Pattern",
    # Configuration
    "Configuration",
    "PdfVersion",
    "Validator",
    "SerializeSettings",
    # Numeric
    "NormalizedF32",
    # Accessibility/Tagging
    "Location",
    "ArtifactType",
    "SpanTag",
    "ContentTag",
    "Identifier",
    # Exceptions
    "KrillaError",
    "FontError",
    "ValidationError",
    "ImageError",
    # Feature detection
    "has_image_support",
    "has_text_support",
    # Submodules
    "color",
]

__version__ = "0.1.0"
