//! Error handling.
//!
//! There are a lot of things that can go wrong when writing a PDF, like for example when
//! invalid fonts are provided. This module provides the basic error types krilla uses.

use crate::configure::ValidationError;
#[cfg(feature = "raster-images")]
use crate::graphics::image::Image;
use crate::pdf::{PdfDocument, PdfError};
use crate::surface::Location;
use crate::tagging::TagId;
use crate::text::Font;

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
    /// A duplicate [`Tag::id`] was provided.
    ///
    /// [`Tag::id`]: crate::interchange::tagging::Tag::id
    DuplicateTagId(TagId, Option<Location>),
    /// A [`TagId`] was not found in the [`TagTree`].
    ///
    /// [`TagTree`]: crate::interchange::tagging::TagTree
    UnknownTagId(TagId, Option<Location>),
    /// An image couldn't be processed properly.
    #[cfg(feature = "raster-images")]
    Image(Image, Option<Location>),
    /// A embedded PDF document couldn't be processed properly.
    #[cfg(feature = "pdf")]
    Pdf(PdfDocument, PdfError, Option<Location>),
    /// A sixteen bit image was used, even though it isn't
    /// supported by the used PDF version (only available in PDF 1.5+).
    #[cfg(feature = "raster-images")]
    SixteenBitImage(Image, Option<Location>),
}
