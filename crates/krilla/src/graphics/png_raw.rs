use png::chunk;

pub fn is_supported_in_pdf(info: &png::Info) -> bool {
    if info.interlaced {
        return false;
    }

    if i32::try_from(info.width).is_err() {
        return false; // width needs to be stored as a PDF integer
    }

    use png::ColorType::*;
    match info.color_type {
        Grayscale | Rgb => true,
        GrayscaleAlpha | Rgba => false,
        Indexed => false, // Can be embedded in PDF but not implemented yet
    }
}

const PNG_MAGIC: &[u8] = b"\x89\x50\x4E\x47\x0D\x0A\x1A\x0A";

pub fn extract_idat(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut reader = Reader { data };

    let magic = reader.read(PNG_MAGIC.len());
    if magic != Some(PNG_MAGIC) {
        return Err("invalid PNG signature");
    }

    // Estimate minimum starting capacity; very unscientific
    let mut idat = Vec::with_capacity((data.len() / 2).next_multiple_of(8));

    // [Chunk Len] [Chunk Type] [Chunk Payload...] [CRC]
    while let Some(chunk_len) = reader.read_u32() {
        let chunk_type = match reader.read_u32() {
            Some(n) => chunk::ChunkType(u32::to_be_bytes(n)),
            None => break,
        };

        let Some(payload) = reader.read(chunk_len as usize) else {
            return Err("chunk is too short");
        };

        let Some(crc) = reader.read_u32() else {
            return Err("chunk is too short");
        };

        if chunk::is_critical(chunk_type) {
            let mut hasher = crc32fast::Hasher::new();
            hasher.update(&chunk_type.0);
            hasher.update(payload);
            if hasher.finalize() != crc {
                return Err("CRC checksum mismatch");
            }
        }

        match chunk_type {
            chunk::IHDR => (),
            chunk::PLTE => return Err("indexed color is not supported"),
            chunk::IDAT => idat.extend_from_slice(payload),
            chunk::IEND => break,
            ty if chunk::is_critical(ty) => {
                return Err("unrecognized critical chunk type")
            }
            _ => (),
        }
    }

    Ok(idat)
}

struct Reader<'a> {
    data: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn read(&mut self, len: usize) -> Option<&'a [u8]> {
        let (bytes, rest) = self.data.split_at_checked(len)?;
        self.data = rest;
        Some(bytes)
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        let bytes = self.read(size_of::<u32>())?;
        Some(u32::from_be_bytes(bytes.try_into().unwrap()))
    }
}
