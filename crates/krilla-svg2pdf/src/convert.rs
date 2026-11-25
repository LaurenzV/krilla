use std::path::{Path, PathBuf};
use std::sync::Arc;

use krilla::geom::Size;
use krilla::page::PageSettings;
use krilla::Document;
use krilla_svg::{usvg, SurfaceExt, SvgSettings};

use crate::args::FontsArgs;
use crate::fonts::create_fontdb;

pub fn convert(
    input: &PathBuf,
    output: Option<PathBuf>,
    svg_settings: SvgSettings,
    dpi: f32,
    font_options: FontsArgs,
) -> Result<(), String> {
    // Set up fontdb
    let mut fontdb = create_fontdb(&font_options);

    fontdb.set_serif_family("Times New Roman");
    fontdb.set_sans_serif_family("Arial");
    fontdb.set_cursive_family("Comic Sans MS");
    fontdb.set_fantasy_family("Impact");
    fontdb.set_monospace_family("Courier New");

    let usvg_options = usvg::Options {
        fontdb: Arc::new(fontdb),
        ..Default::default()
    };

    let name = Path::new(
        input
            .file_name()
            .ok_or("Input path does not point to a file")?,
    );
    let output = output.unwrap_or_else(|| name.with_extension("pdf"));

    let svg_data = std::fs::read(input).map_err(|e| format!("Failed to load SVG file: {e}"))?;

    let svg_tree = usvg::Tree::from_data(&svg_data, &usvg_options)
        .map_err(|e| format!("Failed to parse SVG file: {e}"))?;

    let mut document = Document::new();

    let dpi_ratio = 72.0 / dpi;
    let svg_size = Size::from_wh(
        svg_tree.size().width() * dpi_ratio,
        svg_tree.size().height() * dpi_ratio,
    )
    .ok_or("SVG has invalid size")?;
    let mut page = document.start_page_with(PageSettings::new(svg_size));
    let mut surface = page.surface();
    surface.draw_svg(&svg_tree, svg_size, svg_settings);
    surface.finish();
    page.finish();

    let pdf = document
        .finish()
        .map_err(|e| format!("Failed to generate PDF: {e}"))?;

    std::fs::write(&output, pdf).map_err(|e| format!("Failed to write PDF file: {e}"))?;

    Ok(())
}
