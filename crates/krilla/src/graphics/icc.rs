//! Using ICC profiles.

use md5::{Digest, Md5};
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use pdf_writer::{Finish, Name, Ref};

use crate::chunk_container::ChunkContainer;
use crate::resource;
use crate::resource::Resourceable;
use crate::serialize::{Cacheable, SerializeContext};
use crate::stream::{deflate_encode, FilterStreamBuilder};
use crate::util::Prehashed;

/// An ICC profile.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ICCProfile<const C: u8>(Arc<Prehashed<Repr>>);

impl<const C: u8> ICCProfile<C> {
    /// Create a new ICC profile.
    ///
    /// Returns `None` if the metadata of the profile couldn't be read or if the
    /// number of channels of the underlying data does not correspond to the one
    /// indicated in the constant parameter.
    pub fn new(data: &[u8]) -> Option<Self> {
        let metadata = ICCMetadata::from_data(data)?;

        if metadata.color_space.num_components() != C {
            return None;
        }

        Some(Self(Arc::new(Prehashed::new(Repr {
            data: deflate_encode(data),
            metadata,
        }))))
    }

    pub(crate) fn metadata(&self) -> &ICCMetadata {
        &self.0.metadata
    }
}

impl<const C: u8> Cacheable for ICCProfile<C> {
    fn serialize(
        self,
        sc: &mut SerializeContext,
        chunk_container: &mut ChunkContainer,
        root_ref: Ref,
    ) {
        let mut chunk = sc.new_chunk();
        let icc_stream = FilterStreamBuilder::new_from_deflated(&self.0.deref().data)
            .finish(&sc.serialize_settings());

        let mut icc_profile = chunk.icc_profile(root_ref, icc_stream.encoded_data());
        icc_profile.n(C as i32).range([0.0, 1.0].repeat(C as usize));
        icc_stream.write_filters(icc_profile.deref_mut().deref_mut());
        icc_profile.finish();
        chunk_container.streams.icc_profiles.push(chunk);
    }
}

/// An ICC profile of one of the supported channel counts.
///
/// Used both for embedded raster image profiles and for the externally
/// referenced output profile of PDF/X-4p and PDF/X-6p.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) enum GenericICCProfile {
    Luma(ICCProfile<1>),
    Rgb(ICCProfile<3>),
    Cmyk(ICCProfile<4>),
}

impl GenericICCProfile {
    pub(crate) fn metadata(&self) -> &ICCMetadata {
        match self {
            GenericICCProfile::Luma(l) => l.metadata(),
            GenericICCProfile::Rgb(r) => r.metadata(),
            GenericICCProfile::Cmyk(c) => c.metadata(),
        }
    }
}

// The `Cacheable` implementation is only needed when embedding a profile as a
// stream, which today only happens for raster image color spaces.
#[cfg(feature = "raster-images")]
impl Cacheable for GenericICCProfile {
    fn serialize(
        self,
        sc: &mut SerializeContext,
        chunk_container: &mut ChunkContainer,
        root_ref: Ref,
    ) {
        match self {
            GenericICCProfile::Luma(l) => l.serialize(sc, chunk_container, root_ref),
            GenericICCProfile::Rgb(r) => r.serialize(sc, chunk_container, root_ref),
            GenericICCProfile::Cmyk(c) => c.serialize(sc, chunk_container, root_ref),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub(crate) struct ICCBasedColorSpace<const C: u8>(pub(crate) ICCProfile<C>);

impl<const C: u8> Cacheable for ICCBasedColorSpace<C> {
    fn serialize(
        self,
        sc: &mut SerializeContext,
        chunk_container: &mut ChunkContainer,
        root_ref: Ref,
    ) {
        let icc_ref = sc.register_cacheable(chunk_container, self.0.clone());

        let chunk = &mut chunk_container.non_stream.color_spaces;

        let mut array = chunk.indirect(root_ref).array();
        array.item(Name(b"ICCBased"));
        array.item(icc_ref);
        array.finish();
    }
}

impl<const C: u8> Resourceable for ICCBasedColorSpace<C> {
    type Resource = resource::ColorSpace;
}

#[derive(Clone, Hash, Debug, Eq, PartialEq)]
pub(crate) enum ICCColorSpace {
    Xyz,
    Lab,
    Luv,
    Ycbr,
    Yxy,
    Lms,
    Rgb,
    Gray,
    Hsv,
    Hls,
    Cmyk,
    Cmy,
    OneClr,
    ThreeClr,
    FourClr,
    // There are more, but those should be the most important
    // ones.
}

impl ICCColorSpace {
    pub(crate) fn num_components(&self) -> u8 {
        match self {
            ICCColorSpace::Xyz => 3,
            ICCColorSpace::Lab => 3,
            ICCColorSpace::Luv => 3,
            ICCColorSpace::Ycbr => 3,
            ICCColorSpace::Yxy => 3,
            ICCColorSpace::Lms => 3,
            ICCColorSpace::Rgb => 3,
            ICCColorSpace::Gray => 1,
            ICCColorSpace::Hsv => 3,
            ICCColorSpace::Hls => 3,
            ICCColorSpace::Cmyk => 4,
            ICCColorSpace::Cmy => 3,
            ICCColorSpace::OneClr => 1,
            ICCColorSpace::ThreeClr => 3,
            ICCColorSpace::FourClr => 4,
        }
    }
}

impl TryFrom<u32> for ICCColorSpace {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x58595A20 => Ok(ICCColorSpace::Xyz),
            0x4C616220 => Ok(ICCColorSpace::Lab),
            0x4C757620 => Ok(ICCColorSpace::Luv),
            0x59436272 => Ok(ICCColorSpace::Ycbr),
            0x59787920 => Ok(ICCColorSpace::Yxy),
            0x4C4D5320 => Ok(ICCColorSpace::Lms),
            0x52474220 => Ok(ICCColorSpace::Rgb),
            0x47524159 => Ok(ICCColorSpace::Gray),
            0x48535620 => Ok(ICCColorSpace::Hsv),
            0x484C5320 => Ok(ICCColorSpace::Hls),
            0x434D594B => Ok(ICCColorSpace::Cmyk),
            0x434D5920 => Ok(ICCColorSpace::Cmy),
            0x31434C52 => Ok(ICCColorSpace::OneClr),
            0x33434C52 => Ok(ICCColorSpace::ThreeClr),
            0x34434C52 => Ok(ICCColorSpace::FourClr),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Hash, Debug, Eq, PartialEq)]
pub(crate) struct ICCMetadata {
    pub(crate) major: u8,
    pub(crate) minor: u8,
    pub(crate) version_bytes: [u8; 4],
    pub(crate) color_space: ICCColorSpace,
    pub(crate) color_space_signature: [u8; 4],
    /// The ICC profile/device class signature (header bytes 12..16), e.g.
    /// `prtr` (output), `mntr` (display), `scnr` (input), `link`, `spac`.
    pub(crate) device_class: [u8; 4],
    pub(crate) checksum: [u8; 16],
    pub(crate) profile_name: Option<String>,
}

impl ICCMetadata {
    pub(crate) fn from_data(data: &[u8]) -> Option<Self> {
        let version_bytes: [u8; 4] = data.get(8..12)?.try_into().ok()?;
        let major = version_bytes[0];
        let minor = version_bytes[1] >> 4;
        let device_class: [u8; 4] = data.get(12..16)?.try_into().ok()?;
        let color_space_signature: [u8; 4] = data.get(16..20)?.try_into().ok()?;
        let color_space =
            ICCColorSpace::try_from(u32::from_be_bytes(color_space_signature)).ok()?;
        let checksum = Md5::digest(data).into();
        let profile_name = parse_profile_name(data);
        Some(Self {
            major,
            minor,
            version_bytes,
            color_space,
            color_space_signature,
            device_class,
            checksum,
            profile_name,
        })
    }

    /// Whether this profile is an output device profile (ICC Device Class
    /// `prtr`), as a PDF/X output-intent profile must be (ISO 15930-7 §6.4.2.1:
    /// "an Output Device Profile (Device Class = 'prtr')"). Display (`mntr`),
    /// input (`scnr`) and transform profiles (`link`, `spac`, `abst`, `nmcl`)
    /// are not valid PDF/X output intents — this is stricter than PDF/A, which
    /// also admits a `mntr` display profile.
    pub(crate) fn is_output_rendering_device(&self) -> bool {
        &self.device_class == b"prtr"
    }
}

fn parse_profile_name(data: &[u8]) -> Option<String> {
    let tag_count =
        usize::try_from(u32::from_be_bytes(data.get(128..132)?.try_into().ok()?)).ok()?;

    for index in 0..tag_count {
        let tag_offset = 132usize.checked_add(index.checked_mul(12)?)?;
        let record = data.get(tag_offset..tag_offset.checked_add(12)?)?;

        if record.get(..4)? != b"desc" {
            continue;
        }

        let data_offset =
            usize::try_from(u32::from_be_bytes(record.get(4..8)?.try_into().ok()?)).ok()?;
        let data_len =
            usize::try_from(u32::from_be_bytes(record.get(8..12)?.try_into().ok()?)).ok()?;
        let tag = data.get(data_offset..data_offset.checked_add(data_len)?)?;

        return parse_profile_description(tag);
    }

    None
}

fn parse_profile_description(data: &[u8]) -> Option<String> {
    match data.get(..4)? {
        b"desc" => parse_desc_profile_description(data),
        b"mluc" => parse_mluc_profile_description(data),
        _ => None,
    }
}

fn parse_desc_profile_description(data: &[u8]) -> Option<String> {
    let ascii_len = usize::try_from(u32::from_be_bytes(data.get(8..12)?.try_into().ok()?)).ok()?;
    let string_end = 12usize.checked_add(ascii_len.checked_sub(1)?)?;
    let bytes = data.get(12..string_end)?;
    let name = String::from_utf8(bytes.to_vec()).ok()?;
    (!name.is_empty()).then_some(name)
}

fn parse_mluc_profile_description(data: &[u8]) -> Option<String> {
    let record_count =
        usize::try_from(u32::from_be_bytes(data.get(8..12)?.try_into().ok()?)).ok()?;
    let record_size =
        usize::try_from(u32::from_be_bytes(data.get(12..16)?.try_into().ok()?)).ok()?;

    if record_size < 12 {
        return None;
    }

    for index in 0..record_count {
        let record_offset = 16usize.checked_add(index.checked_mul(record_size)?)?;
        let record = data.get(record_offset..record_offset.checked_add(record_size)?)?;
        let length =
            usize::try_from(u32::from_be_bytes(record.get(4..8)?.try_into().ok()?)).ok()?;
        let offset =
            usize::try_from(u32::from_be_bytes(record.get(8..12)?.try_into().ok()?)).ok()?;
        let bytes = data.get(offset..offset.checked_add(length)?)?;

        if bytes.len() % 2 != 0 {
            continue;
        }

        let utf16 = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let name = String::from_utf16(&utf16).ok()?;

        if !name.is_empty() {
            return Some(name);
        }
    }

    None
}

#[derive(Clone, Hash, Debug)]
struct Repr {
    data: Vec<u8>,
    metadata: ICCMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal ICC profile header with the given color-space signature,
    /// a tag table of the given size, and a raw-bytes tag payload appended
    /// after the tag table.
    fn build_profile(
        color_space: [u8; 4],
        tag_count: u32,
        tag_records: &[u8],
        tag_payload: &[u8],
    ) -> Vec<u8> {
        let mut data = vec![0u8; 128];
        data[8..12].copy_from_slice(&[0x04, 0x20, 0x00, 0x00]);
        data[16..20].copy_from_slice(&color_space);
        data.extend_from_slice(&tag_count.to_be_bytes());
        data.extend_from_slice(tag_records);
        data.extend_from_slice(tag_payload);
        data
    }

    #[test]
    fn icc_tag_table_out_of_range_is_rejected_without_panic() {
        // Header valid, tag count huge: the parser must not panic when walking
        // past the end of the buffer while indexing tag records.
        let data = build_profile(*b"GRAY", u32::MAX / 12, &[], &[]);
        let metadata = ICCMetadata::from_data(&data).expect("header still parses");
        assert!(metadata.profile_name.is_none());
    }

    #[test]
    fn icc_desc_ascii_len_oversized_is_rejected_without_panic() {
        // A 'desc' tag whose ascii length is larger than the available data
        // must not panic while computing string_end.
        let tag_records = tag_record(b"desc", 144, 20);
        let mut tag_payload = Vec::new();
        tag_payload.extend_from_slice(b"desc");
        tag_payload.extend_from_slice(&[0u8; 4]);
        tag_payload.extend_from_slice(&u32::MAX.to_be_bytes());
        let data = build_profile(*b"GRAY", 1, &tag_records, &tag_payload);
        let metadata = ICCMetadata::from_data(&data).expect("header still parses");
        assert!(metadata.profile_name.is_none());
    }

    #[test]
    fn icc_mluc_record_offset_overflow_is_rejected_without_panic() {
        // An 'mluc' desc tag with an adversarial record_size must not panic
        // while computing record_offset + record_size.
        let tag_records = tag_record(b"desc", 144, 16);
        let mut tag_payload = Vec::new();
        tag_payload.extend_from_slice(b"mluc");
        tag_payload.extend_from_slice(&[0u8; 4]);
        tag_payload.extend_from_slice(&1u32.to_be_bytes());
        tag_payload.extend_from_slice(&u32::MAX.to_be_bytes());
        let data = build_profile(*b"GRAY", 1, &tag_records, &tag_payload);
        let metadata = ICCMetadata::from_data(&data).expect("header still parses");
        assert!(metadata.profile_name.is_none());
    }

    fn tag_record(signature: &[u8; 4], offset: u32, size: u32) -> Vec<u8> {
        let mut record = Vec::with_capacity(12);
        record.extend_from_slice(signature);
        record.extend_from_slice(&offset.to_be_bytes());
        record.extend_from_slice(&size.to_be_bytes());
        record
    }
}
