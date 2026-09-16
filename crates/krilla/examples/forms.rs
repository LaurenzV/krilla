//! This example shows how to create a document with form fields.

use std::path;
use std::path::PathBuf;

use krilla::action::ResetFormAction;
use krilla::color::rgb;
use krilla::form::kind::ChoiceOption;
use krilla::form::variable_text::{FormFont, TextAlignment, VariableAppearance};
use krilla::form::{FieldGroup, FieldTree, FormField};
use krilla::geom::{Path, PathBuilder, Point, Rect};
use krilla::page::PageSettings;
use krilla::paint::{Fill, Stroke};
use krilla::text::Font;
use krilla::text::StandardFont;
use krilla::text::TextDirection;
use krilla::Document;

fn main() {
    // Create a new document.
    let mut document = Document::new();

    // Load some fonts.
    let font = {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/fonts/NotoSans-Regular.ttf");
        let data = std::fs::read(&path).unwrap();
        Font::new(data.into(), 0).unwrap()
    };
    let font_serif = {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/fonts/LibertinusSerif-Regular.otf");
        let data = std::fs::read(&path).unwrap();
        Font::new(data.into(), 0).unwrap()
    };

    // Create a checkbox.
    let mut checkbox = FormField::checkbox("checkbox".to_string(), false);
    // Set an alternative name, for accessibility purposes.
    checkbox.set_alt_name("A checkbox".to_string());

    // Create a radio group.
    let mut radio_group = FormField::radio("radio".to_string(), None);
    // Set an alternative name, for accessibility purposes.
    radio_group.set_alt_name("A radio group".to_string());
    // Disallow toggling the radio group back to off.
    radio_group.set_allow_toggling_off(false);

    // Create a text field.
    let mut text_field = FormField::text("text".to_string());
    // Set an alternative name, for accessibility purposes.
    text_field.set_alt_name("A text field".to_string());
    // Set the appearance PDF processors should use when filling out this text field.
    text_field.set_appearance(VariableAppearance {
        font: FormFont::Standard(StandardFont::TimesRoman),
        font_size: 10.0,
        paint: Some(rgb::Color::new(255, 0, 255).into()),
        ..Default::default()
    });
    // Tell PDF processors to center the text when filling out this text field.
    text_field.set_text_alignment(TextAlignment::Center);
    // Pre-fill this field with the given value.
    text_field.set_value("testing".to_string());

    // Create a combobox field.
    let mut combobox_field = FormField::combobox("combobox".to_string());
    // Set an alternative name, for accessibility purposes.
    combobox_field.set_alt_name("A combobox field with 3 colors as options".to_string());
    // Set the available options of this field, along with their display names
    combobox_field.set_options(vec![
        ChoiceOption::new("red".into(), Some("Red".into())),
        ChoiceOption::new("green".into(), Some("Green".into())),
        ChoiceOption::new("blue".into(), Some("Blue".into())),
    ]);
    // Allow users to add their own values instead.
    combobox_field.set_edit(true);
    // Set the appearance PDF processors should use when filling out this combobox field.
    combobox_field.set_appearance(VariableAppearance {
        font: FormFont::Standard(StandardFont::Helvetica),
        font_size: 10.0,
        ..Default::default()
    });

    // Create a listbox field.
    let mut listbox_field = FormField::listbox("listbox".to_string());
    // Set an alternative name, for accessibility purposes.
    listbox_field.set_alt_name("A listbox field with 3 colors as options".to_string());
    // Set the available options of this field, along with their display names
    listbox_field.set_options(vec![
        ChoiceOption::new("red".into(), Some("Red".into())),
        ChoiceOption::new("green".into(), Some("Green".into())),
        ChoiceOption::new("blue".into(), Some("Blue".into())),
    ]);
    // Allow users to select multiple options.
    listbox_field.set_multiple_options(true);
    // Set the appearance PDF processors should use when filling out this listbox field.
    listbox_field.set_appearance(VariableAppearance {
        font: FormFont::Standard(StandardFont::Helvetica),
        font_size: 10.0,
        ..Default::default()
    });

    // Create a button that will reset the form when clicked.
    let mut reset_button = FormField::push_button("reset-button".to_string());
    // Set an alternative name, for accessibility purposes.
    reset_button.set_alt_name("Reset button".to_string());

    // Add a new page with dimensions 200x200.
    let mut page = document.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());
    // Get the surface of the page.
    let mut surface = page.surface();
    // Draw some text.
    surface.draw_text(
        Point::from_xy(0.0, 25.0),
        font.clone(),
        10.0,
        "Below you can find many form fields!",
        false,
        TextDirection::Auto,
    );

    // Create an appearance for when a checkbox is checked.
    let checkbox_on_appearance = {
        let mut builder = surface.stream_builder();
        let mut surface = builder.surface();

        surface.draw_text(
            Point::from_xy(0.0, 12.0),
            font.clone(),
            8.0,
            "ON",
            false,
            TextDirection::Auto,
        );

        surface.finish();
        builder.finish()
    };

    // Create an appearance for when a checkbox is unchecked.
    let checkbox_off_appearance = {
        let mut builder = surface.stream_builder();
        let mut surface = builder.surface();

        surface.draw_text(
            Point::from_xy(0.0, 12.0),
            font.clone(),
            8.0,
            "OFF",
            false,
            TextDirection::Auto,
        );

        surface.finish();
        builder.finish()
    };

    // Create a widget annotation (visual representation) for the checkbox.
    let checkbox_widget = checkbox.new_widget(
        Rect::from_xywh(20.0, 40.0, 20.0, 20.0).unwrap(),
        checkbox_off_appearance,
        checkbox_on_appearance,
    );

    // Draw some accompanying text.
    surface.draw_text(
        Point::from_xy(45.0, 55.0),
        font.clone(),
        10.0,
        "<-- This is a checkbox",
        false,
        TextDirection::Auto,
    );

    // Create an appearance for when a radio button is selected.
    let radio_on_appearance = {
        let mut builder = surface.stream_builder();
        let mut surface = builder.surface();

        surface.set_fill(None);
        surface.set_stroke(Some(Stroke {
            paint: rgb::Color::black().into(),
            ..Default::default()
        }));
        surface.draw_path(&rect_to_path(0.0, 0.0, 20.0, 20.0));

        surface.set_fill(Some(Fill {
            paint: rgb::Color::black().into(),
            ..Default::default()
        }));
        surface.set_stroke(None);
        surface.draw_path(&rect_to_path(3.0, 3.0, 14.0, 14.0));

        surface.finish();
        builder.finish()
    };

    // Create an appearance for when a radio button is unselected.
    let radio_off_appearance = {
        let mut builder = surface.stream_builder();
        let mut surface = builder.surface();

        surface.set_fill(None);
        surface.set_stroke(Some(Stroke {
            paint: rgb::Color::black().into(),
            ..Default::default()
        }));
        surface.draw_path(&rect_to_path(0.0, 0.0, 20.0, 20.0));

        surface.finish();
        builder.finish()
    };

    // Create a widget annotation (visual representation) for the radio group, with value 'option1'.
    let radio_option_1 = radio_group.new_widget(
        Rect::from_xywh(20.0, 80.0, 20.0, 20.0).unwrap(),
        "option1".to_string(),
        radio_off_appearance.clone(),
        radio_on_appearance.clone(),
    );

    // Draw some accompanying text.
    surface.draw_text(
        Point::from_xy(45.0, 95.0),
        font.clone(),
        10.0,
        "Option 1",
        false,
        TextDirection::Auto,
    );

    // Create another widget annotation (visual representation) for the radio group, with value 'option2'.
    let radio_option_2 = radio_group.new_widget(
        Rect::from_xywh(120.0, 80.0, 20.0, 20.0).unwrap(),
        "option2".to_string(),
        radio_off_appearance,
        radio_on_appearance,
    );

    // Draw some accompanying text.
    surface.draw_text(
        Point::from_xy(145.0, 95.0),
        font.clone(),
        10.0,
        "Option 2",
        false,
        TextDirection::Auto,
    );

    // Create an appearance for the text field.
    // It is our responsibility to ensure the styles and the value match those of the field.
    let text_appearance = {
        let mut builder = surface.stream_builder();
        let mut surface = builder.surface();

        // Start the part of the appearance stream that represents the value of the field.
        surface.start_variable_text();

        // Write the current value, taking into consideration the appearance of the field.
        surface.set_fill(Some(Fill {
            paint: rgb::Color::new(255, 0, 255).into(),
            ..Default::default()
        }));
        surface.draw_text(
            Point::from_xy(6.0, 13.0),
            font_serif.clone(),
            10.0,
            "testing",
            false,
            TextDirection::Auto,
        );

        // Do not forget to end the section!
        surface.end_variable_text();

        surface.finish();
        builder.finish()
    };

    // Create a widget annotation (visual representation) for the text field.
    let text_widget = text_field.new_widget(
        Rect::from_xywh(40.0, 120.0, 40.0, 20.0).unwrap(),
        text_appearance,
    );

    // Create an appearance for the combobox field.
    let combobox_appearance = {
        let mut builder = surface.stream_builder();
        let mut surface = builder.surface();

        // Draw a border around the field.
        surface.set_stroke(Some(Stroke {
            paint: rgb::Color::black().into(),
            ..Default::default()
        }));
        surface.draw_path(&rect_to_path(0.0, 0.0, 60.0, 20.0));

        // Start the part of the appearance stream that represents the value of the field.
        surface.start_variable_text();

        // There is no value, so we can leave this empty instead.

        // Do not forget to end the section!
        surface.end_variable_text();

        surface.finish();
        builder.finish()
    };

    // Create a widget annotation (visual representation) for the combobox field.
    let combobox_widget = combobox_field.new_widget(
        Rect::from_xywh(120.0, 120.0, 60.0, 20.0).unwrap(),
        combobox_appearance,
    );

    // Create an appearance for the listbox field.
    // It is our responsibility to draw the available options.
    let listbox_appearance = {
        let mut builder = surface.stream_builder();
        let mut surface = builder.surface();

        // Draw a border around the field.
        surface.set_stroke(Some(Stroke {
            paint: rgb::Color::black().into(),
            ..Default::default()
        }));
        surface.draw_path(&rect_to_path(0.0, 0.0, 60.0, 40.0));
        surface.set_stroke(None);

        // Start the part of the appearance stream that represents the value of the field.
        surface.start_variable_text();

        // Draw the options
        surface.draw_text(
            Point::from_xy(2.0, 10.0),
            font.clone(),
            8.0,
            "Red",
            false,
            TextDirection::Auto,
        );
        surface.draw_text(
            Point::from_xy(2.0, 20.0),
            font.clone(),
            8.0,
            "Green",
            false,
            TextDirection::Auto,
        );
        surface.draw_text(
            Point::from_xy(2.0, 30.0),
            font.clone(),
            8.0,
            "Blue",
            false,
            TextDirection::Auto,
        );

        // Do not forget to end the section!
        surface.end_variable_text();

        surface.finish();
        builder.finish()
    };

    // Create a widget annotation (visual representation) for the listbox field.
    let listbox_widget = listbox_field.new_widget(
        Rect::from_xywh(120.0, 150.0, 60.0, 40.0).unwrap(),
        listbox_appearance,
    );

    // Create an appearance for the reset push button.
    let button_appearance = {
        let mut builder = surface.stream_builder();
        let mut surface = builder.surface();

        surface.draw_text(
            Point::from_xy(0.0, 12.0),
            font.clone(),
            8.0,
            "Reset",
            false,
            TextDirection::Auto,
        );

        surface.finish();
        builder.finish()
    };

    // Create a widget annotation (visual representation) for the push button.
    let mut reset_button_widget = reset_button.new_widget(
        Rect::from_xywh(40.0, 150.0, 40.0, 20.0).unwrap(),
        button_appearance,
    );
    // Set an on click action (reset all fields except for button itself).
    // Some PDF readers erase the button's appearance if it is reset, making it invisible.
    reset_button_widget.set_action_mouse_press(
        ResetFormAction::Exclude(vec!["actions.reset-button".into()]).into(),
    );

    // Finish the surface.
    surface.finish();

    // Add all created annotations to the page.
    page.add_widget_annotation(&mut checkbox, checkbox_widget.into());
    page.add_widget_annotation(&mut radio_group, radio_option_1.into());
    page.add_widget_annotation(&mut radio_group, radio_option_2.into());
    page.add_widget_annotation(&mut text_field, text_widget.into());
    page.add_widget_annotation(&mut combobox_field, combobox_widget.into());
    page.add_widget_annotation(&mut listbox_field, listbox_widget.into());
    page.add_widget_annotation(&mut reset_button, reset_button_widget.into());

    // Finish the page.
    page.finish();

    // Add all fields to the page, in a tree structure.
    document.set_field_tree(FieldTree {
        fields: vec![
            checkbox.into(),
            radio_group.into(),
            text_field.into(),
            combobox_field.into(),
            listbox_field.into(),
            // Fields can be grouped arbitrarily.
            FieldGroup {
                name: "actions".to_string(),
                fields: vec![reset_button.into()],
            }
            .into(),
        ],
    });

    // Finish up and write the resulting PDF.
    let pdf = document.finish().unwrap();
    let path = path::absolute("forms.pdf").unwrap();
    eprintln!("Saved PDF to '{}'", path.display());

    // Write the PDF to a file.
    std::fs::write(path, &pdf).unwrap();
}

// A simple convenience function that allow us to generate rectangle paths.
fn rect_to_path(x: f32, y: f32, w: f32, h: f32) -> Path {
    let mut builder = PathBuilder::new();
    builder.push_rect(Rect::from_xywh(x, y, w, h).unwrap());
    builder.finish().unwrap()
}
