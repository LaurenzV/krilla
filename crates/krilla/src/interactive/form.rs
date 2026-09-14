//! PDF forms, allowing you to add interactive widgets to the document,
//! such as text fields, checkboxes, radio buttons, and more.
//!
//! There is only a single form in the entire document, but it can have
//! multiple fields. These fields are organized in a tree structure.
//! Each field can be displayed on the page via one or more annotations.

use pdf_writer::{types::FieldFlags, writers::Form, Finish, Ref, TextStr};

use crate::{
    annotation::{DualStateAppearanceStream, NamedAppearanceStream, WidgetAnnotation},
    chunk_container::ChunkContainer,
    configure::{PdfVersion, ValidationError},
    form::kind::{Checkbox, Radio},
    geom::Rect,
    serialize::SerializeContext,
    stream::Stream,
    surface::Location,
    tagging::AnnotationIdentifier,
};

#[derive(Default)]
pub(crate) struct AcroForm {
    pub(crate) field_tree: Option<FieldTree>,
}

impl AcroForm {
    pub(crate) fn serialize(
        &self,
        sc: &mut SerializeContext,
        chunk_container: &mut ChunkContainer,
        root_ref: Ref,
    ) {
        let mut chunk = sc.new_chunk();
        let mut form = chunk.indirect(root_ref).start::<Form>();

        if let Some(field_tree) = &self.field_tree {
            let fields = field_tree
                .fields
                .iter()
                .map(|node| node.serialize_node(sc, chunk_container, None));

            form.fields(fields);
        }

        form.finish();

        chunk_container.non_stream.forms = Some((root_ref, chunk));
    }
}

/// A field tree.
#[derive(Default)]
pub struct FieldTree {
    /// The children of the field tree.
    pub fields: Vec<Node>,
}

impl FieldTree {
    /// Create a new field tree.
    pub fn new() -> Self {
        Default::default()
    }

    /// Append a new child to the field tree.
    pub fn push(&mut self, node: impl Into<Node>) {
        self.fields.push(node.into());
    }
}

/// A field group.
pub struct FieldGroup {
    /// The name of the field group.
    /// The group name must not contain any period character (`.`).
    pub name: String,
    /// The children of the field group.
    pub fields: Vec<Node>,
}

impl FieldGroup {
    /// Create a new field group with the given name.
    pub fn new(name: String) -> Self {
        Self {
            name,
            fields: vec![],
        }
    }

    /// Append a new child to the field group.
    pub fn push(&mut self, node: impl Into<Node>) {
        self.fields.push(node.into());
    }

    fn serialize_group(
        &self,
        sc: &mut SerializeContext,
        chunk_container: &mut ChunkContainer,
        parent_ref: Option<Ref>,
    ) -> Ref {
        debug_assert!(
            !self.name.contains('.'),
            "field group name cannot contain a period"
        );

        let ref_ = sc.new_ref();
        let children: Vec<_> = self
            .fields
            .iter()
            .map(|node| node.serialize_node(sc, chunk_container, Some(ref_)))
            .collect();

        let mut field = chunk_container.non_stream.fields.form_field(ref_);
        field.partial_name(TextStr(&self.name)).children(children);

        if let Some(parent_ref) = parent_ref {
            field.parent(parent_ref);
        }

        ref_
    }
}

/// A node in a field tree.
pub enum Node {
    /// A group node.
    Group(FieldGroup),
    /// A leaf node.
    Leaf(FieldKind),
}

impl Node {
    fn serialize_node(
        &self,
        sc: &mut SerializeContext,
        chunk_container: &mut ChunkContainer,
        parent_ref: Option<Ref>,
    ) -> Ref {
        match self {
            Self::Group(field_group) => {
                field_group.serialize_group(sc, chunk_container, parent_ref)
            }
            Self::Leaf(field_kind) => field_kind.serialize_field(sc, chunk_container, parent_ref),
        }
    }
}

impl<T> From<FormField<T>> for Node
where
    FormField<T>: Into<FieldKind>,
{
    fn from(value: FormField<T>) -> Self {
        Self::Leaf(value.into())
    }
}

impl From<FieldGroup> for Node {
    fn from(value: FieldGroup) -> Self {
        Self::Group(value)
    }
}

/// A type-agnostic field.
#[derive(Debug)]
pub enum FieldKind {
    /// A push button field.
    PushButton(FormField<kind::PushButton>),
    /// A checkbox field.
    Checkbox(FormField<kind::Checkbox>),
    /// A radio group field.
    Radio(FormField<kind::Radio>),
}

impl FieldKind {
    fn serialize_field(
        &self,
        sc: &mut SerializeContext,
        chunk_container: &mut ChunkContainer,
        parent_ref: Option<Ref>,
    ) -> Ref {
        match self {
            Self::PushButton(f) => f.serialize_field(sc, chunk_container, parent_ref),
            Self::Checkbox(f) => f.serialize_field(sc, chunk_container, parent_ref),
            Self::Radio(f) => f.serialize_field(sc, chunk_container, parent_ref),
        }
    }
}

impl From<FormField<kind::PushButton>> for FieldKind {
    fn from(value: FormField<kind::PushButton>) -> Self {
        Self::PushButton(value)
    }
}

impl From<FormField<kind::Checkbox>> for FieldKind {
    fn from(value: FormField<kind::Checkbox>) -> Self {
        Self::Checkbox(value)
    }
}

impl From<FormField<kind::Radio>> for FieldKind {
    fn from(value: FormField<kind::Radio>) -> Self {
        Self::Radio(value)
    }
}

/// A form field.
///
/// Fields can be created via [`FormField::push_button`],
/// [`FormField::checkbox`], and [`FormField::radio`].
#[derive(Debug, Default)]
pub struct FormField<T> {
    name: String,
    alt_name: Option<String>,
    mapping_name: Option<String>,
    flags: FieldFlags,
    pub(crate) identifier: Option<Ref>,
    pub(crate) annotations: Vec<AnnotationIdentifier>,
    kind: T,
    location: Option<Location>,
}

impl<T> FormField<T> {
    /// Set the alternative name of the field.
    /// This is used to refer to this field in the user interface,
    /// as well as for accessibility purposes.
    ///
    /// Note that the alt name might be required in some cases, for example
    /// when exporting to PDF/UA.
    pub fn set_alt_name(&mut self, alt_name: String) {
        self.alt_name = Some(alt_name);
    }

    /// Set the alternative name of the field.
    /// This is used to refer to this field in the user interface,
    /// as well as for accessibility purposes.
    ///
    /// Note that the alt name might be required in some cases, for example
    /// when exporting to PDF/UA.
    pub fn with_alt_name(mut self, alt_name: String) -> Self {
        self.set_alt_name(alt_name);
        self
    }

    /// Set the mapping name of the field.
    /// This is used during submission/export.
    pub fn set_mapping_name(&mut self, mapping_name: String) {
        self.mapping_name = Some(mapping_name);
    }

    /// Set the mapping name of the field.
    /// This is used during submission/export.
    pub fn with_mapping_name(mut self, mapping_name: String) -> Self {
        self.set_mapping_name(mapping_name);
        self
    }

    /// Set whether the field is read-only. Default: `false`.
    pub fn set_read_only(&mut self, read_only: bool) {
        self.flags.set(FieldFlags::READ_ONLY, read_only);
    }

    /// Set whether the field is read-only. Default: `false`.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.set_read_only(read_only);
        self
    }

    /// Set whether the field is required. Default: `false`.
    pub fn set_required(&mut self, required: bool) {
        self.flags.set(FieldFlags::REQUIRED, required);
    }

    /// Set whether the field is required. Default: `false`.
    pub fn with_required(mut self, required: bool) -> Self {
        self.set_required(required);
        self
    }

    /// Set whether the field will be exported during submission. Default: `true`.
    pub fn set_export(&mut self, export: bool) {
        self.flags.set(FieldFlags::NO_EXPORT, !export);
    }

    /// Set whether the field will be exported during submission. Default: `true`.
    pub fn with_export(mut self, export: bool) -> Self {
        self.set_export(export);
        self
    }

    /// Set the location of the field.
    pub fn set_location(&mut self, location: Option<Location>) {
        self.location = location;
    }

    /// Set the location of the field.
    pub fn with_location(mut self, location: Option<Location>) -> Self {
        self.set_location(location);
        self
    }
}

impl FormField<kind::PushButton> {
    /// Create a push button field.
    /// The field name must not contain any period character (`.`).
    pub fn push_button(name: String) -> Self {
        debug_assert!(!name.contains('.'), "field name cannot contain a period");
        Self {
            name,
            flags: FieldFlags::PUSHBUTTON,
            ..Default::default()
        }
    }

    /// Create a widget annotation for the push button field.
    ///
    /// - `rect`: The bounding box of the widget annotation that it should cover on the page.
    /// - `appearance`: The appearance of the widget annotation.
    pub fn new_widget(
        &self,
        rect: Rect,
        appearance: Stream,
    ) -> WidgetAnnotation<NamedAppearanceStream> {
        WidgetAnnotation::named(rect, "Yes".to_string(), appearance)
    }
}

impl FormField<kind::Checkbox> {
    /// Create a checkbox field.
    /// The field name must not contain any period character (`.`).
    pub fn checkbox(name: String, checked: bool) -> Self {
        debug_assert!(!name.contains('.'), "field name cannot contain a period");
        Self {
            name,
            kind: Checkbox {
                checked,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Set whether the checkbox is checked by default.
    pub fn set_default_checked(&mut self, checked: bool) {
        self.kind.default_checked = Some(checked);
    }

    /// Set whether the checkbox is checked by default.
    pub fn with_default_checked(mut self, checked: bool) -> Self {
        self.set_default_checked(checked);
        self
    }

    /// Create a widget annotation for the checkbox field.
    ///
    /// - `rect`: The bounding box of the widget annotation that it should cover on the page.
    /// - `off_appearance`: The appearance of the widget annotation when the checkbox is unchecked.
    /// - `on_appearance`: The appearance of the widget annotation when the checkbox is checked.
    pub fn new_widget(
        &self,
        rect: Rect,
        off_appearance: Stream,
        on_appearance: Stream,
    ) -> WidgetAnnotation<DualStateAppearanceStream> {
        WidgetAnnotation::dual(
            rect,
            self.kind.checked,
            "Off".to_string(),
            off_appearance,
            "Yes".to_string(),
            on_appearance,
        )
    }
}

impl FormField<kind::Radio> {
    /// Create a radio group field.
    /// The field name must not contain any period character (`.`).
    /// If the provided value is [`None`], no option is selected.
    pub fn radio(name: String, value: Option<String>) -> Self {
        debug_assert!(!name.contains('.'), "field name cannot contain a period");
        Self {
            name,
            flags: FieldFlags::RADIO,
            kind: Radio {
                value,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Set the default value of the radio group.
    /// It should correspond to a value of one of the annotations.
    /// If the provided value is [`None`], no option is selected.
    pub fn set_default_value(&mut self, value: Option<String>) {
        self.kind.default_value = value;
    }

    /// Set the default value of the radio group.
    /// It should correspond to a value of one of the annotations.
    /// If the provided value is [`None`], no option is selected.
    pub fn with_default_value(mut self, value: Option<String>) -> Self {
        self.set_default_value(value);
        self
    }

    /// Set whether to allow unselecting all buttons of the radio group.
    /// Default: true
    pub fn set_allow_toggling_off(&mut self, allow_off: bool) {
        self.flags.set(FieldFlags::NO_TOGGLE_TO_OFF, !allow_off);
    }

    /// Set whether to allow unselecting all buttons of the radio group.
    /// Default: true
    pub fn with_allow_toggling_off(mut self, allow_off: bool) -> Self {
        self.set_allow_toggling_off(allow_off);
        self
    }

    /// Set whether to toggle on all buttons with the same value simultaneously.
    /// Default: false
    pub fn set_radios_in_unison(&mut self, in_unison: bool) {
        self.flags.set(FieldFlags::RADIOS_IN_UNISON, in_unison);
    }

    /// Set whether to toggle on all buttons with the same value simultaneously.
    /// Default: false
    pub fn with_radios_in_unison(mut self, in_unison: bool) -> Self {
        self.set_radios_in_unison(in_unison);
        self
    }

    /// Create a widget annotation for the radio group field, which is automatically marked as
    /// selected if the value matches the field's value.
    /// If [radios in unison](Self::set_radios_in_unison) is `false` (default) and two or more
    /// annotations share the same `value`, [`Self::new_widget_with_selected`] must be used instead.
    ///
    /// - `rect`: The bounding box of the widget annotation that it should cover on the page.
    /// - `value`: The value the widget annotation represents in the radio group. Cannot be 'Off'.
    /// - `off_appearance`: The appearance of the widget annotation when the radio button is off.
    /// - `on_appearance`: The appearance of the widget annotation when the radio button is on.
    pub fn new_widget(
        &self,
        rect: Rect,
        value: String,
        off_appearance: Stream,
        on_appearance: Stream,
    ) -> WidgetAnnotation<DualStateAppearanceStream> {
        self.new_widget_with_selected(
            rect,
            self.kind.value.as_ref() == Some(&value),
            value,
            off_appearance,
            on_appearance,
        )
    }

    /// Create a widget annotation for the radio group field, manually specifying whether the radio
    /// button is selected.
    /// This must be used if [radios in unison](Self::set_radios_in_unison) is `false` (default)
    /// and two or more annotations share the same `value`.
    ///
    /// - `rect`: The bounding box of the widget annotation that it should cover on the page.
    /// - `selected`: Whether this widget annotation (radio button) is selected.
    /// - `value`: The value the widget annotation represents in the radio group. Cannot be 'Off'.
    /// - `off_appearance`: The appearance of the widget annotation when the radio button is off.
    /// - `on_appearance`: The appearance of the widget annotation when the radio button is on.
    pub fn new_widget_with_selected(
        &self,
        rect: Rect,
        selected: bool,
        value: String,
        off_appearance: Stream,
        on_appearance: Stream,
    ) -> WidgetAnnotation<DualStateAppearanceStream> {
        debug_assert_ne!(
            value, "Off",
            "A radio button cannot have reserved value 'Off'"
        );
        WidgetAnnotation::dual(
            rect,
            selected,
            "Off".to_string(),
            off_appearance,
            value,
            on_appearance,
        )
    }
}

#[allow(private_bounds)]
impl<T: SerializableField> FormField<T> {
    fn serialize_field(
        &self,
        sc: &mut SerializeContext,
        chunk_container: &mut ChunkContainer,
        parent_ref: Option<Ref>,
    ) -> Ref {
        let root_ref = self.identifier.unwrap_or_else(|| sc.new_ref());
        let mut field = chunk_container.non_stream.fields.form_field(root_ref);

        let mut flags = self.flags;
        if sc.serialize_settings().pdf_version() < PdfVersion::Pdf15 {
            // TODO: throw error instead; cannot use a ValidationError::RequiresNewerPdfVersion
            // since that requires exporting to PDF/A or PDF/UA
            flags.remove(FieldFlags::RADIOS_IN_UNISON);
        }

        field.partial_name(TextStr(&self.name)).field_flags(flags);

        if let Some(parent_ref) = parent_ref {
            field.parent(parent_ref);
        }

        if let Some(alt_name) = &self.alt_name {
            field.alternate_name(TextStr(alt_name));
        }
        if self.alt_name.as_ref().is_none_or(String::is_empty) {
            sc.register_validation_error(ValidationError::MissingFieldAltName(self.location));
        }

        if let Some(mapping_name) = &self.mapping_name {
            field.mapping_name(TextStr(mapping_name));
        }

        self.kind.serialize_field(&mut field);

        if !self.annotations.is_empty() {
            let annotations = self.annotations.iter().map(|identifier| {
                let page_annotations = sc.page_infos()[identifier.page_index].annotations();
                page_annotations[identifier.annot_index].0
            });
            field.children(annotations);
        }

        root_ref
    }
}

pub(crate) trait SerializableField {
    fn serialize_field<'a>(&self, field: &mut pdf_writer::writers::Field<'a>);
}

/// Field kind structs.
pub mod kind {
    use pdf_writer::{types::CheckBoxState, Name};

    use super::SerializableField;

    /// A push button.
    /// Create a field of this type via [`FormField::push_button`](super::FormField::push_button).
    #[derive(Debug, Clone, Default)]
    pub struct PushButton;

    impl SerializableField for PushButton {
        fn serialize_field<'a>(&self, field: &mut pdf_writer::writers::Field<'a>) {
            field.field_type(pdf_writer::types::FieldType::Button);
        }
    }

    /// A checkbox.
    /// Create a field of this type via [`FormField::checkbox`](super::FormField::checkbox).
    #[derive(Debug, Clone, Default)]
    pub struct Checkbox {
        pub(super) checked: bool,
        pub(super) default_checked: Option<bool>,
    }

    impl SerializableField for Checkbox {
        fn serialize_field<'a>(&self, field: &mut pdf_writer::writers::Field<'a>) {
            let value = if self.checked {
                CheckBoxState::Yes
            } else {
                CheckBoxState::Off
            };
            let default_value = if self.default_checked.unwrap_or(false) {
                CheckBoxState::Yes
            } else {
                CheckBoxState::Off
            };

            field
                .field_type(pdf_writer::types::FieldType::Button)
                .checkbox_value(value)
                .checkbox_default_value(default_value);
        }
    }

    /// A radio group.
    /// Create a field of this type via [`FormField::radio`](super::FormField::radio).
    #[derive(Debug, Clone, Default)]
    pub struct Radio {
        pub(super) value: Option<String>,
        pub(super) default_value: Option<String>,
    }

    impl SerializableField for Radio {
        fn serialize_field<'a>(&self, field: &mut pdf_writer::writers::Field<'a>) {
            let value = if let Some(value) = &self.value {
                value.as_bytes()
            } else {
                b"Off"
            };
            let default_value = if let Some(value) = &self.default_value {
                value.as_bytes()
            } else {
                b"Off"
            };

            field
                .field_type(pdf_writer::types::FieldType::Button)
                .radio_value(Name(value))
                .radio_default_value(Name(default_value));
        }
    }
}
