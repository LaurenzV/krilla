//! Rendering graphics objects into a PDF file.

pub mod color;
pub mod graphics_state;
#[cfg(feature = "raster-images")]
pub mod image;
pub mod mask;
pub mod paint;
pub mod shading_function;
pub mod shading_pattern;
pub mod tiling_pattern;
pub mod xobject;
