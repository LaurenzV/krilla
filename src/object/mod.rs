use crate::serialize::ObjectSerialize;
use std::hash::Hash;

pub mod color_space;
pub mod ext_g_state;
pub mod image;
pub mod shading_function;
pub mod shading_pattern;
pub mod tiling_pattern;
mod x_object;

/// Marker trait for PDF objects that can be cached. The type
/// should be cheap to clone.
pub trait Cacheable: ObjectSerialize + Hash + Eq + Clone {}
