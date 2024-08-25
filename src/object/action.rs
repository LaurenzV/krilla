use crate::serialize::SerializerContext;
use pdf_writer::types::ActionType;
use pdf_writer::Str;

pub enum Action {
    Link(LinkAction),
}

impl Action {
    pub(crate) fn serialize_into(
        &self,
        sc: &mut SerializerContext,
        action: pdf_writer::writers::Action,
    ) {
        match self {
            Action::Link(link) => link.serialize_into(sc, action),
        }
    }
}

pub struct LinkAction {
    uri: String,
}

impl Into<Action> for LinkAction {
    fn into(self) -> Action {
        Action::Link(self)
    }
}

impl LinkAction {
    pub fn new(uri: String) -> Self {
        Self { uri }
    }
}

impl LinkAction {
    fn serialize_into(&self, _: &mut SerializerContext, mut action: pdf_writer::writers::Action) {
        action
            .action_type(ActionType::Uri)
            .uri(Str(self.uri.as_bytes()));
    }
}
