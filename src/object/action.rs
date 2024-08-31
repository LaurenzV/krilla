//! A collection of actions, which allow you to add interactivity to the document.

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
