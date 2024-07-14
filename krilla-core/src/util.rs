use pdf_writer::Name;

pub fn deflate(data: &[u8]) -> Vec<u8> {
    const COMPRESSION_LEVEL: u8 = 6;
    miniz_oxide::deflate::compress_to_vec_zlib(data, COMPRESSION_LEVEL)
}

pub trait NameExt {
    fn to_pdf_name(&self) -> Name;
}

impl NameExt for String {
    fn to_pdf_name(&self) -> Name {
        Name(self.as_bytes())
    }
}
