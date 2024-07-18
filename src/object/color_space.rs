use crate::resource::PDFResource;
use crate::serialize::{Object, SerializerContext};
use crate::util::deflate;
use once_cell::sync::Lazy;
use pdf_writer::{Finish, Name, Ref};

// The ICC profiles.
pub static SRGB_ICC_DEFLATED: Lazy<Vec<u8>> =
    Lazy::new(|| deflate(include_bytes!("../icc/sRGB-v4.icc")));
pub static GREY_ICC_DEFLATED: Lazy<Vec<u8>> =
    Lazy::new(|| deflate(include_bytes!("../icc/sGrey-v4.icc")));

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum ColorSpace {
    SRGB,
    D65Gray,
}

impl PDFResource for ColorSpace {
    fn get_name() -> &'static str {
        "C"
    }
}

impl Object for ColorSpace {
    const CACHED: bool = true;

    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        match self {
            ColorSpace::SRGB => {
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
            ColorSpace::D65Gray => {
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
}
