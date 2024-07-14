use crate::util::deflate;
use once_cell::sync::Lazy;

// The ICC profiles.
static SRGB_ICC_DEFLATED: Lazy<Vec<u8>> = Lazy::new(|| deflate(include_bytes!("icc/sRGB-v4.icc")));
static GREY_ICC_DEFLATED: Lazy<Vec<u8>> = Lazy::new(|| deflate(include_bytes!("icc/sGrey-v4.icc")));

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Grey {
    pub lightness: u8,
}

#[derive(Debug, Hash, Eq, PartialEq)]
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
