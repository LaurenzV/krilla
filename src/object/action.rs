//! PDF actions, allowing you to add interactivity to the document.
//!
//! PDF has the concept of "actions", which encompass things like navigating to a URL,
//! opening some file on the system, and so on. The PDF reference defines a whole bunch
//! of actions, but krilla does not expose nearly all of them, and never will. As of right now,
//! the only available action is the link action, which allows you to specify a link that
//! should be opened, when activating the action.

use crate::serialize::SerializerContext;
use pdf_writer::types::ActionType;
use pdf_writer::Str;

/// A type of action.
pub enum Action {
    /// A link action.
    Link(LinkAction),
}

impl Action {
    pub(crate) fn serialize(&self, _: &mut SerializerContext, action: pdf_writer::writers::Action) {
        match self {
            Action::Link(link) => link.serialize(action),
        }
    }
}

/// A link action. Will open a link when clicked.
pub struct LinkAction {
    uri: String,
}

impl Into<Action> for LinkAction {
    fn into(self) -> Action {
        Action::Link(self)
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

// No tests here, because we test through `Annotation`.
