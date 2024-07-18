use tiny_skia_path::NormalizedF32;
use crate::object::color_space::ColorSpace;

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub struct Grey {
    pub lightness: u8,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum Color {
    Rgb(Rgb),
    Grey(Grey),
}

impl Color {
    pub fn new_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::Rgb(Rgb { red, green, blue })
    }

    pub fn new_grey(lightness: u8) -> Self {
        Self::Grey(Grey { lightness })
    }

    pub fn black() -> Color {
        Color::new_grey(0)
    }

    pub fn white() -> Color {
        Color::new_grey(255)
    }
}

pub trait PdfColorExt {
    fn to_pdf_components(&self) -> Vec<f32>;
    fn to_normalized_pdf_components(&self) -> Vec<NormalizedF32>;
    fn get_pdf_color_space(&self) -> ColorSpace;
}

impl PdfColorExt for Color {
    fn to_pdf_components(&self) -> Vec<f32> {
        match self {
            Color::Rgb(rgb) => vec![
                rgb.red as f32 / 255.0,
                rgb.green as f32 / 255.0,
                rgb.blue as f32 / 255.0,
            ],
            Color::Grey(grey) => vec![grey.lightness as f32 / 255.0],
        }
    }

    fn to_normalized_pdf_components(&self) -> Vec<NormalizedF32> {
        self.to_pdf_components()
            .into_iter()
            .map(|n| NormalizedF32::new(n).unwrap())
            .collect()
    }

    fn get_pdf_color_space(&self) -> ColorSpace {
        match self {
            Color::Rgb(_) => ColorSpace::SRGB,
            Color::Grey(_) => ColorSpace::D65Gray,
        }
    }
}
