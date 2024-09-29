use crate::error::{KrillaError, KrillaResult};
use crate::font::{CIDIdentifer, Font, FontIdentifier};
use crate::serialize::{FilterStream, SerializerContext};
use crate::util::{RectExt, SipHashable, SliceExt};
use crate::validation::ValidationError;
use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::writers::WMode;
use pdf_writer::{Chunk, Finish, Name, Ref, Str};
use skrifa::raw::tables::cff::Cff;
use skrifa::raw::{TableProvider, TopLevelTable};
use skrifa::GlyphId;
use std::collections::BTreeMap;
use std::ops::DerefMut;
use subsetter::GlyphRemapper;

const SUBSET_TAG_LEN: usize = 6;
const IDENTITY_H: &str = "Identity-H";
pub(crate) const CMAP_NAME: Name = Name(b"Custom");
pub(crate) const SYSTEM_INFO: SystemInfo = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

pub type Cid = u16;

/// A CID-keyed font.
#[derive(Debug, Clone)]
pub(crate) struct CIDFont {
    /// The _actual_ underlying font of the CID-keyed font.
    font: Font,
    /// A mapper that maps GIDs from the original font to CIDs, i.e. the corresponding GID in the font
    /// subset. The subsetter will ensure that for CID-keyed CFF fonts, the CID-to-GID mapping
    /// will be the identity mapping, regardless of what the mapping was in the original font. This
    /// allows us to index both, CFF and glyf-based fonts, transparently using GIDs,
    /// instead of having to distinguish according to the underlying font. See section
    /// 9.7.4.2 for more information on how glyphs are indexed in a CID-keyed font.
    glyph_remapper: GlyphRemapper,
    /// A mapping from CIDs to their string in the original text.
    cmap_entries: BTreeMap<u16, String>,
    /// The widths of the glyphs, _index by their CID_.
    widths: Vec<f32>,
}

impl CIDFont {
    /// Create a new CID-keyed font.
    pub fn new(font: Font) -> CIDFont {
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

    pub fn font(&self) -> Font {
        self.font.clone()
    }

    pub fn units_per_em(&self) -> f32 {
        1000.0
    }

    pub fn get_cid(&self, glyph_id: GlyphId) -> Option<u16> {
        self.glyph_remapper
            .get(u16::try_from(glyph_id.to_u32()).unwrap())
    }

    /// Add a new glyph (if it has not already been added) and return its CID.
    pub fn add_glyph(&mut self, glyph_id: GlyphId) -> Cid {
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

    pub fn get_codepoints(&self, cid: Cid) -> Option<&str> {
        self.cmap_entries.get(&cid).map(|s| s.as_str())
    }

    pub fn set_codepoints(&mut self, cid: Cid, text: String) {
        self.cmap_entries.insert(cid, text);
    }

    pub fn identifier(&self) -> FontIdentifier {
        FontIdentifier::Cid(CIDIdentifer(self.font.clone()))
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        root_ref: Ref,
    ) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let cid_ref = sc.new_ref();
        let descriptor_ref = sc.new_ref();
        let cmap_ref = sc.new_ref();
        let data_ref = sc.new_ref();

        let glyph_remapper = &self.glyph_remapper;

        let is_cff = self.font.font_ref().cff().is_ok();

        let subsetted = {
            let font_data = self.font.font_data();
            subsetter::subset(
                font_data.as_ref().as_ref(),
                self.font.index(),
                glyph_remapper,
            )
            .map_err(|e| {
                KrillaError::SubsetError(self.font.clone(), format!("failed to subset font: {}", e))
            })
        }?;

        let font_stream = {
            let mut data = subsetted.as_slice();

            // If we have a CFF font, only embed the standalone CFF program.
            let subsetted_ref = skrifa::FontRef::new(data).map_err(|_| {
                KrillaError::SubsetError(
                    self.font.clone(),
                    "failed to read font subset".to_string(),
                )
            })?;
            if let Some(cff) = subsetted_ref.data_for_tag(Cff::TAG) {
                data = cff.as_bytes();
            }

            FilterStream::new_from_binary_data(data, &sc.serialize_settings)
        };

        let base_font = base_font_name(&self.font, font_stream.encoded_data());
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
                    Some(text) => {
                        if text
                            .chars()
                            .any(|c| matches!(c as u32, 0x0 | 0xFEFF | 0xFFFE))
                            || text.is_empty()
                        {
                            sc.register_validation_error(ValidationError::InvalidCodepointMapping(
                                self.font.clone(),
                                GlyphId::new(g as u32),
                                Some(text.clone()),
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
fn subset_tag(subsetted_font: &[u8]) -> String {
    const BASE: u128 = 26;
    let mut hash = subsetted_font.sip_hash();
    let mut letter = [b'A'; SUBSET_TAG_LEN];
    for l in letter.iter_mut() {
        *l = b'A' + (hash % BASE) as u8;
        hash /= BASE;
    }
    std::str::from_utf8(&letter).unwrap().to_string()
}

fn base_font_name(font: &Font, subset_data: &[u8]) -> String {
    const REST_LEN: usize = SUBSET_TAG_LEN + 1 + 1 + IDENTITY_H.len();
    let postscript_name = font.postscript_name().unwrap_or("unknown");

    let max_len = 127 - REST_LEN;

    let trimmed = &postscript_name[..postscript_name.len().min(max_len)];

    // Hash the full name (we might have trimmed) and the glyphs to produce
    // a fairly unique subset tag.
    let subset_tag = subset_tag(&subset_data);

    format!("{subset_tag}+{trimmed}")
}

#[cfg(test)]
mod tests {
    use crate::font::Font;
    use std::sync::Arc;

    use crate::path::Fill;
    use crate::serialize::{FontContainer, SerializerContext};
    use crate::surface::{Surface, TextDirection};
    use crate::tests::{LATIN_MODERN_ROMAN, NOTO_SANS, NOTO_SANS_ARABIC};
    use krilla_macros::{snapshot, visreg};
    use skrifa::GlyphId;
    use tiny_skia_path::Point;

    #[snapshot]
    fn cid_font_noto_sans_two_glyphs(sc: &mut SerializerContext) {
        let font = Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap();
        let container = sc.create_or_get_font_container(font.clone());
        let mut font_container = container.borrow_mut();

        match &mut *font_container {
            FontContainer::Type3(_) => panic!("expected CID font"),
            FontContainer::CIDFont(cid_font) => {
                cid_font.add_glyph(GlyphId::new(36));
                cid_font.add_glyph(GlyphId::new(37));
                cid_font.set_codepoints(1, "A".to_string());
                cid_font.set_codepoints(2, "B".to_string());
            }
        }
    }

    #[visreg(all)]
    fn cid_font_noto_sans_simple_text(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap();
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
        let font = Font::new(LATIN_MODERN_ROMAN.clone(), 0, vec![]).unwrap();
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
        let font = Font::new(NOTO_SANS_ARABIC.clone(), 0, vec![]).unwrap();
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
    fn cid_font_latin_modern_four_glyphs(sc: &mut SerializerContext) {
        let font = Font::new(LATIN_MODERN_ROMAN.clone(), 0, vec![]).unwrap();
        let container = sc.create_or_get_font_container(font.clone());
        let mut font_container = container.borrow_mut();

        match &mut *font_container {
            FontContainer::Type3(_) => panic!("expected CID font"),
            FontContainer::CIDFont(cid_font) => {
                cid_font.add_glyph(GlyphId::new(58));
                cid_font.add_glyph(GlyphId::new(54));
                cid_font.add_glyph(GlyphId::new(69));
                cid_font.add_glyph(GlyphId::new(71));
                cid_font.set_codepoints(1, "G".to_string());
                cid_font.set_codepoints(2, "F".to_string());
                cid_font.set_codepoints(3, "K".to_string());
                cid_font.set_codepoints(4, "L".to_string());
            }
        }
    }

    #[visreg(macos)]
    fn cid_font_true_type_collection(surface: &mut Surface) {
        let font_data =
            Arc::new(std::fs::read("/System/Library/Fonts/Supplemental/Songti.ttc").unwrap());
        let font_1 = Font::new(font_data.clone(), 0, vec![]).unwrap();
        let font_2 = Font::new(font_data.clone(), 3, vec![]).unwrap();
        let font_3 = Font::new(font_data, 6, vec![]).unwrap();

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
