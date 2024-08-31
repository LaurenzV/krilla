mod chunk_container;
mod graphics_state;
mod object;
mod resource;
mod serialize;
mod svg;
mod util;

pub mod document;
pub mod error;
pub mod font;
pub mod geom;
pub use object::*;
pub mod paint;
pub mod path;
pub mod stream;
pub mod surface;

#[cfg(test)]
pub mod tests;

pub use serialize::{SerializeSettings, SvgSettings};
