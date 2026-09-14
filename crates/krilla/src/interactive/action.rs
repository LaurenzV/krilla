//! PDF actions, allowing you to add interactivity to the document.
//!
//! PDF has the concept of "actions", which encompass things like navigating to a URL,
//! opening some file on the system, and so on. The PDF reference defines a whole bunch
//! of actions, but krilla does not expose nearly all of them, and never will. As of right now,
//! the only available action is the link action, which allows you to specify a link that
//! should be opened, when activating the action.

use pdf_writer::types::{ActionType, FormActionFlags};
use pdf_writer::{Finish, Name, Str, TextStr};

use crate::configure::ValidationError;
use crate::error::KrillaResult;
use crate::interactive::destination::Destination;
use crate::serialize::SerializeContext;
use crate::surface::Location;

/// A type of action.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Action {
    /// A link action.
    Link(LinkAction),
    /// A go-to action.
    Goto(Destination),
    /// A reset form action.
    ResetForm(ResetFormAction),
}

impl Action {
    pub(crate) fn serialize(
        &self,
        sc: &mut SerializeContext,
        mut action: pdf_writer::writers::Action,
        location: Option<Location>,
    ) -> KrillaResult<()> {
        match self {
            Action::Link(link) => {
                link.serialize(action);

                Ok(())
            }
            Action::Goto(dest) => {
                let dest_entry = action.action_type(ActionType::GoTo).insert(Name(b"D"));
                dest.serialize(sc, dest_entry)
            }
            Action::ResetForm(reset_form) => {
                reset_form.serialize(action);

                sc.register_validation_error(ValidationError::ContainsMutatingAction(location));

                Ok(())
            }
        }
    }
}

/// A link action. Will open a link when clicked.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct LinkAction {
    uri: String,
}

impl From<LinkAction> for Action {
    fn from(value: LinkAction) -> Self {
        Action::Link(value)
    }
}

impl LinkAction {
    /// Create a new link action that will open a URI when clicked.
    pub fn new(uri: String) -> Self {
        Self { uri }
    }
}

impl LinkAction {
    fn serialize(&self, mut action: pdf_writer::writers::Action) {
        action
            .action_type(ActionType::Uri)
            .uri(Str(self.uri.as_bytes()));
    }
}

/// A reset form action. Will reset the given form fields when triggered.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ResetFormAction {
    /// Reset all fields in the document. Convenience variant for an empty [`ResetFormAction::Exclude`] variant.
    All,
    /// Reset only the fields with the given fully qualified names.
    Include(Vec<String>),
    /// Reset all fields in the document except the ones with the given fully qualified names.
    Exclude(Vec<String>),
}

impl From<ResetFormAction> for Action {
    fn from(value: ResetFormAction) -> Self {
        Action::ResetForm(value)
    }
}

impl ResetFormAction {
    fn serialize(&self, mut action: pdf_writer::writers::Action) {
        action.action_type(ActionType::ResetForm);
        match self {
            ResetFormAction::All => {}
            ResetFormAction::Include(items) => {
                action
                    .fields()
                    .items(items.iter().map(|name| TextStr(name)))
                    .finish();
            }
            ResetFormAction::Exclude(items) => {
                action
                    .fields()
                    .items(items.iter().map(|name| TextStr(name)))
                    .finish();
                action.form_flags(FormActionFlags::INCLUDE_EXCLUDE);
            }
        }
    }
}
