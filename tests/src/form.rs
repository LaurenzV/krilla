use krilla::action::ResetFormAction;
use krilla::form::kind::PushButton;
use krilla::form::{FieldGroup, FieldTree, FormField};
use krilla::geom::Rect;
use krilla::page::PageSettings;
use krilla::paint::Fill;
use krilla::stream::Stream;
use krilla::surface::Surface;
use krilla::Document;
use krilla_macros::snapshot;

use crate::{blue_fill, green_fill, rect_to_path, red_fill, square_stream};

#[snapshot(document)]
fn forms_push_button(d: &mut Document) {
    let mut page = d.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());

    let mut surface = page.surface();
    let button_appearance = square_stream(surface.stream_builder(), red_fill(1.0));
    let button_hover_appearance = square_stream(surface.stream_builder(), green_fill(1.0));
    surface.finish();

    let mut button = FormField::push_button("btn1".to_string());
    let mut button_widget = button.new_widget(
        Rect::from_xywh(50.0, 0.0, 10.0, 10.0).unwrap(),
        button_appearance,
    );
    button_widget.set_rollover_appearance(button_hover_appearance);
    button_widget.set_action_mouse_press(ResetFormAction::All.into());
    page.add_widget_annotation(&mut button, button_widget.into());

    page.finish();

    d.set_field_tree(FieldTree {
        fields: vec![button.into()],
    });
}

#[snapshot(document)]
fn forms_checkbox(d: &mut Document) {
    let mut page = d.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());

    let mut surface = page.surface();

    let on_appearance = square_stream(surface.stream_builder(), green_fill(1.0));
    let off_appearance = square_stream(surface.stream_builder(), red_fill(1.0));
    surface.finish();

    let mut checkbox = FormField::checkbox("checkbox1".to_string(), true);
    checkbox.set_alt_name("A checkbox".to_string());
    checkbox.set_mapping_name("chx1".to_string());
    checkbox.set_default_checked(false);

    let checkbox_widget = checkbox.new_widget(
        Rect::from_xywh(50.0, 0.0, 10.0, 10.0).unwrap(),
        off_appearance,
        on_appearance,
    );

    page.add_widget_annotation(&mut checkbox, checkbox_widget.into());

    page.finish();

    d.set_field_tree(FieldTree {
        fields: vec![checkbox.into()],
    });
}

#[snapshot(document)]
fn forms_radio_group(d: &mut Document) {
    let mut page = d.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());

    let mut surface = page.surface();

    let on_appearance = square_stream(surface.stream_builder(), green_fill(1.0));
    let off_appearance = square_stream(surface.stream_builder(), red_fill(1.0));
    surface.finish();

    let mut radio_group = FormField::radio("radio1".to_string(), Some("option1".to_string()));
    radio_group.set_required(true);
    radio_group.set_export(false);
    radio_group.set_radios_in_unison(true);
    radio_group.set_allow_toggling_off(false);

    let option_1_widget = radio_group.new_widget(
        Rect::from_xywh(50.0, 0.0, 10.0, 10.0).unwrap(),
        "option1".to_string(),
        off_appearance.clone(),
        on_appearance.clone(),
    );

    let option_2_widget = radio_group.new_widget(
        Rect::from_xywh(60.0, 0.0, 10.0, 10.0).unwrap(),
        "option2".to_string(),
        off_appearance.clone(),
        on_appearance.clone(),
    );

    let option_2_dup_widget = radio_group.new_widget(
        Rect::from_xywh(70.0, 0.0, 10.0, 10.0).unwrap(),
        "option2".to_string(),
        off_appearance,
        on_appearance,
    );

    page.add_widget_annotation(&mut radio_group, option_1_widget.into());
    page.add_widget_annotation(&mut radio_group, option_2_widget.into());
    page.add_widget_annotation(&mut radio_group, option_2_dup_widget.into());

    page.finish();

    d.set_field_tree(FieldTree {
        fields: vec![radio_group.into()],
    });
}

#[snapshot(document)]
fn forms_reset_action(d: &mut Document) {
    let mut page = d.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());

    let mut surface = page.surface();
    let on_appearance = square_stream(surface.stream_builder(), green_fill(1.0));
    let off_appearance = square_stream(surface.stream_builder(), red_fill(1.0));
    let button_appearance = square_stream(surface.stream_builder(), blue_fill(1.0));
    surface.finish();

    let mut checkbox_1 = FormField::checkbox("checkbox1".to_string(), false);
    checkbox_1.set_default_checked(true);
    let checkbox_1_widget = checkbox_1.new_widget(
        Rect::from_xywh(20.0, 0.0, 10.0, 10.0).unwrap(),
        off_appearance.clone(),
        on_appearance.clone(),
    );

    let mut checkbox_2 = FormField::checkbox("checkbox2".to_string(), false);
    checkbox_2.set_default_checked(false);
    let checkbox_2_widget = checkbox_2.new_widget(
        Rect::from_xywh(30.0, 0.0, 10.0, 10.0).unwrap(),
        off_appearance,
        on_appearance,
    );

    let mut reset_button_1 = FormField::push_button("reset1".to_string());
    let mut reset_button_1_widget = reset_button_1.new_widget(
        Rect::from_xywh(50.0, 0.0, 10.0, 10.0).unwrap(),
        button_appearance.clone(),
    );
    reset_button_1_widget.set_action_mouse_press(
        ResetFormAction::Include(vec!["checkboxes.checkbox1".to_string()]).into(),
    );

    let mut reset_button_2 = FormField::push_button("reset2".to_string());
    let mut reset_button_2_widget = reset_button_2.new_widget(
        Rect::from_xywh(60.0, 0.0, 10.0, 10.0).unwrap(),
        button_appearance,
    );
    reset_button_2_widget.set_action_mouse_press(
        ResetFormAction::Exclude(vec!["checkboxes.checkbox1".to_string()]).into(),
    );

    page.add_widget_annotation(&mut checkbox_1, checkbox_1_widget.into());
    page.add_widget_annotation(&mut checkbox_2, checkbox_2_widget.into());
    page.add_widget_annotation(&mut reset_button_1, reset_button_1_widget.into());
    page.add_widget_annotation(&mut reset_button_2, reset_button_2_widget.into());

    page.finish();

    d.set_field_tree(FieldTree {
        fields: vec![
            FieldGroup {
                name: "checkboxes".to_string(),
                fields: vec![checkbox_1.into(), checkbox_2.into()],
            }
            .into(),
            reset_button_1.into(),
            reset_button_2.into(),
        ],
    });
}
