mod chunk_container;
pub mod document;
pub mod font;
mod graphics_state;
mod object;
pub mod paint;
pub mod path;
mod resource;
mod serialize;
pub mod stream;
pub mod surface;
pub mod svg;
pub mod transform;
mod util;

mod error;
#[cfg(test)]
pub mod tests;

pub use fontdb::*;
pub use object::mask::MaskType;
pub use object::*;
pub use paint::*;

pub use path::*;
pub use tiny_skia_path::{Size, Transform};

// TODO: Add acknowledgements and license files
