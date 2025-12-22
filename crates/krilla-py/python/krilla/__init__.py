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
    # Core types
    Document,
    Page,
    Surface,
    PageSettings,
    # Geometry
    Point,
    Size,
    Rect,
    Transform,
    Path,
    PathBuilder,
    # Graphics
    Fill,
    Stroke,
    Paint,
    StrokeDash,
    # Gradients
    LinearGradient,
    RadialGradient,
    SweepGradient,
    Stop,
    # Enums
    BlendMode,
    FillRule,
    LineCap,
    LineJoin,
    SpreadMethod,
    MaskType,
    # Text
    Font,
    GlyphId,
    KrillaGlyph,
    # Streams and masks
    Stream,
    StreamBuilder,
    Mask,
    Pattern,
    # Configuration
    Configuration,
    PdfVersion,
    Validator,
    SerializeSettings,
    # Numeric
    NormalizedF32,
    # Accessibility/Tagging
    Location,
    ArtifactType,
    SpanTag,
    ContentTag,
    Identifier,
    # Exceptions
    KrillaError,
    FontError,
    ValidationError,
    ImageError,
    # Feature detection
    has_image_support,
    has_text_support,
)

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
    "KrillaGlyph",
    "TextDirection",
    # Image
    "Image",
    # Streams and masks
    "Stream",
    "StreamBuilder",
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
