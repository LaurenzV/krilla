use pdf_writer::Pdf;
use xmp_writer::XmpWriter;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PdfVersion {
    Pdf14,
    Pdf15,
    Pdf16,
    Pdf17,
}

impl PdfVersion {
    pub fn write_xmp(&self, xmp: &mut XmpWriter) {
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

    pub fn set_version(&self, pdf: &mut Pdf) {
        match self {
            PdfVersion::Pdf14 => pdf.set_version(1, 4),
            PdfVersion::Pdf15 => pdf.set_version(1, 5),
            PdfVersion::Pdf16 => pdf.set_version(1, 6),
            PdfVersion::Pdf17 => pdf.set_version(1, 7),
        };
    }
}
