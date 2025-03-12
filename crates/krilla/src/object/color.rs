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
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use pdf_writer::{Chunk, Finish, Name, Ref};

use crate::configure::ValidationError;
use crate::object::{Cacheable, ChunkContainerFn, Resourceable};
use crate::resource;
use crate::serialize::SerializeContext;
use crate::stream::{deflate_encode, FilterStreamBuilder};
use crate::util::Prehashed;

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
    use crate::object::color::ColorSpace;

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

#[derive(Clone, Hash, Debug)]
struct Repr {
    data: Vec<u8>,
    metadata: ICCMetadata,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) enum GenericICCProfile {
    Luma(ICCProfile<1>),
    Rgb(ICCProfile<3>),
    Cmyk(ICCProfile<4>),
}

impl GenericICCProfile {
    pub(crate) fn metadata(&self) -> &ICCMetadata {
        match self {
            GenericICCProfile::Luma(l) => l.metadata(),
            GenericICCProfile::Rgb(r) => r.metadata(),
            GenericICCProfile::Cmyk(c) => c.metadata(),
        }
    }
}

impl Cacheable for GenericICCProfile {
    fn chunk_container(&self) -> ChunkContainerFn {
        |cc| &mut cc.icc_profiles
    }

    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk {
        match self {
            GenericICCProfile::Luma(l) => l.serialize(sc, root_ref),
            GenericICCProfile::Rgb(r) => r.serialize(sc, root_ref),
            GenericICCProfile::Cmyk(c) => c.serialize(sc, root_ref),
        }
    }
}

/// An ICC profile.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ICCProfile<const C: u8>(Arc<Prehashed<Repr>>);

impl<const C: u8> ICCProfile<C> {
    /// Create a new ICC profile.
    ///
    /// Returns `None` if the metadata of the profile couldn't be read or if the
    /// number of channels of the underlying data does not correspond to the one
    /// indicated in the constant parameter.
    pub fn new(data: &[u8]) -> Option<Self> {
        let metadata = ICCMetadata::from_data(data)?;

        if metadata.color_space.num_components() != C {
            return None;
        }

        Some(Self(Arc::new(Prehashed::new(Repr {
            data: deflate_encode(data),
            metadata,
        }))))
    }

    pub(crate) fn metadata(&self) -> &ICCMetadata {
        &self.0.metadata
    }
}

impl<const C: u8> Cacheable for ICCProfile<C> {
    fn chunk_container(&self) -> ChunkContainerFn {
        |cc| &mut cc.icc_profiles
    }

    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();
        let icc_stream = FilterStreamBuilder::new_from_deflated(&self.0.deref().data)
            .finish(&sc.serialize_settings());

        let mut icc_profile = chunk.icc_profile(root_ref, icc_stream.encoded_data());
        icc_profile.n(C as i32).range([0.0, 1.0].repeat(C as usize));
        icc_stream.write_filters(icc_profile.deref_mut().deref_mut());
        icc_profile.finish();

        chunk
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) struct ICCBasedColorSpace<const C: u8>(pub(crate) ICCProfile<C>);

impl<const C: u8> Cacheable for ICCBasedColorSpace<C> {
    fn chunk_container(&self) -> ChunkContainerFn {
        |cc| &mut cc.color_spaces
    }

    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk {
        let icc_ref = sc.register_cacheable(self.0.clone());

        let mut chunk = Chunk::new();

        let mut array = chunk.indirect(root_ref).array();
        array.item(Name(b"ICCBased"));
        array.item(icc_ref);
        array.finish();

        chunk
    }
}

impl<const C: u8> Resourceable for ICCBasedColorSpace<C> {
    type Resource = resource::ColorSpace;
}

#[derive(Clone, Hash, Debug, Eq, PartialEq)]
pub(crate) enum ICCColorSpace {
    Xyz,
    Lab,
    Luv,
    Ycbr,
    Yxy,
    Lms,
    Rgb,
    Gray,
    Hsv,
    Hls,
    Cmyk,
    Cmy,
    OneClr,
    ThreeClr,
    FourClr,
    // There are more, but those should be the most important
    // ones.
}

impl ICCColorSpace {
    pub(crate) fn num_components(&self) -> u8 {
        match self {
            ICCColorSpace::Xyz => 3,
            ICCColorSpace::Lab => 3,
            ICCColorSpace::Luv => 3,
            ICCColorSpace::Ycbr => 3,
            ICCColorSpace::Yxy => 3,
            ICCColorSpace::Lms => 3,
            ICCColorSpace::Rgb => 3,
            ICCColorSpace::Gray => 1,
            ICCColorSpace::Hsv => 3,
            ICCColorSpace::Hls => 3,
            ICCColorSpace::Cmyk => 4,
            ICCColorSpace::Cmy => 3,
            ICCColorSpace::OneClr => 1,
            ICCColorSpace::ThreeClr => 3,
            ICCColorSpace::FourClr => 4,
        }
    }
}

impl TryFrom<u32> for ICCColorSpace {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x58595A20 => Ok(ICCColorSpace::Xyz),
            0x4C616220 => Ok(ICCColorSpace::Lab),
            0x4C757620 => Ok(ICCColorSpace::Luv),
            0x59436272 => Ok(ICCColorSpace::Ycbr),
            0x59787920 => Ok(ICCColorSpace::Yxy),
            0x4C4D5320 => Ok(ICCColorSpace::Lms),
            0x52474220 => Ok(ICCColorSpace::Rgb),
            0x47524159 => Ok(ICCColorSpace::Gray),
            0x48535620 => Ok(ICCColorSpace::Hsv),
            0x484C5320 => Ok(ICCColorSpace::Hls),
            0x434D594B => Ok(ICCColorSpace::Cmyk),
            0x434D5920 => Ok(ICCColorSpace::Cmy),
            0x31434C52 => Ok(ICCColorSpace::OneClr),
            0x33434C52 => Ok(ICCColorSpace::ThreeClr),
            0x34434C52 => Ok(ICCColorSpace::FourClr),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Hash, Debug, Eq, PartialEq)]
pub(crate) struct ICCMetadata {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) color_space: ICCColorSpace,
}

impl ICCMetadata {
    pub(crate) fn from_data(data: &[u8]) -> Option<Self> {
        let major = *data.get(8)?;
        let minor = *data.get(9)? >> 4;
        let color_space = {
            let marker = u32::from_be_bytes(data.get(16..20)?.try_into().ok()?);
            ICCColorSpace::try_from(marker).ok()?
        };
        Some(Self {
            major,
            minor,
            color_space,
        })
    }
}
