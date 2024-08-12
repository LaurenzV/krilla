use crate::resource::ColorSpaceEnum;
use crate::serialize::{Object, SerializerContext};
use crate::util::Prehashed;
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub const DEVICE_RGB: &'static str = "DeviceRGB";
pub const DEVICE_GRAY: &'static str = "DeviceGray";
pub const DEVICE_CMYK: &'static str = "DeviceCMYK";

pub trait InternalColor {
    fn to_pdf_color(&self) -> impl IntoIterator<Item = f32>;
    fn color_space(&self, no_device_cs: bool) -> ColorSpaceEnum;
}

pub trait ColorSpace: Debug + Hash + Eq + PartialEq + Clone + Copy {
    type Color: InternalColor + Into<Color> + Debug + Clone + Copy + Default;
}

#[derive(Clone)]
struct ICCBasedColorSpace(Arc<dyn AsRef<[u8]>>, u8);

impl ICCBasedColorSpace {
    fn serialize_into(self, sc: &mut SerializerContext) -> (Ref, Chunk) {
        let root_ref = sc.new_ref();
        let icc_ref = sc.new_ref();

        let mut chunk = Chunk::new();

        let mut array = chunk.indirect(root_ref).array();
        array.item(Name(b"ICCBased"));
        array.item(icc_ref);
        array.finish();

        let (stream, filter) = sc.get_binary_stream(self.0.as_ref().as_ref());

        chunk
            .icc_profile(icc_ref, &stream)
            .n(self.1 as i32)
            .range([0.0, 1.0].repeat(self.1 as usize))
            .filter(filter);

        (root_ref, chunk)
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum Color {
    Rgb(rgb::Color),
    Luma(luma::Color),
    DeviceCmyk(device_cmyk::Color),
}

impl Color {
    pub fn to_pdf_color(&self) -> Vec<f32> {
        match self {
            Color::Rgb(rgb) => rgb.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::Luma(luma) => luma.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::DeviceCmyk(cmyk) => cmyk.to_pdf_color().into_iter().collect::<Vec<_>>(),
        }
    }

    pub fn color_space(&self, no_device_cs: bool) -> ColorSpaceEnum {
        match self {
            Color::Rgb(rgb) => rgb.color_space(no_device_cs),
            Color::Luma(luma) => luma.color_space(no_device_cs),
            Color::DeviceCmyk(cmyk) => cmyk.color_space(no_device_cs),
        }
    }
}

pub mod device_cmyk {
    use crate::object::color_space::{ColorSpace, InternalColor};
    use crate::resource::ColorSpaceEnum;
    use crate::serialize::{Object, SerializerContext};
    use pdf_writer::{Chunk, Ref};

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
        fn into(self) -> crate::object::color_space::Color {
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

        fn color_space(&self, _: bool) -> ColorSpaceEnum {
            ColorSpaceEnum::DeviceCmyk(DeviceCmyk)
        }
    }

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct DeviceCmyk;

    impl ColorSpace for DeviceCmyk {
        type Color = Color;
    }

    impl Object for DeviceCmyk {
        fn serialize_into(self, _: &mut SerializerContext) -> (Ref, Chunk) {
            unreachable!()
        }
    }
}

pub mod rgb {
    use crate::object::color_space::{ColorSpace, ICCBasedColorSpace, InternalColor};
    use crate::resource::ColorSpaceEnum;
    use crate::serialize::{Object, SerializerContext};
    use std::sync::Arc;

    use pdf_writer::{Chunk, Finish, Name, Ref};

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color(pub(crate) u8, pub(crate) u8, pub(crate) u8);

    impl Default for Color {
        fn default() -> Self {
            Color::black()
        }
    }

    impl Color {
        pub fn new(red: u8, green: u8, blue: u8) -> Self {
            Color(red, green, blue)
        }

        pub fn black() -> Self {
            Self::new(0, 0, 0)
        }

        pub fn white() -> Self {
            Self::new(255, 255, 255)
        }
    }

    impl Into<super::Color> for Color {
        fn into(self) -> crate::object::color_space::Color {
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

        fn color_space(&self, no_device_cs: bool) -> ColorSpaceEnum {
            if no_device_cs {
                ColorSpaceEnum::Srgb(Srgb)
            } else {
                ColorSpaceEnum::DeviceRgb(DeviceRgb)
            }
        }
    }

    static SRGB_ICC: &[u8] = include_bytes!("../icc/sRGB-v4.icc");

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Rgb;

    impl ColorSpace for Rgb {
        type Color = Color;
    }

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub(crate) struct Srgb;

    impl Object for Srgb {
        fn serialize_into(self, sc: &mut SerializerContext) -> (Ref, Chunk) {
            let icc_based = ICCBasedColorSpace(Arc::new(SRGB_ICC), 3);
            icc_based.serialize_into(sc)
        }
    }

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct DeviceRgb;

    impl ColorSpace for DeviceRgb {
        type Color = Color;
    }

    impl Object for DeviceRgb {
        fn serialize_into(self, _: &mut SerializerContext) -> (Ref, Chunk) {
            unreachable!()
        }
    }
}

pub mod luma {
    use crate::object::color_space::{ColorSpace, ICCBasedColorSpace, InternalColor};
    use crate::resource::ColorSpaceEnum;
    use crate::serialize::{Object, SerializerContext};
    use pdf_writer::{Chunk, Finish, Name, Ref};
    use std::sync::Arc;

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
        fn into(self) -> crate::object::color_space::Color {
            super::Color::Luma(self)
        }
    }

    impl InternalColor for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [self.0 as f32 / 255.0]
        }

        fn color_space(&self, no_device_cs: bool) -> ColorSpaceEnum {
            if no_device_cs {
                ColorSpaceEnum::SGray(SGray)
            } else {
                ColorSpaceEnum::DeviceGray(DeviceGray)
            }
        }
    }

    pub static GREY_ICC: &[u8] = include_bytes!("../icc/sGrey-v4.icc");

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct SGray;

    impl ColorSpace for SGray {
        type Color = Color;
    }

    impl Object for SGray {
        fn serialize_into(self, sc: &mut SerializerContext) -> (Ref, Chunk) {
            let icc_based = ICCBasedColorSpace(Arc::new(GREY_ICC), 1);
            icc_based.serialize_into(sc)
        }
    }

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct DeviceGray;

    impl ColorSpace for DeviceGray {
        type Color = Color;
    }

    impl Object for DeviceGray {
        fn serialize_into(self, _: &mut SerializerContext) -> (Ref, Chunk) {
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::object::color_space::luma::SGray;
    use crate::object::color_space::rgb::Srgb;
    use crate::resource::ColorSpaceEnum;
    use crate::serialize::{SerializeSettings, SerializerContext};
    use crate::test_utils::check_snapshot;
    use fontdb::Database;

    fn sc() -> SerializerContext {
        let settings = SerializeSettings::default_test();
        SerializerContext::new(settings)
    }

    #[test]
    fn sgray() {
        let mut sc = sc();
        sc.add(ColorSpaceEnum::SGray(SGray));
        check_snapshot("color_space/sgray", sc.finish(&Database::new()).as_bytes());
    }

    #[test]
    fn srgb() {
        let mut sc = sc();
        sc.add(ColorSpaceEnum::Srgb(Srgb));
        check_snapshot("color_space/srgb", sc.finish(&Database::new()).as_bytes());
    }
}
