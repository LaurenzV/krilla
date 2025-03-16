//! Dealing with colors and color spaces.
//!
//! # Color spaces
//!
//! krilla currently supports three color spaces:
//! - Rgb (including linear RGB)
//! - Luma
//! - CMYK
//!
//! Each color space is associated with its specific color type, which you can use to create new
//! instances of a specific color in that color space.
//!
//! # Representation of colors
//!
//! When specifying colors, it is important to understand the distinction between device-dependent
//! and decide-independent color specification. What follows is only a very brief
//! explanation, if you want to dive into more details, please look for
//! appropriate resources on the web.
//!
//! When specifying colors in a *device-dependent way*, if I instruct the program to draw
//! the RGB color (145, 120, 45), then the program will use these literal values to activate
//! the R/G/B lights to achieve displaying a certain color. The problem is that specifying
//! colors in such a way can lead to slightly different results when actually displaying it,
//! depending on the screen that is used, since each screen is calibrated differently and
//! based on different display technologies. This is especially critical for printers, where
//! different values for CMYK colors might result in different-looking colors when being printed.
//!
//! This is why there is also the option to specify colors in a *device-independent* way,
//! which basically means that the color value (145, 120, 45) is represented in a well-specified
//! color space, and each device can then convert the colors to their native color space
//! so that they match the representation in the given color space as closely as possible.
//! This should lead to a more accurate color representation across different screens.
//!
//! In 90% of the cases, it is totally fine to just use a device-dependent colorspace, and it's
//! what krilla does by default. However, if you do care about that, then you can set the
//! `no_device_cs` property to true, in which case krilla will embed an ICC profile for the
//! sgrey and srgb color space (for luma and rgb colors, respectively). If a CMYK profile
//! was provided to the serialize settings, this will be used for CMYK colors. Otherwise,
//! it will fall back to device CMYK.

use std::fmt::Debug;
use std::hash::Hash;


use crate::configure::ValidationError;
use crate::graphics::icc::ICCBasedColorSpace;
use crate::serialize::SerializeContext;

/// The PDF name for the device RGB color space.
pub(crate) const DEVICE_RGB: &str = "DeviceRGB";
/// The PDF name for the device gray color space.
pub(crate) const DEVICE_GRAY: &str = "DeviceGray";
/// The PDF name for the device CMYK color space.
pub(crate) const DEVICE_CMYK: &str = "DeviceCMYK";

/// A wrapper enum that can hold colors from different color spaces.
#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub(crate) enum Color {
    /// An RGB-based color.
    Rgb(rgb::Color),
    /// A luma-based color.
    Luma(luma::Color),
    /// A device CMYK color.
    Cmyk(cmyk::Color),
}

impl Color {
    pub(crate) fn to_pdf_color(self) -> Vec<f32> {
        match self {
            Color::Rgb(rgb) => rgb.to_pdf_color().to_vec(),
            Color::Luma(l) => vec![l.to_pdf_color()],
            Color::Cmyk(cmyk) => cmyk.to_pdf_color().to_vec(),
        }
    }

    pub(crate) fn color_space(&self, sc: &mut SerializeContext) -> ColorSpace {
        match self {
            Color::Rgb(r) => r.color_space(sc.serialize_settings().no_device_cs),
            Color::Luma(_) => luma::Color::color_space(sc.serialize_settings().no_device_cs),
            Color::Cmyk(_) => {
                let color_space = cmyk::Color::color_space(&sc.serialize_settings());
                if color_space == ColorSpace::DeviceCmyk {
                    sc.register_validation_error(ValidationError::MissingCMYKProfile);
                }
                color_space
            }
        }
    }
}

/// Gray-scale colors.
pub mod luma {
    use crate::graphics::color::ColorSpace;

    /// A luma color.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color(pub(crate) u8);

    impl Color {
        /// Create a new luma color.
        pub fn new(lightness: u8) -> Color {
            Color(lightness)
        }

        /// Create a black luma color.
        pub fn black() -> Self {
            Self::new(0)
        }

        /// Create a white RGB color.
        pub fn white() -> Self {
            Self::new(255)
        }

        pub(crate) fn to_pdf_color(self) -> f32 {
            self.0 as f32 / 255.0
        }

        pub(crate) fn color_space(no_device_cs: bool) -> ColorSpace {
            if no_device_cs {
                ColorSpace::Luma
            } else {
                ColorSpace::DeviceGray
            }
        }
    }

    impl From<Color> for super::Color {
        fn from(val: Color) -> Self {
            super::Color::Luma(val)
        }
    }

    impl Default for Color {
        fn default() -> Self {
            Color::new(0)
        }
    }
}

/// CMYK colors.
pub mod cmyk {
    use crate::graphics::color::ColorSpace;
    use crate::graphics::icc::ICCBasedColorSpace;
    use crate::SerializeSettings;

    /// A CMYK color.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color(pub(crate) u8, pub(crate) u8, pub(crate) u8, pub(crate) u8);

    impl Color {
        /// Create a new CMYK color.
        pub fn new(cyan: u8, magenta: u8, yellow: u8, black: u8) -> Color {
            Color(cyan, magenta, yellow, black)
        }

        pub(crate) fn to_pdf_color(self) -> [f32; 4] {
            [
                self.0 as f32 / 255.0,
                self.1 as f32 / 255.0,
                self.2 as f32 / 255.0,
                self.3 as f32 / 255.0,
            ]
        }

        pub(crate) fn color_space(ss: &SerializeSettings) -> ColorSpace {
            if ss.no_device_cs {
                ss.clone()
                    .cmyk_profile
                    .map(|p| ColorSpace::Cmyk(ICCBasedColorSpace::<4>(p.clone())))
                    .unwrap_or(ColorSpace::DeviceCmyk)
            } else {
                ColorSpace::DeviceCmyk
            }
        }
    }

    impl From<Color> for super::Color {
        fn from(val: Color) -> Self {
            super::Color::Cmyk(val)
        }
    }

    impl Default for Color {
        fn default() -> Self {
            Color::new(0, 0, 0, 255)
        }
    }
}

/// RGB colors.
pub mod rgb {
    use crate::graphics::color::ColorSpace;

    /// An RGB color.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color(pub(crate) u8, pub(crate) u8, pub(crate) u8);

    impl Default for Color {
        fn default() -> Self {
            Color::black()
        }
    }

    impl Color {
        /// Create a new RGB color.
        pub fn new(red: u8, green: u8, blue: u8) -> Self {
            Color(red, green, blue)
        }

        /// Create a new linear RGB color.
        pub fn new_linear(red: u8, green: u8, blue: u8) -> Self {
            Color(red, green, blue)
        }

        /// Create a black RGB color.
        pub fn black() -> Self {
            Self::new(0, 0, 0)
        }

        /// Create a white RGB color.
        pub fn white() -> Self {
            Self::new(255, 255, 255)
        }

        /// The `red` component of the color.
        pub fn red(&self) -> u8 {
            self.0
        }

        /// The `green` component of the color.
        pub fn green(&self) -> u8 {
            self.1
        }

        /// The `blue` component of the color.
        pub fn blue(&self) -> u8 {
            self.2
        }

        pub(crate) fn to_pdf_color(self) -> [f32; 3] {
            [
                self.0 as f32 / 255.0,
                self.1 as f32 / 255.0,
                self.2 as f32 / 255.0,
            ]
        }

        pub(crate) fn color_space(&self, no_device_cs: bool) -> ColorSpace {
            Color::rgb_color_space(no_device_cs)
        }

        // TODO: Rename
        pub(crate) fn rgb_color_space(no_device_cs: bool) -> ColorSpace {
            if no_device_cs {
                ColorSpace::Srgb
            } else {
                ColorSpace::DeviceRgb
            }
        }
    }

    impl From<Color> for super::Color {
        fn from(val: Color) -> Self {
            super::Color::Rgb(val)
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) enum ColorSpace {
    DeviceRgb,
    DeviceGray,
    DeviceCmyk,
    Srgb,
    Luma,
    Cmyk(ICCBasedColorSpace<4>),
}
