//! Dealing with colors and color spaces.

use crate::color::device_cmyk::DeviceCmyk;
use crate::color::luma::{DeviceGray, SGray};
use crate::color::rgb::{DeviceRgb, Srgb};
use crate::error::KrillaResult;
use crate::serialize::{FilterStream, SerializerContext};
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::DerefMut;
use std::sync::Arc;

/// The PDF name for the device RGB color space.
pub(crate) const DEVICE_RGB: &'static str = "DeviceRGB";
/// The PDF name for the device gray color space.
pub(crate) const DEVICE_GRAY: &'static str = "DeviceGray";
/// The PDF name for the device CMYK color space.
pub(crate) const DEVICE_CMYK: &'static str = "DeviceCMYK";

/// An internal helper trait to more easily deal with colors
/// of different color spaces.
pub(crate) trait InternalColor {
    /// Return the components of the color as a normalized f32.
    fn to_pdf_color(&self) -> impl IntoIterator<Item = f32>;
    /// Return the color space of the color.
    fn color_space(&self, no_device_cs: bool) -> ColorSpaceType;
}

#[allow(private_bounds)]
/// A color space and it's associated color.
pub trait ColorSpace: Debug + Hash + Eq + PartialEq + Clone + Copy {
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
    /// A luma-based color.
    Luma(luma::Color),
    /// A device CMYK color.
    DeviceCmyk(device_cmyk::Color),
}

impl Color {
    pub(crate) fn to_pdf_color(&self) -> Vec<f32> {
        match self {
            Color::Rgb(rgb) => rgb.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::Luma(luma) => luma.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::DeviceCmyk(cmyk) => cmyk.to_pdf_color().into_iter().collect::<Vec<_>>(),
        }
    }

    pub(crate) fn color_space(&self, no_device_cs: bool) -> ColorSpaceType {
        match self {
            Color::Rgb(rgb) => rgb.color_space(no_device_cs),
            Color::Luma(luma) => luma.color_space(no_device_cs),
            Color::DeviceCmyk(cmyk) => cmyk.color_space(no_device_cs),
        }
    }
}

/// Device CMYK colors.
pub mod device_cmyk {
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

    impl Into<super::Color> for Color {
        fn into(self) -> crate::object::color::Color {
            super::Color::DeviceCmyk(self)
        }
    }

    impl Default for Color {
        fn default() -> Self {
            Color::new(0, 0, 0, 255)
        }
    }

    impl InternalColor for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [
                self.0 as f32 / 255.0,
                self.1 as f32 / 255.0,
                self.2 as f32 / 255.0,
                self.3 as f32 / 255.0,
            ]
        }

        fn color_space(&self, _: bool) -> ColorSpaceType {
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
    use crate::serialize::{Object, SerializerContext};
    use std::sync::Arc;

    use crate::chunk_container::ChunkContainer;
    use crate::error::KrillaResult;
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
    }

    impl Into<super::Color> for Color {
        fn into(self) -> crate::object::color::Color {
            super::Color::Rgb(self)
        }
    }

    impl InternalColor for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [
                self.0 as f32 / 255.0,
                self.1 as f32 / 255.0,
                self.2 as f32 / 255.0,
            ]
        }

        fn color_space(&self, no_device_cs: bool) -> ColorSpaceType {
            if no_device_cs {
                ColorSpaceType::Srgb(Srgb)
            } else {
                ColorSpaceType::DeviceRgb(DeviceRgb)
            }
        }
    }

    /// The ICC profile for the SRGB color space.
    static SRGB_ICC: &[u8] = include_bytes!("../../assets/icc/sRGB-v4.icc");

    /// The RGB color space. Depending on whether the `no_device_cs` serialize option is set,
    /// this will internally be encoded either using the PDF `DeviceRgb` color space, or in the
    /// SRGB color space using an ICC profile.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Rgb;

    impl ColorSpace for Rgb {
        type Color = Color;
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
}

/// Luma (= gray-scale) colors.
pub mod luma {
    use crate::chunk_container::ChunkContainer;
    use crate::error::KrillaResult;
    use crate::object::color::{ColorSpace, ColorSpaceType, ICCBasedColorSpace, InternalColor};
    use crate::serialize::{Object, SerializerContext};
    use pdf_writer::{Chunk, Ref};
    use std::sync::Arc;

    /// An luma color.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color(u8);

    impl Default for Color {
        fn default() -> Self {
            Color::black()
        }
    }

    impl Color {
        pub fn new(lightness: u8) -> Self {
            Color(lightness)
        }

        pub fn black() -> Self {
            Self::new(0)
        }

        pub fn white() -> Self {
            Self::new(255)
        }
    }

    impl Into<super::Color> for Color {
        fn into(self) -> crate::object::color::Color {
            super::Color::Luma(self)
        }
    }

    impl InternalColor for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [self.0 as f32 / 255.0]
        }

        fn color_space(&self, no_device_cs: bool) -> ColorSpaceType {
            if no_device_cs {
                ColorSpaceType::SGray(SGray)
            } else {
                ColorSpaceType::DeviceGray(DeviceGray)
            }
        }
    }

    /// The ICC profile for the SGray color space.
    pub(crate) static GREY_ICC: &[u8] = include_bytes!("../../assets/icc/sGrey-v4.icc");

    /// The luma color space. Depending on whether the `no_device_cs` serialize option is set,
    /// this will internally be encoded either using the PDF `DeviceGray` color space, or in the
    /// SGray color space using an ICC profile.
    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Luma;

    impl ColorSpace for Luma {
        type Color = Color;
    }

    /// The SGray color space.
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

    impl ColorSpace for DeviceGray {
        type Color = Color;
    }
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
    use crate::object::color::luma::SGray;
    use crate::object::color::rgb::Srgb;
    use crate::resource::ColorSpaceResource;
    use crate::serialize::SerializerContext;

    use krilla_macros::snapshot;

    #[snapshot]
    fn color_space_sgray(sc: &mut SerializerContext) {
        sc.add_object(ColorSpaceResource::SGray(SGray)).unwrap();
    }

    #[snapshot]
    fn color_space_srgb(sc: &mut SerializerContext) {
        sc.add_object(ColorSpaceResource::Srgb(Srgb)).unwrap();
    }
}
