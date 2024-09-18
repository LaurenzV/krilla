//! Error handling.
//!
//! There are a lot of things that can go wrong when writing a PDF, like for example when
//! invalid fonts are provided. This module provides the basic error types krilla uses.

/// A wrapper type for krilla errors.
pub type KrillaResult<T> = Result<T, KrillaError>;

/// An error in krilla.
#[derive(Debug, PartialEq, Eq)]
pub enum KrillaError {
    /// A font-related error, most likely indicated that the font is either not
    /// supported or has issues. The first field contains the ID of the font,
    /// if set, and the second field the message.
    Font(Option<u32>, String),
    /// A user-related error, indicating API misuse (for example attempting to add
    /// a link to a page that doesn't exist).
    UserError(String),
}
