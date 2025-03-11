//! CID fonts.

use std::collections::BTreeMap;
use std::hash::Hash;
use std::ops::DerefMut;

use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::writers::WMode;
use pdf_writer::{Chunk, Finish, Name, Ref, Str};
use skrifa::raw::tables::cff::Cff;
use skrifa::raw::{TableProvider, TopLevelTable};
use skrifa::GlyphId;
use subsetter::GlyphRemapper;

use super::{CIDIdentifer, FontIdentifier};
use crate::configure::ValidationError;
use crate::error::{KrillaError, KrillaResult};
use crate::font::Font;
use crate::serialize::SerializeContext;
use crate::stream::FilterStreamBuilder;
use crate::surface::Location;
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
    /// The widths of the glyphs, _index by their CID_.
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
        1000.0
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
        if !text.is_empty() {
            self.cmap_entries.insert(cid, (text, location));
        }
    }

    #[inline]
    pub(crate) fn identifier(&self) -> FontIdentifier {
        FontIdentifier::Cid(CIDIdentifer(self.font.clone()))
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
                Err(KrillaError::FontError(
                    self.font.clone(),
                    "CFF2 fonts are not supported".to_string(),
                ))
            } else {
                Err(KrillaError::FontError(
                    self.font.clone(),
                    "font is missing `glyf` or `CFF` table".to_string(),
                ))
            };
        }

        let subsetted = subset_font(self.font.clone(), glyph_remapper)?;

        let font_stream = {
            let mut data = subsetted.as_slice();

            // If we have a CFF font, only embed the standalone CFF program.
            let subsetted_ref = skrifa::FontRef::new(data).map_err(|_| {
                KrillaError::FontError(self.font.clone(), "failed to read font subset".to_string())
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

        if !sc.serialize_settings().pdf_version().deprecates_cid_set() {
            let cid_stream_data = {
                // It's always guaranteed by the subsetter that CIDs start from 0 and are
                // consecutive, so this encoding is very straight-forward.
                let mut bytes = vec![];
                bytes.extend([0xFFu8].repeat((self.glyph_remapper.num_gids() / 8) as usize));
                let padding = self.glyph_remapper.num_gids() % 8;
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
                match self.cmap_entries.get(&g) {
                    None => sc.register_validation_error(ValidationError::InvalidCodepointMapping(
                        self.font.clone(),
                        GlyphId::new(g as u32),
                        None,
                    )),
                    Some((text, loc)) => {
                        // Note: Keep in sync with Type3
                        let mut invalid_codepoint = false;
                        let mut private_unicode = false;

                        for c in text.chars() {
                            invalid_codepoint |= matches!(c as u32, 0x0 | 0xFEFF | 0xFFFE);
                            private_unicode |= matches!(c as u32, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD);
                        }

                        if invalid_codepoint {
                            sc.register_validation_error(ValidationError::InvalidCodepointMapping(
                                self.font.clone(),
                                GlyphId::new(g as u32),
                                *loc,
                            ))
                        }

                        if private_unicode {
                            sc.register_validation_error(ValidationError::UnicodePrivateArea(
                                self.font.clone(),
                                GlyphId::new(g as u32),
                                *loc,
                            ))
                        }

                        if !text.is_empty() {
                            cmap.pair_with_multiple(g, text.chars());
                        }
                    }
                }
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

    // Hash the full name (we might have trimmed) and the glyphs to produce
    // a fairly unique subset tag.
    let subset_tag = subset_tag(&data);

    format!("{subset_tag}+{trimmed}")
}

#[cfg_attr(feature = "comemo", comemo::memoize)]
fn subset_font(font: Font, glyph_remapper: &GlyphRemapper) -> KrillaResult<Vec<u8>> {
    let font_data = font.font_data();
    subsetter::subset(font_data.as_ref(), font.index(), glyph_remapper)
        .map_err(|e| KrillaError::FontError(font.clone(), format!("failed to subset font: {}", e)))
}

#[cfg(test)]
mod tests {
    use crate::font::Font;

    use crate::object::font::FontContainer;
    use crate::path::Fill;
    use crate::serialize::SerializeContext;
    use crate::surface::{Surface, TextDirection};
    use crate::tests::{LATIN_MODERN_ROMAN, NOTO_SANS, NOTO_SANS_ARABIC};
    use krilla_macros::{snapshot, visreg};
    use skrifa::GlyphId;
    use tiny_skia_path::Point;

    #[snapshot]
    fn cid_font_noto_sans_two_glyphs(sc: &mut SerializeContext) {
        let font = Font::new(NOTO_SANS.clone(), 0, true).unwrap();
        let container = sc.register_font_container(font.clone());
        let mut font_container = container.borrow_mut();

        match &mut *font_container {
            FontContainer::Type3(_) => panic!("expected CID font"),
            FontContainer::CIDFont(cid_font) => {
                cid_font.add_glyph(GlyphId::new(36));
                cid_font.add_glyph(GlyphId::new(37));
                cid_font.set_codepoints(1, "A".to_string(), None);
                cid_font.set_codepoints(2, "B".to_string(), None);
            }
        }
    }

    #[visreg(all)]
    fn cid_font_noto_sans_simple_text(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS.clone(), 0, true).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            32.0,
            &[],
            "hello world",
            false,
            TextDirection::Auto,
        );
    }

    #[visreg(all)]
    fn cid_font_latin_modern_simple_text(surface: &mut Surface) {
        let font = Font::new(LATIN_MODERN_ROMAN.clone(), 0, true).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            32.0,
            &[],
            "hello world",
            false,
            TextDirection::Auto,
        );
    }

    #[visreg(all)]
    fn cid_font_noto_arabic_simple_text(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS_ARABIC.clone(), 0, true).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            32.0,
            &[],
            "مرحبا بالعالم",
            false,
            TextDirection::Auto,
        );
    }

    #[snapshot]
    fn cid_font_latin_modern_four_glyphs(sc: &mut SerializeContext) {
        let font = Font::new(LATIN_MODERN_ROMAN.clone(), 0, true).unwrap();
        let container = sc.register_font_container(font.clone());
        let mut font_container = container.borrow_mut();

        match &mut *font_container {
            FontContainer::Type3(_) => panic!("expected CID font"),
            FontContainer::CIDFont(cid_font) => {
                cid_font.add_glyph(GlyphId::new(58));
                cid_font.add_glyph(GlyphId::new(54));
                cid_font.add_glyph(GlyphId::new(69));
                cid_font.add_glyph(GlyphId::new(71));
                cid_font.set_codepoints(1, "G".to_string(), None);
                cid_font.set_codepoints(2, "F".to_string(), None);
                cid_font.set_codepoints(3, "K".to_string(), None);
                cid_font.set_codepoints(4, "L".to_string(), None);
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[visreg(macos)]
    fn cid_font_true_type_collection(surface: &mut Surface) {
        let font_data: crate::Data = std::fs::read("/System/Library/Fonts/Supplemental/Songti.ttc")
            .unwrap()
            .into();
        let font_1 = Font::new(font_data.clone(), 0, true).unwrap();
        let font_2 = Font::new(font_data.clone(), 3, true).unwrap();
        let font_3 = Font::new(font_data, 6, true).unwrap();

        surface.fill_text(
            Point::from_xy(0.0, 75.0),
            Fill::default(),
            font_1.clone(),
            20.0,
            &[],
            "这是一段测试文字。",
            false,
            TextDirection::Auto,
        );
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font_2.clone(),
            20.0,
            &[],
            "这是一段测试文字。",
            false,
            TextDirection::Auto,
        );
        surface.fill_text(
            Point::from_xy(0.0, 125.0),
            Fill::default(),
            font_3.clone(),
            20.0,
            &[],
            "这是一段测试文字。",
            false,
            TextDirection::Auto,
        );
    }
}
