#![forbid(unsafe_code)]

pub mod canvas;
pub mod color;
mod object;
pub mod paint;
pub mod path;
pub mod resource;
pub mod serialize;
pub mod transform;
pub mod util;

pub use path::*;
pub use tiny_skia_path::{Size, Transform};
