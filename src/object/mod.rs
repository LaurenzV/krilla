use std::hash::Hash;
use crate::serialize::ObjectSerialize;

mod ext_g_state;
mod image;
mod shading_function;
mod shading_pattern;

/// Marker trait for PDF objects that can be cached. The type
/// should be cheap to clone.
pub trait Cacheable: ObjectSerialize + Hash + Eq + Clone {}