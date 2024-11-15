//! Error handling.
//!
//! There are a lot of things that can go wrong when writing a PDF, like for example when
//! invalid fonts are provided. This module provides the basic error types krilla uses.

use crate::font::Font;
use crate::validation::ValidationError;

/// A wrapper type for krilla errors.
pub type KrillaResult<T> = Result<T, KrillaError>;

/// An error in krilla.
#[derive(Debug, PartialEq, Eq)]
pub enum KrillaError {
    /// An error while attempting to embed a font.
    FontError(Font, String),
    /// A user-related error, indicating API misuse (for example attempting to add
    /// a link to a page that doesn't exist).
    UserError(String),
    /// A list of validation errors. Can only occur if you set the `validator` in
    /// the [`SerializeSettings`] to something else than the dummy validator.
    ///
    /// [`SerializeSettings`]: crate::SerializeSettings
    ValidationError(Vec<ValidationError>),
}
