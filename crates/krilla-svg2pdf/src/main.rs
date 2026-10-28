use std::process;

use clap::Parser;
use krilla_svg::SvgSettings;

use crate::args::{CliArguments, Command};

mod args;
mod convert;
mod fonts;

fn main() {
    if let Err(msg) = run() {
        eprintln!("error: {msg}.");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = CliArguments::parse();

    // If an input argument is provided, convert the SVG to PDF.
    if let Some(input) = args.input {
        // Set up SVG settings.
        let svg_settings = SvgSettings {
            embed_text: !args.text_to_paths,
            filter_scale: args.raster_scale,
        };

        return convert::convert(
            &input,
            args.output,
            svg_settings,
            args.dpi,
            args.font,
        );
    }

    // Otherwise execute the command provided if any.
    if let Some(command) = args.command {
        match command {
            Command::Fonts(command) => return fonts::fonts(&command),
        }
    }

    Err("no command was provided".to_string())
}
