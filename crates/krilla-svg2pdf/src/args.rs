use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct CliArguments {
    /// The command to run.
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Path to read SVG file from.
    pub input: Option<PathBuf>,

    /// Path to write PDF file to.
    pub output: Option<PathBuf>,

    /// The number of SVG pixels per PDF points.
    #[arg(long, default_value_t = 72.0)]
    pub dpi: f32,

    /// Whether text should be converted to paths before embedding into the PDF.
    #[arg(long, short, action=ArgAction::SetTrue)]
    pub text_to_paths: bool,

    /// How much raster images of rasterized effects should be scaled up.
    #[arg(long, default_value_t = 1.5)]
    pub raster_scale: f32,

    /// Common font arguments.
    #[command(flatten)]
    pub font: FontsArgs,
}

/// What to do.
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Lists all discovered fonts in system.
    Fonts(FontsCommand),
}

/// Lists all discovered fonts in system.
#[derive(Debug, Clone, Parser)]
pub struct FontsCommand {
    /// Also lists style variants of each font family.
    #[arg(long)]
    pub all: bool,

    /// Common font arguments.
    #[command(flatten)]
    pub font: FontsArgs,
}

/// The character typically used to separate path components
/// in environment variables.
const ENV_PATH_SEP: char = if cfg!(windows) { ';' } else { ':' };

/// Common arguments to customize available fonts.
#[derive(Debug, Clone, Parser)]
pub struct FontsArgs {
    /// Adds additional directories to search for fonts.
    ///
    /// If multiple paths are specified, they are separated by the system's path
    /// separator (`:` on Unix-like systems and `;` on Windows).
    #[arg(long = "font-path", value_name = "DIR", value_delimiter = ENV_PATH_SEP)]
    pub font_paths: Vec<PathBuf>,

    /// Ensures system fonts won't be used, unless explicitly included via
    /// `--font-path`.
    #[arg(long)]
    pub ignore_system_fonts: bool,
}
