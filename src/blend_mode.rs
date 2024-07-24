#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum BlendMode {
    /// The composite mode 'Clear'.
    Clear,
    /// The composite mode 'Source'.
    Source,
    /// The composite mode 'Destination'.
    Destination,
    /// The composite mode 'SourceOver'.
    SourceOver,
    /// The composite mode 'DestinationOver'.
    DestinationOver,
    /// The composite mode 'SourceIn'.
    SourceIn,
    /// The composite mode 'DestinationIn'.
    DestinationIn,
    /// The composite mode 'SourceOut'.
    SourceOut,
    /// The composite mode 'DestinationOut'.
    DestinationOut,
    /// The composite mode 'SourceAtop'.
    SourceAtop,
    /// The composite mode 'DestinationAtop'.
    DestinationAtop,
    /// The composite mode 'Xor'.
    Xor,
    /// The composite mode 'Plus'.
    Plus,
    /// The composite mode 'Screen'.
    Screen,
    /// The composite mode 'Overlay'.
    Overlay,
    /// The composite mode 'Darken'.
    Darken,
    /// The composite mode 'Lighten'.
    Lighten,
    /// The composite mode 'ColorDodge'.
    ColorDodge,
    /// The composite mode 'ColorBurn'.
    ColorBurn,
    /// The composite mode 'HardLight'.
    HardLight,
    /// The composite mode 'SoftLight'.
    SoftLight,
    /// The composite mode 'Difference'.
    Difference,
    /// The composite mode 'Exclusion'.
    Exclusion,
    /// The composite mode 'Multiply'.
    Multiply,
    /// The composite mode 'Hue'.
    Hue,
    /// The composite mode 'Saturation'.
    Saturation,
    /// The composite mode 'Color'.
    Color,
    /// The composite mode 'Luminosity'.
    Luminosity,
}

impl TryInto<pdf_writer::types::BlendMode> for BlendMode {
    type Error = ();

    fn try_into(self) -> Result<pdf_writer::types::BlendMode, Self::Error> {
        match self {
            BlendMode::SourceOver => Ok(pdf_writer::types::BlendMode::Normal),
            // BlendMode::Clear => {}
            // BlendMode::Source => {}
            // BlendMode::Destination => {}
            // BlendMode::DestinationOver => {}
            // BlendMode::SourceIn => {}
            // BlendMode::DestinationIn => {}
            // BlendMode::SourceOut => {}
            // BlendMode::DestinationOut => {}
            // BlendMode::SourceAtop => {}
            // BlendMode::DestinationAtop => {}
            // BlendMode::Xor => {}
            // BlendMode::Plus => {}
            BlendMode::Screen => Ok(pdf_writer::types::BlendMode::Screen),
            BlendMode::Overlay => Ok(pdf_writer::types::BlendMode::Overlay),
            BlendMode::Darken => Ok(pdf_writer::types::BlendMode::Darken),
            BlendMode::Lighten => Ok(pdf_writer::types::BlendMode::Lighten),
            BlendMode::ColorDodge => Ok(pdf_writer::types::BlendMode::ColorDodge),
            BlendMode::ColorBurn => Ok(pdf_writer::types::BlendMode::ColorBurn),
            BlendMode::HardLight => Ok(pdf_writer::types::BlendMode::HardLight),
            BlendMode::SoftLight => Ok(pdf_writer::types::BlendMode::SoftLight),
            BlendMode::Difference => Ok(pdf_writer::types::BlendMode::Difference),
            BlendMode::Exclusion => Ok(pdf_writer::types::BlendMode::Exclusion),
            BlendMode::Multiply => Ok(pdf_writer::types::BlendMode::Multiply),
            BlendMode::Hue => Ok(pdf_writer::types::BlendMode::Hue),
            BlendMode::Saturation => Ok(pdf_writer::types::BlendMode::Saturation),
            BlendMode::Color => Ok(pdf_writer::types::BlendMode::Color),
            BlendMode::Luminosity => Ok(pdf_writer::types::BlendMode::Luminosity),
            _ => Err(()),
        }
    }
}
