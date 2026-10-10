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
use crate::util::NameExt;

/// A type of action.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Action {
    /// A link action.
    Link(LinkAction),
    /// A go-to action.
    Goto(Destination),
    /// A reset form action.
    ResetForm(ResetFormAction),
    /// A submit form action.
    SubmitForm(SubmitFormAction),
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
            Action::SubmitForm(submit_form) => {
                submit_form.serialize(action);

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
#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
pub struct ResetFormAction {
    /// Which fields to reset.
    pub fields: ActionFieldFilter,
}

impl ResetFormAction {
    /// Create a new reset action that targets the given fields.
    pub fn new(fields: ActionFieldFilter) -> Self {
        Self { fields }
    }
}
impl ResetFormAction {
    fn serialize(&self, mut action: pdf_writer::writers::Action) {
        action.action_type(ActionType::ResetForm);
        let flags = self.fields.serialize(&mut action);
        if !flags.is_empty() {
            action.form_flags(flags);
        }
    }
}

impl From<ResetFormAction> for Action {
    fn from(value: ResetFormAction) -> Self {
        Action::ResetForm(value)
    }
}

/// A submit form action. Will submit the given form fields when triggered.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct SubmitFormAction {
    /// The URL to submit the form to.
    pub url: String,
    /// Which fields to submit.
    pub fields: ActionFieldFilter,
    /// The format to submit the form in.
    pub format: SubmissionFormat,
}

impl SubmitFormAction {
    /// Create a new submit action with the given URL.
    pub fn new(url: String) -> Self {
        Self {
            url,
            fields: ActionFieldFilter::default(),
            format: SubmissionFormat::default(),
        }
    }

    /// Set the fields to submit.
    pub fn with_fields(mut self, fields: ActionFieldFilter) -> Self {
        self.fields = fields;
        self
    }

    /// Set the format to submit in.
    pub fn with_format(mut self, format: SubmissionFormat) -> Self {
        self.format = format;
        self
    }
}

impl From<SubmitFormAction> for Action {
    fn from(value: SubmitFormAction) -> Self {
        Action::SubmitForm(value)
    }
}

impl SubmitFormAction {
    fn serialize(&self, mut action: pdf_writer::writers::Action) {
        action.action_type(ActionType::SubmitForm);
        action
            .file_spec()
            .file_system("URL".to_pdf_name())
            .path(Str(self.url.as_bytes()))
            .finish();
        let mut flags = self.fields.serialize(&mut action);
        flags |= self.format.serialize();
        if !flags.is_empty() {
            action.form_flags(flags);
        }
    }
}

/// Define which form fields are targeted/affected by an action.
#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
pub enum ActionFieldFilter {
    /// Target all fields in the document. Convenience variant for an empty [`ActionFieldFilter::Exclude`] variant.
    #[default]
    All,
    /// Target only the fields with the given fully qualified names.
    Include(Vec<String>),
    /// Target all fields in the document except the ones with the given fully qualified names.
    Exclude(Vec<String>),
}

impl ActionFieldFilter {
    fn serialize(&self, action: &mut pdf_writer::writers::Action) -> FormActionFlags {
        match self {
            Self::All => FormActionFlags::empty(),
            Self::Include(items) => {
                action
                    .fields()
                    .items(items.iter().map(|name| TextStr(name)))
                    .finish();
                FormActionFlags::empty()
            }
            Self::Exclude(items) => {
                action
                    .fields()
                    .items(items.iter().map(|name| TextStr(name)))
                    .finish();
                FormActionFlags::INCLUDE_EXCLUDE
            }
        }
    }
}

/// The format to submit the form in.
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub enum SubmissionFormat {
    /// application/x-www-form-urlencoded as a body if method is POST or as a query string in the URL if GET.
    UrlEncoded(HttpMethod),
    /// application/fdf
    #[default]
    Fdf,
    /// application/xfdf
    Xfdf,
    /// application/pdf
    Pdf,
}

impl SubmissionFormat {
    fn serialize(self) -> FormActionFlags {
        match self {
            Self::UrlEncoded(http_method) => match http_method {
                HttpMethod::Get => FormActionFlags::EXPORT_FORMAT | FormActionFlags::GET_METHOD,
                HttpMethod::Post => FormActionFlags::EXPORT_FORMAT,
            },
            Self::Fdf => FormActionFlags::empty(),
            Self::Xfdf => FormActionFlags::XFDF,
            Self::Pdf => FormActionFlags::SUBMIT_PDF,
        }
    }
}

/// The HTTP method to send the request in.
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub enum HttpMethod {
    /// GET
    Get,
    /// POST
    #[default]
    Post,
}
