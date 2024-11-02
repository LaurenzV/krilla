use pdf_writer::Pdf;
use xmp_writer::XmpWriter;
use crate::color::{ICCMetadata, ICCProfile};
use crate::resource::{GREY_V2_ICC, GREY_V4_ICC, SRGB_V2_ICC, SRGB_V4_ICC};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PdfVersion {
    Pdf14,
    Pdf15,
    Pdf16,
    Pdf17,
}

impl PdfVersion {
    pub(crate) fn write_xmp(&self, xmp: &mut XmpWriter) {
        match self {
            PdfVersion::Pdf14 => xmp.pdf_version("1.4"),
            PdfVersion::Pdf15 => xmp.pdf_version("1.5"),
            PdfVersion::Pdf16 => xmp.pdf_version("1.6"),
            PdfVersion::Pdf17 => xmp.pdf_version("1.7"),
        };
    }

    pub fn as_str(&self) -> &str {
        match self {
            PdfVersion::Pdf14 => "PDF-1.4",
            PdfVersion::Pdf15 => "PDF-1.5",
            PdfVersion::Pdf16 => "PDF-1.6",
            PdfVersion::Pdf17 => "PDF-1.7",
        }
    }

    pub(crate) fn rgb_icc(&self) -> ICCProfile<3> {
        match self {
            PdfVersion::Pdf14 => SRGB_V2_ICC.clone(),
            PdfVersion::Pdf15 => SRGB_V2_ICC.clone(),
            PdfVersion::Pdf16 => SRGB_V2_ICC.clone(),
            PdfVersion::Pdf17 => SRGB_V4_ICC.clone()
        }
    }

    pub(crate) fn grey_icc(&self) -> ICCProfile<1> {
        match self {
            PdfVersion::Pdf14 => GREY_V2_ICC.clone(),
            PdfVersion::Pdf15 => GREY_V2_ICC.clone(),
            PdfVersion::Pdf16 => GREY_V2_ICC.clone(),
            PdfVersion::Pdf17 => GREY_V4_ICC.clone(),
        }
    }

    pub(crate) fn supports_icc(&self, metadata: &ICCMetadata) -> bool {
        match self {
            PdfVersion::Pdf14 => metadata.major <= 2 && metadata.minor <= 2,
            PdfVersion::Pdf15 => metadata.major <= 4,
            PdfVersion::Pdf16 => metadata.major <= 4 && metadata.minor <= 1,
            PdfVersion::Pdf17 => metadata.major <= 4 && metadata.minor <= 2,
        }
    }

    pub(crate) fn set_version(&self, pdf: &mut Pdf) {
        match self {
            PdfVersion::Pdf14 => pdf.set_version(1, 4),
            PdfVersion::Pdf15 => pdf.set_version(1, 5),
            PdfVersion::Pdf16 => pdf.set_version(1, 6),
            PdfVersion::Pdf17 => pdf.set_version(1, 7),
        };
    }
}
