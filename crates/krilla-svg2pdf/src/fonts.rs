use std::collections::BTreeMap;

use crate::args::{FontsArgs, FontsCommand};

/// Create a font database from the common font arguments.
pub fn create_fontdb(font_args: &FontsArgs) -> fontdb::Database {
    let mut fontdb = fontdb::Database::new();

    if !font_args.ignore_system_fonts {
        fontdb.load_system_fonts();
    }

    for font_path in &font_args.font_paths {
        fontdb.load_fonts_dir(font_path);
    }

    fontdb
}

/// Execute a font listing command.
pub fn fonts(command: &FontsCommand) -> Result<(), String> {
    // Prepare the font database.
    let fontdb = create_fontdb(&command.font);

    // Collect the font families.
    let mut font_families: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for face in fontdb.faces() {
        for family in &face.families {
            font_families
                .entry(family.0.clone())
                .and_modify(|value| value.push(face.post_script_name.clone()))
                .or_insert(vec![face.post_script_name.clone()]);
        }
    }

    // Display the results.
    for (family, mut names) in font_families {
        names.sort();
        let mut name_string = String::new();
        name_string.push_str(&family);
        if command.all {
            for (idx, name) in names.iter().enumerate() {
                if idx == (names.len() - 1) {
                    name_string.push_str("\n└ ")
                } else {
                    name_string.push_str("\n├ ")
                }
                name_string.push_str(name);
            }
        }
        println!("{name_string}");
    }

    Ok(())
}
