//! Error handling.
//!
//! There are a lot of things that can go wrong when writing a PDF, like for example when
//! invalid fonts are provided. This module provides the basic error types krilla uses.

use crate::configure::ValidationError;
use crate::font::Font;
#[cfg(feature = "raster-images")]
use crate::Image;

/// A wrapper type for krilla errors.
pub type KrillaResult<T> = Result<T, KrillaError>;

/// An error in krilla.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum KrillaError {
    /// An error while attempting to embed a font.
    Font(Font, String),
    /// A list of validation errors. Can only occur if you set the `validator` in
    /// the [`SerializeSettings`] to something else than the dummy validator.
    ///
    /// [`SerializeSettings`]: crate::SerializeSettings
    Validation(Vec<ValidationError>),
    /// An image couldn't be processed properly.
    #[cfg(feature = "raster-images")]
    Image(Image),
}
