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
    AssociationKind,
    BBox,
    BlendMode,
    BlockAlign,
    BorderStyle,
    ColumnDimensions,
    # Configuration
    Configuration,
    ContentTag,
    # Metadata/Interchange
    DateTime,
    # Core types
    Document,
    EmbeddedFile,
    # Graphics
    Fill,
    FillRule,
    # Text (low-level)
    Font,
    FontError,
    GlyphId,
    GlyphOrientationVertical,
    Identifier,
    ImageError,
    InlineAlign,
    # Exceptions
    KrillaError,
    # Gradients
    LinearGradient,
    LineCap,
    LineHeight,
    LineJoin,
    ListNumbering,
    # Accessibility/Tagging
    Location,
    Mask,
    MaskType,
    Metadata,
    MetadataTextDirection,
    MimeType,
    NaiveRgbColor,
    Node,
    # Numeric
    NormalizedF32,
    Outline,
    OutlineNode,
    Page,
    PageLayout,
    PageSettings,
    Paint,
    Path,
    PathBuilder,
    Pattern,
    PdfVersion,
    Placement,
    # Geometry
    Point,
    RadialGradient,
    Rect,
    SerializeSettings,
    SidesF32,
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
    TableHeaderScope,
    Tag,
    TagGroup,
    TagId,
    TagKind,
    TagTree,
    TextAlign,
    TextDecorationType,
    Transform,
    ValidationError,
    Validator,
    WritingMode,
    XyzDestination,
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
    "AssociationKind",
    "PageLayout",
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
    # Tag Tree (PDF/UA)
    "TagTree",
    "TagGroup",
    "Node",
    "TagKind",
    "Tag",
    # Tag Attribute Enums
    "ListNumbering",
    "TableHeaderScope",
    "Placement",
    "WritingMode",
    "BorderStyle",
    "TextAlign",
    "BlockAlign",
    "InlineAlign",
    "TextDecorationType",
    "GlyphOrientationVertical",
    "LineHeight",
    # Tag Attribute Types
    "TagId",
    "BBox",
    "NaiveRgbColor",
    "SidesF32",
    "ColumnDimensions",
    # Metadata/Interchange
    "DateTime",
    "Metadata",
    "MetadataTextDirection",
    "XyzDestination",
    "OutlineNode",
    "Outline",
    "MimeType",
    "EmbeddedFile",
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

# Load Surface extension methods for context managers
from krilla import surface_extensions  # noqa: F401
