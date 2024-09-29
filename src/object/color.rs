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
//! sgrey and srgb color space (for luma and rgb colors, respectively). If a CMYK profile
//! was provided to the serialize settings, this will be used for CMYK colors. Otherwise,
//! it will fall back to device CMYK.

use crate::object::{ChunkContainerFn, Object};
use crate::resource::RegisterableResource;
use crate::serialize::{FilterStream, SerializerContext};
use crate::util::Prehashed;
use crate::validation::ValidationError;
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

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
    /// A device CMYK color.
    Cmyk(cmyk::Color),
}

impl Color {
    pub(crate) fn to_pdf_color(self, allow_gray: bool) -> Vec<f32> {
        match self {
            Color::Rgb(rgb) => rgb.to_pdf_color(allow_gray).into_iter().collect::<Vec<_>>(),
            Color::Cmyk(cmyk) => cmyk.to_pdf_color().into_iter().collect::<Vec<_>>(),
        }
    }

    pub(crate) fn color_space(&self, sc: &mut SerializerContext, allow_gray: bool) -> ColorSpace {
        match self {
            Color::Rgb(rgb) => rgb.color_space(sc.serialize_settings.no_device_cs, allow_gray),
            Color::Cmyk(cmyk) => {
                let color_space = cmyk.color_space(&sc.serialize_settings);
                if color_space == ColorSpace::DeviceCmyk {
                    sc.register_validation_error(ValidationError::MissingCMYKProfile);
                }
                color_space
            }
        }
    }
}

/// CMYK colors.
pub mod cmyk {
    use crate::color::ICCBasedColorSpace;
    use crate::object::color::ColorSpace;
    use crate::SerializeSettings;

    /// A CMYK color.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color(pub(crate) u8, pub(crate) u8, pub(crate) u8, pub(crate) u8);

    impl Color {
        /// Create a new CMYK color.
        pub fn new(cyan: u8, magenta: u8, yellow: u8, black: u8) -> Color {
            Color(cyan, magenta, yellow, black)
        }

        pub(crate) fn to_pdf_color(self) -> impl IntoIterator<Item = f32> {
            [
                self.0 as f32 / 255.0,
                self.1 as f32 / 255.0,
                self.2 as f32 / 255.0,
                self.3 as f32 / 255.0,
            ]
        }

        pub(crate) fn color_space(&self, ss: &SerializeSettings) -> ColorSpace {
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
    use crate::object::color::ColorSpace;

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

        // For gradients, we don't want to coerce to luma if possible, because gradients
        // require the number of components for each stop to be the same. Because of this
        // we always use 3 components for RGB and 4 components for CMYK. No automatic
        // detection of greyscale colors.
        pub(crate) fn to_pdf_color(self, allow_gray: bool) -> impl IntoIterator<Item = f32> {
            if self.is_gray_scale() && allow_gray {
                vec![self.0 as f32 / 255.0]
            } else {
                vec![
                    self.0 as f32 / 255.0,
                    self.1 as f32 / 255.0,
                    self.2 as f32 / 255.0,
                ]
            }
        }

        pub(crate) fn luma_based_color_space(no_device_cs: bool) -> ColorSpace {
            if no_device_cs {
                ColorSpace::Gray
            } else {
                ColorSpace::DeviceGray
            }
        }

        pub(crate) fn rgb_based_color_space(no_device_cs: bool) -> ColorSpace {
            if no_device_cs {
                ColorSpace::Rgb
            } else {
                ColorSpace::DeviceRgb
            }
        }

        pub(crate) fn color_space(&self, no_device_cs: bool, allow_gray: bool) -> ColorSpace {
            if self.is_gray_scale() && allow_gray {
                Self::luma_based_color_space(no_device_cs)
            } else {
                Self::rgb_based_color_space(no_device_cs)
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
    Rgb,
    Gray,
    Cmyk(ICCBasedColorSpace<4>),
}

#[derive(Clone)]
struct Repr(Arc<dyn AsRef<[u8]> + Send + Sync>);

impl Debug for Repr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ICC Profile")
    }
}

impl Hash for Repr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ref().as_ref().hash(state);
    }
}

/// An ICC profile.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ICCProfile<const C: u8>(Arc<Prehashed<Repr>>);

impl<const C: u8> ICCProfile<C> {
    /// Create a new ICC profile.
    pub fn new(data: Arc<dyn AsRef<[u8]> + Send + Sync>) -> Self {
        Self(Arc::new(Prehashed::new(Repr(data))))
    }
}

impl<const C: u8> Object for ICCProfile<C> {
    fn chunk_container(&self) -> ChunkContainerFn {
        Box::new(|cc| &mut cc.icc_profiles)
    }

    fn serialize(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();
        let icc_stream = FilterStream::new_from_binary_data(
            self.0.deref().0.as_ref().as_ref(),
            &sc.serialize_settings,
        );

        let mut icc_profile = chunk.icc_profile(root_ref, icc_stream.encoded_data());
        icc_profile.n(C as i32).range([0.0, 1.0].repeat(C as usize));

        icc_stream.write_filters(icc_profile.deref_mut().deref_mut());
        icc_profile.finish();

        chunk
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) struct ICCBasedColorSpace<const C: u8>(pub(crate) ICCProfile<C>);

impl<const C: u8> Object for ICCBasedColorSpace<C> {
    fn chunk_container(&self) -> ChunkContainerFn {
        Box::new(|cc| &mut cc.color_spaces)
    }

    fn serialize(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let icc_ref = sc.add_object(self.0.clone());

        let mut chunk = Chunk::new();

        let mut array = chunk.indirect(root_ref).array();
        array.item(Name(b"ICCBased"));
        array.item(icc_ref);
        array.finish();

        chunk
    }
}

impl RegisterableResource<crate::resource::ColorSpace> for ICCBasedColorSpace<4> {}
impl RegisterableResource<crate::resource::ColorSpace> for ICCBasedColorSpace<3> {}
impl RegisterableResource<crate::resource::ColorSpace> for ICCBasedColorSpace<1> {}

#[cfg(test)]
mod tests {

    use crate::serialize::SerializerContext;

    use crate::resource::Resource;
    use crate::surface::Surface;
    use crate::tests::{cmyk_fill, rect_to_path};
    use krilla_macros::{snapshot, visreg};

    #[snapshot]
    fn color_space_sgray(sc: &mut SerializerContext) {
        sc.add_resource(Resource::Gray);
    }

    #[snapshot]
    fn color_space_srgb(sc: &mut SerializerContext) {
        sc.add_resource(Resource::Rgb);
    }

    #[visreg(all)]
    fn cmyk_color(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

        surface.fill_path(&path, cmyk_fill(1.0));
    }

    #[visreg(all, settings_6)]
    fn cmyk_with_icc(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

        surface.fill_path(&path, cmyk_fill(1.0));
    }
}
