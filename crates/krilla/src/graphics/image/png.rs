use std::borrow::Cow;

use png::chunk;

pub(super) struct PngData<'a> {
    pub(super) idat: Cow<'a, [u8]>,
}

impl<'a> PngData<'a> {
    pub(super) fn new(data: &'a [u8], info: &png::Info) -> Option<Self> {
        if !is_supported(info) {
            return None;
        }

        let mut idat = None;
        let mut reached_iend = false;

        for png_chunk in Chunks::new(data).ok()? {
            let png_chunk = png_chunk.ok()?;

            if png_chunk.is_critical && !png_chunk.has_valid_crc() {
                return None;
            }

            match png_chunk.kind {
                chunk::IHDR => {}
                chunk::PLTE => return None,
                chunk::IDAT => append_idat(&mut idat, png_chunk.data),
                chunk::IEND => {
                    reached_iend = true;
                    break;
                }
                _ if png_chunk.is_critical => return None,
                _ => {}
            }
        }

        reached_iend.then_some(Self { idat: idat? })
    }
}

fn is_supported(info: &png::Info) -> bool {
    use png::ColorType::*;
    // Indexed can be supported in the future.
    let kind_supported = matches!(info.color_type, Grayscale | Rgb);

    if !kind_supported
        || info.interlaced
        // Those can also be supported in the future.
        || info.bit_depth != png::BitDepth::Eight
        || info.trns.is_some()
    {
        return false;
    }

    true
}

fn append_idat<'a>(idat: &mut Option<Cow<'a, [u8]>>, data: &'a [u8]) {
    *idat = Some(match idat.take() {
        None => Cow::Borrowed(data),
        Some(Cow::Borrowed(previous)) => {
            let mut combined = Vec::with_capacity(previous.len() + data.len());
            combined.extend_from_slice(previous);
            combined.extend_from_slice(data);
            Cow::Owned(combined)
        }
        Some(Cow::Owned(mut combined)) => {
            combined.extend_from_slice(data);
            Cow::Owned(combined)
        }
    });
}

const PNG_MAGIC: &[u8] = b"\x89\x50\x4E\x47\x0D\x0A\x1A\x0A";

struct PngChunk<'a> {
    kind: chunk::ChunkType,
    data: &'a [u8],
    crc: u32,
    is_critical: bool,
}

impl PngChunk<'_> {
    fn has_valid_crc(&self) -> bool {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.kind.0);
        hasher.update(self.data);
        hasher.finalize() == self.crc
    }
}

struct Chunks<'a> {
    reader: Reader<'a>,
    failed: bool,
}

impl<'a> Chunks<'a> {
    fn new(data: &'a [u8]) -> Result<Self, &'static str> {
        let mut reader = Reader { data };

        if reader.read(PNG_MAGIC.len()) != Some(PNG_MAGIC) {
            return Err("invalid PNG signature");
        }

        Ok(Self {
            reader,
            failed: false,
        })
    }

    fn read_chunk(&mut self) -> Result<PngChunk<'a>, &'static str> {
        let chunk_len = self.reader.read_u32().ok_or("chunk is too short")?;
        let kind = self.reader.read_u32().ok_or("chunk is too short")?;
        let data = self
            .reader
            .read(chunk_len as usize)
            .ok_or("chunk is too short")?;
        let crc = self.reader.read_u32().ok_or("chunk is too short")?;

        let kind = chunk::ChunkType(kind.to_be_bytes());

        Ok(PngChunk {
            kind,
            data,
            crc,
            is_critical: chunk::is_critical(kind),
        })
    }
}

impl<'a> Iterator for Chunks<'a> {
    type Item = Result<PngChunk<'a>, &'static str>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.reader.data.is_empty() {
            return None;
        }

        let chunk = self.read_chunk();
        self.failed = chunk.is_err();
        Some(chunk)
    }
}

struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    fn read(&mut self, len: usize) -> Option<&'a [u8]> {
        let (bytes, rest) = self.data.split_at_checked(len)?;
        self.data = rest;
        Some(bytes)
    }

    fn read_u32(&mut self) -> Option<u32> {
        let bytes = self.read(size_of::<u32>())?;
        Some(u32::from_be_bytes(bytes.try_into().unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::append_idat;
    use std::borrow::Cow;

    #[test]
    fn single_idat_is_borrowed() {
        let data = [1, 2, 3];
        let mut idat = None;
        append_idat(&mut idat, &data);

        assert!(matches!(idat, Some(Cow::Borrowed(_))));
    }

    #[test]
    fn multiple_idats_are_concatenated() {
        let mut idat = None;
        append_idat(&mut idat, &[1, 2]);
        append_idat(&mut idat, &[3, 4]);

        assert_eq!(idat, Some(Cow::Owned(vec![1, 2, 3, 4])));
    }
}
