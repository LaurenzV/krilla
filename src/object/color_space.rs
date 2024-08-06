use crate::resource::ColorSpaceEnum;
use crate::serialize::{Object, RegisterableObject};
use pdf_writer::Finish;
use std::fmt::Debug;
use std::hash::Hash;

pub trait InternalColor<C>
where
    C: Clone + Copy + ColorSpace,
{
    fn to_pdf_color(&self) -> impl IntoIterator<Item = f32>;
    fn color_space(&self) -> C;
}

pub trait ColorSpace: Object + Debug + Hash + Eq + PartialEq + Clone + Copy {
    type Color: InternalColor<Self>;
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum Color {
    Srgb(srgb::Color),
    D65Gray(d65_gray::Color),
    DeviceGray(device_gray::Color),
    DeviceRgb(device_rgb::Color),
    DeviceCmyk(device_cmyk::Color),
}

impl Color {
    pub fn to_pdf_color(&self) -> Vec<f32> {
        match self {
            Color::Srgb(srgb) => srgb.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::D65Gray(d65_gray) => d65_gray.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::DeviceGray(dg) => dg.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::DeviceRgb(dr) => dr.to_pdf_color().into_iter().collect::<Vec<_>>(),
            Color::DeviceCmyk(dc) => dc.to_pdf_color().into_iter().collect::<Vec<_>>(),
        }
    }

    pub fn color_space(&self) -> ColorSpaceEnum {
        match self {
            Color::Srgb(srgb) => ColorSpaceEnum::Srgb(srgb.color_space()),
            Color::D65Gray(d65_gray) => ColorSpaceEnum::D65Gray(d65_gray.color_space()),
            Color::DeviceGray(d) => ColorSpaceEnum::DeviceGray(d.color_space()),
            Color::DeviceRgb(d) => ColorSpaceEnum::DeviceRgb(d.color_space()),
            Color::DeviceCmyk(d) => ColorSpaceEnum::DeviceCmyk(d.color_space()),
        }
    }
}

pub mod device_cmyk {
    use crate::object::color_space::{ColorSpace, InternalColor};
    use crate::serialize::{Object, SerializerContext};
    use pdf_writer::Ref;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct DeviceCmyk;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color([u8; 4], DeviceCmyk);

    impl DeviceCmyk {
        pub fn new_cmyk(cyan: u8, magenta: u8, yellow: u8, black: u8) -> super::Color {
            super::Color::DeviceCmyk(Color([cyan, magenta, yellow, black], DeviceCmyk))
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

        fn color_space(&self) -> DeviceCmyk {
            self.1
        }
    }

    impl Object for DeviceCmyk {
        fn serialize_into(self, _: &mut SerializerContext, _: Ref) {
            unreachable!()
        }
    }
}

pub mod device_rgb {
    use crate::object::color_space::{ColorSpace, InternalColor};
    use crate::serialize::{Object, SerializerContext};
    use pdf_writer::Ref;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct DeviceRgb;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color([u8; 3], DeviceRgb);

    impl DeviceRgb {
        pub fn new_rgb(red: u8, green: u8, blue: u8) -> super::Color {
            super::Color::DeviceRgb(Color([red, green, blue], DeviceRgb))
        }

        pub fn black() -> super::Color {
            Self::new_rgb(0, 0, 0)
        }

        pub fn white() -> super::Color {
            Self::new_rgb(255, 255, 255)
        }
    }

    impl ColorSpace for DeviceRgb {
        type Color = Color;
    }

    impl InternalColor<DeviceRgb> for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [
                self.0[0] as f32 / 255.0,
                self.0[1] as f32 / 255.0,
                self.0[2] as f32 / 255.0,
            ]
        }

        fn color_space(&self) -> DeviceRgb {
            self.1
        }
    }

    impl Object for DeviceRgb {
        fn serialize_into(self, _: &mut SerializerContext, _: Ref) {
            unreachable!()
        }
    }
}

pub mod srgb {
    use crate::object::color_space::{ColorSpace, InternalColor};
    use crate::serialize::{Object, SerializerContext};
    use crate::util::deflate;
    use once_cell::sync::Lazy;
    use pdf_writer::{Finish, Name, Ref};

    pub static SRGB_ICC_DEFLATED: Lazy<Vec<u8>> =
        Lazy::new(|| deflate(include_bytes!("../icc/sRGB-v4.icc")));

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Srgb;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color([u8; 3], Srgb);

    impl Srgb {
        pub fn new_rgb(red: u8, green: u8, blue: u8) -> super::Color {
            super::Color::Srgb(Color([red, green, blue], Srgb))
        }

        pub fn black() -> super::Color {
            Self::new_rgb(0, 0, 0)
        }

        pub fn white() -> super::Color {
            Self::new_rgb(255, 255, 255)
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

        fn color_space(&self) -> Srgb {
            self.1
        }
    }

    impl Object for Srgb {
        fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
            let icc_ref = sc.new_ref();
            let mut array = sc.chunk_mut().indirect(root_ref).array();
            array.item(Name(b"ICCBased"));
            array.item(icc_ref);
            array.finish();

            sc.chunk_mut()
                .icc_profile(icc_ref, &SRGB_ICC_DEFLATED)
                .n(3)
                .range([0.0, 1.0, 0.0, 1.0, 0.0, 1.0])
                .filter(pdf_writer::Filter::FlateDecode);
        }
    }
}

pub mod device_gray {
    use crate::object::color_space::{ColorSpace, InternalColor};
    use crate::serialize::{Object, SerializerContext};
    use pdf_writer::Ref;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct DeviceGray;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color(u8, DeviceGray);

    impl DeviceGray {
        pub fn new_gray(lightness: u8) -> super::Color {
            super::Color::DeviceGray(Color(lightness, DeviceGray))
        }

        pub fn black() -> super::Color {
            Self::new_gray(0)
        }

        pub fn white() -> super::Color {
            Self::new_gray(255)
        }
    }

    impl ColorSpace for DeviceGray {
        type Color = Color;
    }

    impl InternalColor<DeviceGray> for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [self.0 as f32 / 255.0]
        }

        fn color_space(&self) -> DeviceGray {
            self.1
        }
    }

    impl Object for DeviceGray {
        fn serialize_into(self, _: &mut SerializerContext, _: Ref) {
            unreachable!()
        }
    }
}

pub mod d65_gray {
    use crate::object::color_space::{ColorSpace, InternalColor};
    use crate::serialize::{Object, SerializerContext};
    use crate::util::deflate;
    use once_cell::sync::Lazy;
    use pdf_writer::{Finish, Name, Ref};

    pub static GREY_ICC_DEFLATED: Lazy<Vec<u8>> =
        Lazy::new(|| deflate(include_bytes!("../icc/sGrey-v4.icc")));

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct D65Gray;

    #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
    pub struct Color(u8, D65Gray);

    impl D65Gray {
        pub fn new_gray(lightness: u8) -> super::Color {
            super::Color::D65Gray(Color(lightness, D65Gray))
        }

        pub fn black() -> super::Color {
            Self::new_gray(0)
        }

        pub fn white() -> super::Color {
            Self::new_gray(255)
        }
    }

    impl ColorSpace for D65Gray {
        type Color = Color;
    }

    impl InternalColor<D65Gray> for Color {
        fn to_pdf_color(&self) -> impl IntoIterator<Item = f32> {
            [self.0 as f32 / 255.0]
        }

        fn color_space(&self) -> D65Gray {
            self.1
        }
    }

    impl Object for D65Gray {
        fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
            let icc_ref = sc.new_ref();
            let mut array = sc.chunk_mut().indirect(root_ref).array();
            array.item(Name(b"ICCBased"));
            array.item(icc_ref);
            array.finish();

            sc.chunk_mut()
                .icc_profile(icc_ref, &GREY_ICC_DEFLATED)
                .n(1)
                .range([0.0, 1.0])
                .filter(pdf_writer::Filter::FlateDecode);
        }
    }
}

// #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
// pub struct D65Gray;
//

// impl ColorSpace for D65Gray {}
//
// #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
// pub struct DeviceGray;
//
// impl Object for DeviceGray {
//     fn serialize_into(self, _: &mut SerializerContext, _: Ref) {
//         unreachable!()
//     }
// }
//
// impl ColorSpace for DeviceGray {}
//
// #[derive(Debug, Hash, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
// pub struct DeviceRgb;
//
// impl DeviceRgb {
//     pub fn new_rgb(red: u8, green: u8, blue: u8) -> TestColor {
//         TestColor::DeviceRgb(InternalColor([FiniteF32::new(red as f32 / 255.0).unwrap(), FiniteF32::new(green as f32 / 255.0).unwrap(), FiniteF32::new(blue as f32 / 255.0).unwrap()], DeviceRgb))
//     }
//
//     pub fn black() -> TestColor {
//         Self::new_rgb(0, 0, 0)
//     }
//
//     pub fn white() -> TestColor {
//         Self::new_rgb(255, 255, 255)
//     }
// }
//
// impl Object for DeviceRgb {
//     fn serialize_into(self, _: &mut SerializerContext, _: Ref) {
//         unreachable!()
//     }
// }
//
// impl ColorSpace for DeviceRgb {}
