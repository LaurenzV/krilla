//! Optional content groups.
//!
//! An optional content group is a named layer whose visibility a viewer lets the user toggle.
//! Register one with
//! [`Document::add_optional_content_group`](crate::document::Document::add_optional_content_group),
//! then wrap the content belonging to it in
//! [`Surface::start_optional_content`](crate::surface::Surface::start_optional_content) and
//! [`Surface::end_optional_content`](crate::surface::Surface::end_optional_content).

use pdf_writer::Ref;

/// A handle to an optional content group.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OptionalContentGroupId(pub(crate) Ref);
