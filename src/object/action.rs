use crate::serialize::SerializerContext;
use pdf_writer::types::ActionType;
use pdf_writer::Str;

pub trait Action {
    fn serialize_into(&self, sc: &mut SerializerContext, action: pdf_writer::writers::Action);
}

pub struct LinkAction {
    uri: String,
}

impl LinkAction {
    pub fn new(uri: String) -> Self {
        Self { uri }
    }
}

impl Action for LinkAction {
    fn serialize_into(&self, _: &mut SerializerContext, mut action: pdf_writer::writers::Action) {
        // TODO: Ensure only ASCII?

        action
            .action_type(ActionType::Uri)
            .uri(Str(self.uri.as_bytes()));
    }
}
