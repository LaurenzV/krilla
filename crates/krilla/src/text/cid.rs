//! CID fonts.

use std::collections::BTreeMap;
use std::hash::Hash;
use std::ops::DerefMut;

use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::writers::WMode;
use pdf_writer::{Chunk, Finish, Name, Ref, Str};
use skrifa::raw::tables::cff::Cff;
use skrifa::raw::{TableProvider, TopLevelTable};
use subsetter::GlyphRemapper;

use super::{CIDIdentifier, FontIdentifier, PDF_UNITS_PER_EM};
use crate::configure::ValidationError;
use crate::error::{KrillaError, KrillaResult};
use crate::serialize::SerializeContext;
use crate::stream::FilterStreamBuilder;
use crate::surface::Location;
use crate::text::{Font, GlyphId};
use crate::util::{hash128, RectExt, SliceExt};

const SUBSET_TAG_LEN: usize = 6;
pub(crate) const IDENTITY_H: &str = "Identity-H";
pub(crate) const CMAP_NAME: Name = Name(b"Custom");
pub(crate) const SYSTEM_INFO: SystemInfo = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

pub(crate) type Cid = u16;

/// A shared macro for CID fonts and Type3 fonts to write the cmap entries.
#[macro_export]
macro_rules! cmap_inner {
    ($self:expr, $entry:expr, $sc:expr, $cmap:expr, $g:expr) => {
        match $entry {
            None => $sc.register_validation_error(ValidationError::InvalidCodepointMapping(
                $self.font.clone(),
                GlyphId::new($g as u32),
                None,
                None,
            )),
            Some((text, loc)) => {

                let mut invalid_codepoint = text.is_empty();
                let mut invalid_code = None;
                let mut private_unicode = None;

                for c in text.chars() {
                    if matches!(c as u32, 0x0 | 0xFEFF | 0xFFFE) {
                        invalid_code = Some(c);
                        invalid_codepoint = true;
                    }

                    if matches!(c as u32, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD)
                    {
                        private_unicode = Some(c);
                    }
                }

                if invalid_codepoint {
                    $sc.register_validation_error(ValidationError::InvalidCodepointMapping(
                        $self.font.clone(),
                        GlyphId::new($g as u32),
                        invalid_code,
                        *loc,
                    ))
                }

                if let Some(code) = private_unicode {
                    $sc.register_validation_error(ValidationError::UnicodePrivateArea(
                        $self.font.clone(),
                        GlyphId::new($g as u32),
                        code,
                        *loc,
                    ))
                }

                if !text.is_empty() {
                    $cmap.pair_with_multiple($g, text.chars());
                }
            }
        }
    };
}

/// A CID-keyed font.
#[derive(Debug, Clone)]
pub(crate) struct CIDFont {
    /// The _actual_ underlying OTF font of the CID-keyed font.
    font: Font,
    /// A mapper that maps GIDs from the original font to CIDs, i.e. the corresponding GID in the font
    /// subset. The subsetter will ensure that for CID-keyed CFF fonts, the CID-to-GID mapping
    /// will be the identity mapping, regardless of what the mapping was in the original font. This
    /// allows us to index both, CFF and glyf-based fonts, transparently using GIDs,
    /// instead of having to distinguish according to the underlying font. See section
    /// 9.7.4.2 for more information on how glyphs are indexed in a CID-keyed font.
    glyph_remapper: GlyphRemapper,
    /// A mapping from CIDs to their string in the original text.
    cmap_entries: BTreeMap<u16, (String, Option<Location>)>,
    /// The widths of the glyphs, _indexed by their CID_.
    widths: Vec<f32>,
}

impl CIDFont {
    /// Create a new CID-keyed font.
    pub(crate) fn new(font: Font) -> CIDFont {
        // Always include the .notdef glyph. Will also always be included by the subsetter in
        // the glyph remapper.
        let widths = vec![font.advance_width(GlyphId::new(0)).unwrap_or(0.0)];

        Self {
            glyph_remapper: GlyphRemapper::new(),
            cmap_entries: BTreeMap::new(),
            widths,
            font,
        }
    }

    pub(crate) fn font(&self) -> Font {
        self.font.clone()
    }

    // Note that this refers to the units per em in PDF (which is always 1000), and not the
    // units per em of the underlying font.
    pub(crate) fn units_per_em(&self) -> f32 {
        PDF_UNITS_PER_EM
    }

    #[inline]
    pub(crate) fn get_cid(&self, glyph_id: GlyphId) -> Option<u16> {
        self.glyph_remapper
            .get(u16::try_from(glyph_id.to_u32()).unwrap())
    }

    /// Add a new glyph (if it has not already been added) and return its CID.
    #[inline]
    pub(crate) fn add_glyph(&mut self, glyph_id: GlyphId) -> Cid {
        let new_id = self
            .glyph_remapper
            .remap(u16::try_from(glyph_id.to_u32()).unwrap());

        // This means that the glyph ID has been newly assigned, and thus we need to add its width.
        if new_id as usize >= self.widths.len() {
            self.widths
                .push(self.font.advance_width(glyph_id).unwrap_or(0.0));
        }

        new_id
    }

    #[inline]
    pub(crate) fn get_codepoints(&self, cid: Cid) -> Option<&str> {
        self.cmap_entries.get(&cid).map(|s| s.0.as_str())
    }

    #[inline]
    pub(crate) fn set_codepoints(&mut self, cid: Cid, text: String, location: Option<Location>) {
        self.cmap_entries.insert(cid, (text, location));
    }

    #[inline]
    pub(crate) fn identifier(&self) -> FontIdentifier {
        FontIdentifier::Cid(CIDIdentifier(self.font.clone()))
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializeContext,
        root_ref: Ref,
    ) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let cid_ref = sc.new_ref();
        let descriptor_ref = sc.new_ref();
        let cmap_ref = sc.new_ref();
        let cid_set_ref = sc.new_ref();
        let data_ref = sc.new_ref();

        let glyph_remapper = &self.glyph_remapper;

        let is_glyf = self.font.font_ref().glyf().is_ok();
        let is_cff = self.font.font_ref().cff().is_ok();

        if !is_glyf && !is_cff {
            let is_cff2 = self.font.font_ref().cff2().is_ok();

            return if is_cff2 {
                Err(KrillaError::Font(
                    self.font.clone(),
                    "CFF2 fonts are not supported".to_string(),
                ))
            } else {
                Err(KrillaError::Font(
                    self.font.clone(),
                    "font is missing `glyf` or `CFF` table".to_string(),
                ))
            };
        }

        let subsetted = subset_font(self.font.clone(), glyph_remapper)?;
        let num_glyphs;

        let font_stream = {
            let mut data = subsetted.as_slice();

            // If we have a CFF font, only embed the standalone CFF program.
            let subsetted_ref = skrifa::FontRef::new(data).map_err(|_| {
                KrillaError::Font(self.font.clone(), "failed to read font subset".to_string())
            })?;

            num_glyphs = subsetted_ref.maxp().map(|m| m.num_glyphs()).map_err(|_| {
                KrillaError::Font(self.font.clone(), "failed to read font subset".to_string())
            })?;

            if let Some(cff) = subsetted_ref.data_for_tag(Cff::TAG) {
                data = cff.as_bytes();
            }

            FilterStreamBuilder::new_from_binary_data(data).finish(&sc.serialize_settings())
        };

        let base_font = base_font_name(&self.font, &self.glyph_remapper);
        let base_font_type0 = if is_cff {
            format!("{base_font}-{}", IDENTITY_H)
        } else {
            base_font.clone()
        };

        chunk
            .type0_font(root_ref)
            .base_font(Name(base_font_type0.as_bytes()))
            .encoding_predefined(Name(IDENTITY_H.as_bytes()))
            .descendant_font(cid_ref)
            .to_unicode(cmap_ref);

        let mut cid = chunk.cid_font(cid_ref);
        cid.subtype(if is_cff {
            CidFontType::Type0
        } else {
            CidFontType::Type2
        });
        cid.base_font(Name(base_font.as_bytes()));
        cid.system_info(SYSTEM_INFO);
        cid.font_descriptor(descriptor_ref);
        cid.default_width(0.0);

        if !is_cff {
            cid.cid_to_gid_map_predefined(Name(b"Identity"));
        }

        // IN CID fonts, a upem value of 1000 is assumed for all fonts, so we need to convert.
        let to_pdf_units = |v: f32| v / self.font.units_per_em() * self.units_per_em();

        let mut first = 0;
        let mut width_writer = cid.widths();
        for (w, group) in self.widths.group_by_key(|&w| w) {
            let end = first + group.len();
            if w != 0.0 {
                let last = end - 1;
                width_writer.same(first as u16, last as u16, to_pdf_units(w));
            }
            first = end;
        }

        width_writer.finish();
        cid.finish();

        // The only reason we write this in the first place is that PDF/A1-b requires
        // a CIDSet.
        if !sc.serialize_settings().pdf_version().deprecates_cid_set() {
            let cid_stream_data = {
                // It's always guaranteed by the subsetter that CIDs start from 0 and are
                // consecutive, so this encoding is very straight-forward.
                let mut bytes = vec![];
                bytes.extend([0xFFu8].repeat((num_glyphs / 8) as usize));
                let padding = num_glyphs % 8;
                if padding != 0 {
                    bytes.push(!(0xFF >> padding))
                }

                bytes
            };

            let cid_stream = FilterStreamBuilder::new_from_binary_data(&cid_stream_data)
                .finish(&sc.serialize_settings());
            let mut cid_set = chunk.stream(cid_set_ref, cid_stream.encoded_data());
            cid_stream.write_filters(cid_set.deref_mut());
            cid_set.finish();
            cid_stream.finish();
        }

        let mut flags = FontFlags::empty();
        flags.set(
            FontFlags::SERIF,
            self.font
                .postscript_name()
                .is_some_and(|n| n.contains("Serif")),
        );
        flags.set(FontFlags::FIXED_PITCH, self.font.is_monospaced());
        flags.set(FontFlags::ITALIC, self.font.italic_angle() != 0.0);
        flags.insert(FontFlags::SYMBOLIC);
        flags.insert(FontFlags::SMALL_CAP);

        let bbox = self.font.bbox().to_pdf_rect();

        let italic_angle = self.font.italic_angle();
        let ascender = to_pdf_units(self.font.ascent());
        let descender = to_pdf_units(self.font.descent());
        let cap_height = self.font.cap_height().map(to_pdf_units).unwrap_or(ascender);
        let stem_v = 10.0 + 0.244 * (self.font.weight() - 50.0);

        let mut font_descriptor = chunk.font_descriptor(descriptor_ref);
        font_descriptor
            .name(Name(base_font.as_bytes()))
            .flags(flags)
            .bbox(bbox)
            .italic_angle(italic_angle)
            .ascent(ascender)
            .descent(descender)
            .cap_height(cap_height)
            .stem_v(stem_v);

        if !sc.serialize_settings().pdf_version().deprecates_cid_set() {
            font_descriptor.cid_set(cid_set_ref);
        }

        if is_cff {
            font_descriptor.font_file3(data_ref);
        } else {
            font_descriptor.font_file2(data_ref);
        }

        font_descriptor.finish();

        let cmap = {
            let mut cmap = UnicodeCmap::new(CMAP_NAME, SYSTEM_INFO);

            // For the .notdef glyph, it's fine if no mapping exists, since it is included
            // even if it was not referenced in the text.
            for g in 1..self.glyph_remapper.num_gids() {
                let entry = self.cmap_entries.get(&g);
                cmap_inner!(&self, entry, sc, &mut cmap, g);
            }

            cmap
        };

        let cmap_stream = cmap.finish();
        let mut cmap = chunk.cmap(cmap_ref, &cmap_stream);
        cmap.writing_mode(WMode::Horizontal);
        cmap.finish();

        let mut stream = chunk.stream(data_ref, font_stream.encoded_data());
        font_stream.write_filters(stream.deref_mut());
        if is_cff {
            stream.pair(Name(b"Subtype"), Name(b"CIDFontType0C"));
        }

        stream.finish();

        Ok(chunk)
    }
}

/// Create a tag for a font subset.
pub(crate) fn subset_tag<T: Hash>(data: &T) -> String {
    const BASE: u128 = 26;
    let mut hash = hash128(data);
    let mut letter = [b'A'; SUBSET_TAG_LEN];
    for l in letter.iter_mut() {
        *l = b'A' + (hash % BASE) as u8;
        hash /= BASE;
    }
    std::str::from_utf8(&letter).unwrap().to_string()
}

pub(crate) fn base_font_name<T: Hash>(font: &Font, data: &T) -> String {
    const REST_LEN: usize = SUBSET_TAG_LEN + 1 + 1 + IDENTITY_H.len();

    let postscript_name = font.postscript_name().unwrap_or("unknown");
    let max_len = 127 - REST_LEN;
    let trimmed = &postscript_name[..postscript_name.len().min(max_len)];
    let subset_tag = subset_tag(&data);

    format!("{subset_tag}+{trimmed}")
}

#[cfg_attr(feature = "comemo", comemo::memoize)]
fn subset_font(font: Font, glyph_remapper: &GlyphRemapper) -> KrillaResult<Vec<u8>> {
    let font_data = font.font_data();
    subsetter::subset(font_data.as_ref(), font.index(), glyph_remapper)
        .map_err(|e| KrillaError::Font(font.clone(), format!("failed to subset font: {}", e)))
}
