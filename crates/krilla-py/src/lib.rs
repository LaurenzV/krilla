//! Python bindings for the krilla PDF library.
//!
//! This crate provides PyO3-based Python bindings for krilla, a high-level
//! Rust library for creating PDF files.

use pyo3::prelude::*;

mod color;
mod config;
mod document;
mod enums;
mod error;
mod geometry;
#[cfg(feature = "raster-images")]
mod image;
mod interchange;
mod mask;
mod num;
mod paint;
mod pattern;
mod stream;
mod tagging;
mod text;

/// Check if image support is available (compiled with raster-images feature).
#[pyfunction]
fn has_image_support() -> bool {
    cfg!(feature = "raster-images")
}

/// Check if simple text support is available (compiled with simple-text feature).
#[pyfunction]
fn has_text_support() -> bool {
    cfg!(feature = "simple-text")
}

/// Python bindings for the krilla PDF library.
#[pymodule]
fn _krilla(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Feature detection
    m.add_function(wrap_pyfunction!(has_image_support, m)?)?;
    m.add_function(wrap_pyfunction!(has_text_support, m)?)?;

    // Exceptions
    m.add("KrillaError", py.get_type::<error::KrillaError>())?;
    m.add("FontError", py.get_type::<error::FontError>())?;
    m.add("ValidationError", py.get_type::<error::ValidationError>())?;
    m.add("ImageError", py.get_type::<error::ImageError>())?;

    // Geometry types
    m.add_class::<geometry::Point>()?;
    m.add_class::<geometry::Size>()?;
    m.add_class::<geometry::Rect>()?;
    m.add_class::<geometry::Transform>()?;
    m.add_class::<geometry::Path>()?;
    m.add_class::<geometry::PathBuilder>()?;

    // Numeric types
    m.add_class::<num::NormalizedF32>()?;

    // Enums
    m.add_class::<enums::FillRule>()?;
    m.add_class::<enums::LineCap>()?;
    m.add_class::<enums::LineJoin>()?;
    m.add_class::<enums::SpreadMethod>()?;
    m.add_class::<enums::BlendMode>()?;
    m.add_class::<enums::MaskType>()?;

    // Color submodule
    let color_module = PyModule::new(py, "color")?;
    color::register_module(&color_module)?;
    m.add_submodule(&color_module)?;

    // Paint types
    m.add_class::<paint::Paint>()?;
    m.add_class::<paint::Fill>()?;
    m.add_class::<paint::Stroke>()?;
    m.add_class::<paint::StrokeDash>()?;
    m.add_class::<paint::Stop>()?;
    m.add_class::<paint::LinearGradient>()?;
    m.add_class::<paint::RadialGradient>()?;
    m.add_class::<paint::SweepGradient>()?;

    // Text types
    m.add_class::<text::Font>()?;
    m.add_class::<text::GlyphId>()?;
    m.add_class::<text::_KrillaGlyph>()?;
    #[cfg(feature = "simple-text")]
    m.add_class::<text::TextDirection>()?;

    // Text conversion utilities
    m.add_function(wrap_pyfunction!(text::char_to_byte_offset, m)?)?;
    m.add_function(wrap_pyfunction!(text::char_range_to_bytes, m)?)?;

    // Stream types
    m.add_class::<stream::Stream>()?;
    m.add_class::<stream::StreamBuilder>()?;
    m.add_class::<stream::StreamSurface>()?;

    // Mask and pattern
    m.add_class::<mask::Mask>()?;
    m.add_class::<pattern::Pattern>()?;

    // Image types (feature-gated)
    #[cfg(feature = "raster-images")]
    m.add_class::<image::Image>()?;

    // Configuration types
    m.add_class::<config::PdfVersion>()?;
    m.add_class::<config::Validator>()?;
    m.add_class::<config::Configuration>()?;
    m.add_class::<config::SerializeSettings>()?;

    // Accessibility/tagging types
    m.add_class::<tagging::Location>()?;
    m.add_class::<tagging::ArtifactType>()?;
    m.add_class::<tagging::SpanTag>()?;
    m.add_class::<tagging::ContentTag>()?;
    m.add_class::<tagging::Identifier>()?;

    // Tag tree types
    m.add_class::<tagging::TagTree>()?;
    m.add_class::<tagging::TagGroup>()?;
    m.add_class::<tagging::Node>()?;
    m.add_class::<tagging::TagKind>()?;
    m.add_class::<tagging::Tag>()?;

    // Tag attribute enums
    m.add_class::<tagging::ListNumbering>()?;
    m.add_class::<tagging::TableHeaderScope>()?;
    m.add_class::<tagging::Placement>()?;
    m.add_class::<tagging::WritingMode>()?;
    m.add_class::<tagging::BorderStyle>()?;
    m.add_class::<tagging::TextAlign>()?;
    m.add_class::<tagging::BlockAlign>()?;
    m.add_class::<tagging::InlineAlign>()?;
    m.add_class::<tagging::TextDecorationType>()?;
    m.add_class::<tagging::GlyphOrientationVertical>()?;
    m.add_class::<tagging::LineHeight>()?;

    // Tag attribute complex types
    m.add_class::<tagging::TagId>()?;
    m.add_class::<tagging::BBox>()?;
    m.add_class::<tagging::NaiveRgbColor>()?;
    m.add_class::<tagging::SidesF32>()?;
    m.add_class::<tagging::ColumnDimensions>()?;

    // Interchange types (metadata, outline, embed)
    m.add_class::<interchange::DateTime>()?;
    m.add_class::<interchange::MetadataTextDirection>()?;
    m.add_class::<interchange::PageLayout>()?;
    m.add_class::<interchange::Metadata>()?;
    m.add_class::<interchange::XyzDestination>()?;
    m.add_class::<interchange::OutlineNode>()?;
    m.add_class::<interchange::Outline>()?;
    m.add_class::<interchange::AssociationKind>()?;
    m.add_class::<interchange::MimeType>()?;
    m.add_class::<interchange::EmbeddedFile>()?;

    // Core document types
    m.add_class::<document::PageSettings>()?;
    m.add_class::<document::Document>()?;
    m.add_class::<document::Page>()?;
    m.add_class::<document::Surface>()?;

    Ok(())
}
