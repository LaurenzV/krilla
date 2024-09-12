//! Dealing with colors and color spaces.
//!
//! Unlike other graphics libraries, krilla does not use a single RGB color space that can be
//! used to draw content with, the reason being that PDF supports much more complex color
//! management, and krilla tried to expose at least some of that functionality, while still
//! trying to abstract over the nitty-gritty details that are part of dealing with colors in PDF.
//!
//! # Color spaces
//!
//! krilla currently supports two color spaces:
//! - Rgb (gray-scale colors will automatically be converted to the luma color space)
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
//! sgrey and srgb color space (for luma and rgb colors, respectively). At the moment, krilla
//! does not allow for custom CMYK ICC profiles, so CMYK colors are currently always encoded
//! in a device-dependent way.

use crate::color::cmyk::DeviceCmyk;
use crate::color::rgb::{DeviceGray, DeviceRgb, SGray, Srgb};
use crate::error::KrillaResult;
use crate::serialize::{FilterStream, SerializerContext};
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::DerefMut;
use std::sync::Arc;

/// The PDF name for the device RGB color space.
pub(crate) const DEVICE_RGB: &str = "DeviceRGB";
/// The PDF name for the device gray color space.
pub(crate) const DEVICE_GRAY: &str = "DeviceGray";
/// The PDF name for the device CMYK color space.
pub(crate) const DEVICE_CMYK: &str = "DeviceCMYK";

/// An internal helper trait to more easily deal with colors
/// of different color spaces.
pub(crate) trait InternalColor {
    /// Return the components of the color as a normalized f32.
    fn to_pdf_color(&self, is_gradient: bool) -> impl IntoIterator<Item = f32>;
    /// Return the color space of the color.
    fn color_space(&self, no_device_cs: bool, is_gradient: bool) -> ColorSpaceType;
}

#[allow(private_bounds)]
/// A color space and it's associated color.
pub trait ColorSpace: Debug + Hash + Eq + PartialEq + Clone + Copy {
    /// The associated color type of the color space.
    type Color: InternalColor + Into<Color> + Debug + Clone + Copy + Default;
}

#[derive(Clone)]
struct ICCBasedColorSpace(Arc<dyn AsRef<[u8]>>, u8);

impl ICCBasedColorSpace {
    fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        let icc_ref = sc.new_ref();

        let mut chunk = Chunk::new();

        let mut array = chunk.indirect(root_ref).array();
        array.item(Name(b"ICCBased"));
        array.item(icc_ref);
        array.finish();

        let icc_stream =
            FilterStream::new_from_binary_data(self.0.as_ref().as_ref(), &sc.serialize_settings);

        let mut icc_profile = chunk.icc_profile(icc_ref, icc_stream.encoded_data());
        icc_profile
            .n(self.1 as i32)
            .range([0.0, 1.0].repeat(self.1 as usize));

        icc_stream.write_filters(icc_profile.deref_mut().deref_mut());
        icc_profile.finish();

        Ok(chunk)
    }
}

/// A wrapper enum that can hold colors from different color spaces.
#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub(crate) enum Color {
    /// An RGB-based color.
    Rgb(rgb::Color),
    /// A device CMYK color.
    DeviceCmyk(cmyk::Color),
}

impl Color {
    pub(crate) fn to_pdf_color(self, is_gradient: bool) -> Vec<f32> {
        match self {
            Color::Rgb(rgb) => rgb
                .to_pdf_color(is_gradient)
                .into_iter()
                .collect::<Vec<_>>(),
            Color::DeviceCmyk(cmyk) => cmyk
                .to_pdf_color(is_gradient)
                .into_iter()
                .collect::<Vec<_>>(),
        }
    }

    pub(crate) fn color_space(&self, no_device_cs: bool, is_gradient: bool) -> ColorSpaceType {
        match self {
            Color::Rgb(rgb) => rgb.color_space(no_device_cs, is_gradient),
            Color::DeviceCmyk(cmyk) => cmyk.color_space(no_device_cs, is_gradient),
        }
    }
}

/// CMYK colors.
pub mod cmyk {
    use crate::object::color::{ColorSpace, ColorSpaceType, InternalColor};

    /// A CMYK color.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color(pub(crate) u8, pub(crate) u8, pub(crate) u8, pub(crate) u8);

    impl Color {
        /// Create a new CMYK color.
        pub fn new(cyan: u8, magenta: u8, yellow: u8, black: u8) -> Color {
            Color(cyan, magenta, yellow, black)
        }
    }

    impl From<Color> for super::Color {
        fn from(val: Color) -> Self {
            super::Color::DeviceCmyk(val)
        }
    }

    impl Default for Color {
        fn default() -> Self {
            Color::new(0, 0, 0, 255)
        }
    }

    impl InternalColor for Color {
        fn to_pdf_color(&self, _: bool) -> impl IntoIterator<Item = f32> {
            [
                self.0 as f32 / 255.0,
                self.1 as f32 / 255.0,
                self.2 as f32 / 255.0,
                self.3 as f32 / 255.0,
            ]
        }

        fn color_space(&self, _: bool, _: bool) -> ColorSpaceType {
            ColorSpaceType::DeviceCmyk(DeviceCmyk)
        }
    }

    /// The device CMYK color space.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct DeviceCmyk;

    impl ColorSpace for DeviceCmyk {
        type Color = Color;
    }
}

/// RGB colors.
pub mod rgb {
    use crate::object::color::{ColorSpace, ColorSpaceType, ICCBasedColorSpace, InternalColor};
    use crate::serialize::SerializerContext;
    use std::sync::Arc;

    use crate::chunk_container::ChunkContainer;
    use crate::error::KrillaResult;
    use crate::object::Object;
    use pdf_writer::{Chunk, Ref};

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

        /// Create a black RGB color.
        pub fn black() -> Self {
            Self::new(0, 0, 0)
        }

        /// Create a white RGB color.
        pub fn white() -> Self {
            Self::new(255, 255, 255)
        }

        /// Create a gray RGB color.
        pub fn gray(lightness: u8) -> Self {
            Self::new(lightness, lightness, lightness)
        }

        fn is_gray_scale(&self) -> bool {
            self.0 == self.1 && self.1 == self.2
        }
    }

    impl From<Color> for super::Color {
        fn from(val: Color) -> Self {
            super::Color::Rgb(val)
        }
    }

    impl InternalColor for Color {
        // For gradients, we don't want to coerce to luma if possible, because gradients
        // require the number of components for each stop to be the same. Because of this
        // we always use 3 components for RGB and 4 components for CMYK. No automatic
        // detection of greyscale colors.
        fn to_pdf_color(&self, is_gradient: bool) -> impl IntoIterator<Item = f32> {
            if self.is_gray_scale() && !is_gradient {
                vec![self.0 as f32 / 255.0]
            } else {
                vec![
                    self.0 as f32 / 255.0,
                    self.1 as f32 / 255.0,
                    self.2 as f32 / 255.0,
                ]
            }
        }

        fn color_space(&self, no_device_cs: bool, is_gradient: bool) -> ColorSpaceType {
            if self.is_gray_scale() && !is_gradient {
                Luma::color_space(no_device_cs)
            } else {
                Rgb::color_space(no_device_cs)
            }
        }
    }

    /// The ICC profile for the SRGB color space.
    static SRGB_ICC: &[u8] = include_bytes!("../icc/sRGB-v4.icc");
    /// The ICC profile for the sgray color space.
    static GREY_ICC: &[u8] = include_bytes!("../icc/sGrey-v4.icc");

    /// The RGB color space. Depending on whether the `no_device_cs` serialize option is set,
    /// this will internally be encoded either using the PDF `DeviceRgb` color space, or in the
    /// SRGB color space using an ICC profile.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Rgb;

    impl ColorSpace for Rgb {
        type Color = Color;
    }

    impl Rgb {
        pub(crate) fn color_space(no_device_cs: bool) -> ColorSpaceType {
            if no_device_cs {
                ColorSpaceType::Srgb(Srgb)
            } else {
                ColorSpaceType::DeviceRgb(DeviceRgb)
            }
        }
    }

    pub(crate) struct Luma;

    impl Luma {
        pub(crate) fn color_space(no_device_cs: bool) -> ColorSpaceType {
            if no_device_cs {
                ColorSpaceType::SGray(SGray)
            } else {
                ColorSpaceType::DeviceGray(DeviceGray)
            }
        }
    }

    /// The SRGB color space.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub(crate) struct Srgb;

    impl Object for Srgb {
        fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
            &mut cc.color_spaces
        }

        fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
            let icc_based = ICCBasedColorSpace(Arc::new(SRGB_ICC), 3);
            icc_based.serialize(sc, root_ref)
        }
    }

    /// The device RGB color space.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub(crate) struct DeviceRgb;

    impl ColorSpace for DeviceRgb {
        type Color = Color;
    }

    /// The sgray color space.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub(crate) struct SGray;

    impl Object for SGray {
        fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
            &mut cc.color_spaces
        }

        fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
            let icc_based = ICCBasedColorSpace(Arc::new(GREY_ICC), 1);
            icc_based.serialize(sc, root_ref)
        }
    }

    /// The device gray color space.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub(crate) struct DeviceGray;
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) enum ColorSpaceType {
    Srgb(Srgb),
    SGray(SGray),
    DeviceGray(DeviceGray),
    DeviceRgb(DeviceRgb),
    DeviceCmyk(DeviceCmyk),
}

#[cfg(test)]
mod tests {
    use crate::object::color::rgb::Srgb;
    use crate::resource::ColorSpaceResource;
    use crate::serialize::SerializerContext;

    use crate::color::rgb::SGray;
    use crate::surface::Surface;
    use crate::tests::{cmyk_fill, rect_to_path};
    use krilla_macros::{snapshot, visreg};

    #[snapshot]
    fn color_space_sgray(sc: &mut SerializerContext) {
        sc.add_object(ColorSpaceResource::SGray(SGray)).unwrap();
    }

    #[snapshot]
    fn color_space_srgb(sc: &mut SerializerContext) {
        sc.add_object(ColorSpaceResource::Srgb(Srgb)).unwrap();
    }

    #[visreg(all)]
    fn cmyk_color(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

        surface.fill_path(&path, cmyk_fill(1.0));
    }
}
