use crate::object::color_space::device_cmyk::DeviceCmyk;
use crate::object::color_space::device_gray::DeviceGray;
use crate::object::color_space::device_rgb::DeviceRgb;
use crate::object::color_space::sgray::SGray;
use crate::object::color_space::srgb::Srgb;
use crate::resource::ColorSpaceEnum;
use crate::serialize::Object;
use pdf_writer::Name;
use std::fmt::Debug;
use std::hash::Hash;

pub const DEVICE_RGB: &'static str = "DeviceRGB";
pub const DEVICE_GRAY: &'static str = "DeviceGray";
pub const DEVICE_CMYK: &'static str = "DeviceCMYK";

pub trait InternalColor<C>
where
    C: Clone + Copy + ColorSpace + Debug,
{
    fn to_pdf_color(&self) -> impl IntoIterator<Item = f32>;
}

pub trait ColorSpace:
    Object + Debug + Hash + Eq + PartialEq + Clone + Copy + Into<ColorSpaceEnum>
{
    type Color: InternalColor<Self> + Into<Color> + Debug + Clone + Copy + Default;
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum Color {
    Srgb(srgb::Color),
    SGray(sgray::Color),
    DeviceGray(device_gray::Color),
    DeviceRgb(device_rgb::Color),
    DeviceCmyk(device_cmyk::Color),
}

impl Color {
    pub fn to_pdf_color(&self) -> Vec<f32> {
        match self {
            Color::Srgb(srgb) => srgb.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::SGray(sgray) => sgray.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::DeviceGray(dg) => dg.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::DeviceRgb(dr) => dr.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::DeviceCmyk(dc) => dc.to_pdf_color().into_iter().collect::<Vec<_>>(),
        }
    }

    pub fn color_space(&self, no_device_cs: bool) -> ColorSpaceEnum {
        match self {
            Color::Srgb(_) => if no_device_cs {
                ColorSpaceEnum::Srgb(Srgb)
            }   else {
                ColorSpaceEnum::DeviceRgb(DeviceRgb)
            },
            Color::SGray(_) => if no_device_cs {
                ColorSpaceEnum::SGray(SGray)
            }   else {
                ColorSpaceEnum::DeviceGray(DeviceGray)
            },
            Color::DeviceGray(_) => ColorSpaceEnum::DeviceGray(DeviceGray),
            Color::DeviceRgb(_) => ColorSpaceEnum::DeviceRgb(DeviceRgb),
            Color::DeviceCmyk(_) => ColorSpaceEnum::DeviceCmyk(DeviceCmyk),
        }
    }
}

pub mod device_cmyk {
    use crate::object::color_space::{ColorSpace, InternalColor};
    use crate::resource::ColorSpaceEnum;
    use crate::serialize::{Object, SerializerContext};
    use pdf_writer::{Chunk, Ref};

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct DeviceCmyk;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color([u8; 4]);

    impl DeviceCmyk {
        pub fn new_cmyk(cyan: u8, magenta: u8, yellow: u8, black: u8) -> Color {
            Color([cyan, magenta, yellow, black])
        }
    }

    impl Into<super::Color> for Color {
        fn into(self) -> crate::object::color_space::Color {
            super::Color::DeviceCmyk(self)
        }
    }

    impl Default for Color {
        fn default() -> Self {
            DeviceCmyk::new_cmyk(0, 0, 0, 255)
        }
    }

    impl Into<ColorSpaceEnum> for DeviceCmyk {
        fn into(self) -> ColorSpaceEnum {
            ColorSpaceEnum::DeviceCmyk(self)
        }
    }

    impl ColorSpace for DeviceCmyk {
        type Color = Color;
    }

    impl InternalColor<DeviceCmyk> for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [
                self.0[0] as f32 / 255.0,
                self.0[1] as f32 / 255.0,
                self.0[2] as f32 / 255.0,
                self.0[3] as f32 / 255.0,
            ]
        }
    }

    impl Object for DeviceCmyk {
        fn serialize_into(self, _: &mut SerializerContext) -> (Ref, Chunk) {
            unreachable!()
        }
    }
}

pub mod device_rgb {
    use crate::object::color_space::{ColorSpace, InternalColor};
    use crate::resource::ColorSpaceEnum;
    use crate::serialize::{Object, SerializerContext};
    use pdf_writer::{Chunk, Ref};

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct DeviceRgb;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color([u8; 3]);

    impl DeviceRgb {
        pub fn new_rgb(red: u8, green: u8, blue: u8) -> Color {
            Color([red, green, blue])
        }

        pub fn black() -> Color {
            Self::new_rgb(0, 0, 0)
        }

        pub fn white() -> Color {
            Self::new_rgb(255, 255, 255)
        }
    }

    impl Into<super::Color> for Color {
        fn into(self) -> crate::object::color_space::Color {
            super::Color::DeviceRgb(self)
        }
    }

    impl Into<ColorSpaceEnum> for DeviceRgb {
        fn into(self) -> ColorSpaceEnum {
            ColorSpaceEnum::DeviceRgb(self)
        }
    }

    impl ColorSpace for DeviceRgb {
        type Color = Color;
    }

    impl Default for Color {
        fn default() -> Self {
            DeviceRgb::black()
        }
    }

    impl InternalColor<DeviceRgb> for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [
                self.0[0] as f32 / 255.0,
                self.0[1] as f32 / 255.0,
                self.0[2] as f32 / 255.0,
            ]
        }
    }

    impl Object for DeviceRgb {
        fn serialize_into(self, _: &mut SerializerContext) -> (Ref, Chunk) {
            unreachable!()
        }
    }
}

pub mod srgb {
    use crate::object::color_space::{ColorSpace, InternalColor};
    use crate::resource::ColorSpaceEnum;
    use crate::serialize::{Object, SerializerContext};

    use pdf_writer::{Chunk, Finish, Name, Ref};

    pub static SRGB_ICC: &[u8] = include_bytes!("../icc/sRGB-v4.icc");

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Srgb;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color([u8; 3]);

    impl Default for Color {
        fn default() -> Self {
            Srgb::black()
        }
    }

    impl Srgb {
        pub fn new_rgb(red: u8, green: u8, blue: u8) -> Color {
            Color([red, green, blue])
        }

        pub fn black() -> Color {
            Self::new_rgb(0, 0, 0)
        }

        pub fn white() -> Color {
            Self::new_rgb(255, 255, 255)
        }
    }

    impl Into<super::Color> for Color {
        fn into(self) -> crate::object::color_space::Color {
            super::Color::Srgb(self)
        }
    }

    impl Into<ColorSpaceEnum> for Srgb {
        fn into(self) -> ColorSpaceEnum {
            ColorSpaceEnum::Srgb(self)
        }
    }

    impl ColorSpace for Srgb {
        type Color = Color;
    }

    impl InternalColor<Srgb> for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [
                self.0[0] as f32 / 255.0,
                self.0[1] as f32 / 255.0,
                self.0[2] as f32 / 255.0,
            ]
        }
    }

    impl Object for Srgb {
        fn serialize_into(self, sc: &mut SerializerContext) -> (Ref, Chunk) {
            let root_ref = sc.new_ref();
            let icc_ref = sc.new_ref();

            let mut chunk = Chunk::new();

            let mut array = chunk.indirect(root_ref).array();
            array.item(Name(b"ICCBased"));
            array.item(icc_ref);
            array.finish();

            let (stream, filter) = sc.get_binary_stream(SRGB_ICC);

            chunk
                .icc_profile(icc_ref, &stream)
                .n(3)
                .range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
                .filter(filter);

            (root_ref, chunk)
        }
    }
}

pub mod device_gray {
    use crate::object::color_space::{ColorSpace, InternalColor};
    use crate::resource::ColorSpaceEnum;
    use crate::serialize::{Object, SerializerContext};
    use pdf_writer::{Chunk, Ref};

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct DeviceGray;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color(u8);

    impl Default for Color {
        fn default() -> Self {
            DeviceGray::black()
        }
    }

    impl DeviceGray {
        pub fn new_gray(lightness: u8) -> Color {
            Color(lightness)
        }

        pub fn black() -> Color {
            Self::new_gray(0)
        }

        pub fn white() -> Color {
            Self::new_gray(255)
        }
    }

    impl Into<ColorSpaceEnum> for DeviceGray {
        fn into(self) -> ColorSpaceEnum {
            ColorSpaceEnum::DeviceGray(self)
        }
    }

    impl Into<super::Color> for Color {
        fn into(self) -> crate::object::color_space::Color {
            super::Color::DeviceGray(self)
        }
    }

    impl ColorSpace for DeviceGray {
        type Color = Color;
    }

    impl InternalColor<DeviceGray> for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [self.0 as f32 / 255.0]
        }
    }

    impl Object for DeviceGray {
        fn serialize_into(self, _: &mut SerializerContext) -> (Ref, Chunk) {
            unreachable!()
        }
    }
}

pub mod sgray {

    use crate::object::color_space::{ColorSpace, InternalColor};
    use crate::resource::ColorSpaceEnum;
    use crate::serialize::{Object, SerializerContext};
    use pdf_writer::{Chunk, Finish, Name, Ref};

    pub static GREY_ICC: &[u8] = include_bytes!("../icc/sGrey-v4.icc");

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct SGray;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color(u8);

    impl Default for Color {
        fn default() -> Self {
            SGray::black()
        }
    }

    impl SGray {
        pub fn new_gray(lightness: u8) -> Color {
            Color(lightness)
        }

        pub fn black() -> Color {
            Self::new_gray(0)
        }

        pub fn white() -> Color {
            Self::new_gray(255)
        }
    }

    impl Into<ColorSpaceEnum> for SGray {
        fn into(self) -> ColorSpaceEnum {
            ColorSpaceEnum::SGray(self)
        }
    }

    impl Into<super::Color> for Color {
        fn into(self) -> crate::object::color_space::Color {
            super::Color::SGray(self)
        }
    }

    impl ColorSpace for SGray {
        type Color = Color;
    }

    impl InternalColor<SGray> for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [self.0 as f32 / 255.0]
        }
    }

    impl Object for SGray {
        fn serialize_into(self, sc: &mut SerializerContext) -> (Ref, Chunk) {
            let root_ref = sc.new_ref();
            let icc_ref = sc.new_ref();
            let mut chunk = Chunk::new();

            let mut array = chunk.indirect(root_ref).array();
            array.item(Name(b"ICCBased"));
            array.item(icc_ref);
            array.finish();

            let (stream, filter) = sc.get_binary_stream(GREY_ICC);

            chunk
                .icc_profile(icc_ref, &stream)
                .n(1)
                .range([0.0, 1.0])
                .filter(filter);

            (root_ref, chunk)
        }
    }
}
