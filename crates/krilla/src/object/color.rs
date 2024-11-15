//! Dealing with colors and color spaces.
//!
//! # Color spaces
//!
//! krilla currently supports three color spaces:
//! - Rgb
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

use crate::object::{ChunkContainerFn, Object};
use crate::resource::RegisterableResource;
use crate::serialize::SerializerContext;
use crate::stream::FilterStream;
use crate::util::Prehashed;
use crate::validation::ValidationError;
use pdf_writer::{Buf, Chunk, Finish, Name, Ref};
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
    /// A luma-based color.
    Luma(luma::Color),
    /// A device CMYK color.
    Cmyk(cmyk::Color),
}

impl Color {
    pub(crate) fn to_pdf_color(self) -> Vec<f32> {
        match self {
            Color::Rgb(rgb) => rgb.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::Luma(l) => l.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::Cmyk(cmyk) => cmyk.to_pdf_color().into_iter().collect::<Vec<_>>(),
        }
    }

    pub(crate) fn color_space(&self, sc: &mut SerializerContext) -> ColorSpace {
        match self {
            Color::Rgb(_) => rgb::Color::color_space(sc.serialize_settings().no_device_cs),
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

        pub(crate) fn to_pdf_color(self) -> impl IntoIterator<Item = f32> {
            [self.0 as f32 / 255.0]
        }

        pub(crate) fn color_space(no_device_cs: bool) -> ColorSpace {
            if no_device_cs {
                ColorSpace::Gray
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

        pub(crate) fn to_pdf_color(self) -> impl IntoIterator<Item = f32> {
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

        /// Create a black RGB color.
        pub fn black() -> Self {
            Self::new(0, 0, 0)
        }

        /// Create a white RGB color.
        pub fn white() -> Self {
            Self::new(255, 255, 255)
        }

        pub(crate) fn to_pdf_color(self) -> impl IntoIterator<Item = f32> {
            vec![
                self.0 as f32 / 255.0,
                self.1 as f32 / 255.0,
                self.2 as f32 / 255.0,
            ]
        }

        pub(crate) fn color_space(no_device_cs: bool) -> ColorSpace {
            if no_device_cs {
                ColorSpace::Rgb
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
    Rgb,
    Gray,
    Cmyk(ICCBasedColorSpace<4>),
}

#[derive(Clone)]
struct Repr {
    data: Arc<dyn AsRef<[u8]> + Send + Sync>,
    metadata: ICCMetadata,
}

impl Debug for Repr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ICC Profile")
    }
}

impl Hash for Repr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.as_ref().as_ref().hash(state);
        self.metadata.hash(state);
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) enum ICCProfileWrapper {
    Luma(ICCProfile<1>),
    Rgb(ICCProfile<3>),
    Cmyk(ICCProfile<4>),
}

impl ICCProfileWrapper {
    pub fn metadata(&self) -> &ICCMetadata {
        match self {
            ICCProfileWrapper::Luma(l) => l.metadata(),
            ICCProfileWrapper::Rgb(r) => r.metadata(),
            ICCProfileWrapper::Cmyk(c) => c.metadata(),
        }
    }
}

impl Object for ICCProfileWrapper {
    fn chunk_container(&self) -> ChunkContainerFn {
        Box::new(|cc| &mut cc.icc_profiles)
    }

    fn serialize(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        match self {
            ICCProfileWrapper::Luma(l) => l.serialize(sc, root_ref),
            ICCProfileWrapper::Rgb(r) => r.serialize(sc, root_ref),
            ICCProfileWrapper::Cmyk(c) => c.serialize(sc, root_ref),
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
    pub fn new(data: Arc<dyn AsRef<[u8]> + Send + Sync>) -> Option<Self> {
        let metadata = ICCMetadata::from_data(data.as_ref().as_ref())?;

        if metadata.color_space.num_components() != C {
            return None;
        }

        Some(Self(Arc::new(Prehashed::new(Repr { data, metadata }))))
    }

    pub(crate) fn metadata(&self) -> &ICCMetadata {
        &self.0.metadata
    }
}

impl<const C: u8> Object for ICCProfile<C> {
    fn chunk_container(&self) -> ChunkContainerFn {
        Box::new(|cc| &mut cc.icc_profiles)
    }

    fn serialize(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();
        let icc_stream = FilterStream::new_from_binary_data(
            self.0.deref().data.as_ref().as_ref(),
            &sc.serialize_settings(),
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
    pub fn num_components(&self) -> u8 {
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
    pub fn from_data(data: &[u8]) -> Option<Self> {
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

impl RegisterableResource<crate::resource::ColorSpace> for ICCBasedColorSpace<4> {}
impl RegisterableResource<crate::resource::ColorSpace> for ICCBasedColorSpace<3> {}
impl RegisterableResource<crate::resource::ColorSpace> for ICCBasedColorSpace<1> {}

#[cfg(test)]
mod tests {

    use crate::serialize::SerializerContext;

    use crate::page::Page;
    use crate::path::Fill;
    use crate::resource::Resource;
    use crate::surface::Surface;
    use crate::tests::{cmyk_fill, rect_to_path, red_fill};
    use krilla_macros::{snapshot, visreg};

    #[snapshot]
    fn color_space_sgray(sc: &mut SerializerContext) {
        sc.add_resource(Resource::Gray);
    }

    #[snapshot]
    fn color_space_srgb(sc: &mut SerializerContext) {
        sc.add_resource(Resource::Rgb);
    }

    #[snapshot(single_page, settings_18)]
    fn icc_v2_srgb(page: &mut Page) {
        let mut surface = page.surface();
        surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), red_fill(1.0));
    }

    #[snapshot(single_page, settings_18)]
    fn icc_v2_sgrey(page: &mut Page) {
        let mut surface = page.surface();
        surface.fill_path(&rect_to_path(50.0, 50.0, 100.0, 100.0), Fill::default());
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

/// Stores either the name of one of the default color spaces (i.e. DeviceRGB), or
/// a reference to a color space in the PDF.
#[derive(Copy, Clone)]
pub(crate) enum CSWrapper {
    Ref(Ref),
    Name(Name<'static>),
}

impl pdf_writer::Primitive for CSWrapper {
    fn write(self, buf: &mut Buf) {
        match self {
            CSWrapper::Ref(r) => r.write(buf),
            CSWrapper::Name(n) => n.write(buf),
        }
    }
}
