use crate::font::{CIDIdentifer, Font, FontIdentifier};
use crate::serialize::{SerializerContext, SipHashable};
use crate::util::{RectExt, SliceExt};
use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Chunk, Filter, Finish, Name, Ref, Str};
use skrifa::raw::tables::cff::Cff;
use skrifa::raw::{TableProvider, TopLevelTable};
use skrifa::GlyphId;
use std::collections::BTreeMap;
use subsetter::GlyphRemapper;

const CMAP_NAME: Name = Name(b"Custom");
const SYSTEM_INFO: SystemInfo = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

pub type Cid = u16;

/// A CID-keyed font.
#[derive(Debug, Clone)]
pub struct CIDFont {
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

    pub fn to_pdf_font_units(&self, val: f32) -> f32 {
        val / self.font.units_per_em() * 1000.0
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

    pub(crate) fn serialize_into(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        let cid_ref = sc.new_ref();
        let descriptor_ref = sc.new_ref();
        let cmap_ref = sc.new_ref();
        let data_ref = sc.new_ref();

        let glyph_remapper = &self.glyph_remapper;

        let is_cff = self.font.font_ref().cff().is_ok();

        let (subsetted_font, filter) =
            subset_font(sc, &self.font, self.font.index(), &glyph_remapper);

        let postscript_name = self.font.postscript_name().unwrap_or("unknown");
        let subset_tag = subset_tag(&subsetted_font);

        let base_font = format!("{subset_tag}+{postscript_name}");
        let base_font_type0 = if is_cff {
            format!("{base_font}-Identity-H")
        } else {
            base_font.clone()
        };

        chunk
            .type0_font(root_ref)
            .base_font(Name(base_font_type0.as_bytes()))
            .encoding_predefined(Name(b"Identity-H"))
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

        let mut first = 0;
        let mut width_writer = cid.widths();
        for (w, group) in self.widths.group_by_key(|&w| w) {
            let end = first + group.len();
            if w != 0.0 {
                let last = end - 1;
                width_writer.same(first as u16, last as u16, self.to_pdf_font_units(w));
            }
            first = end;
        }

        width_writer.finish();
        cid.finish();

        let mut flags = FontFlags::empty();
        flags.set(FontFlags::SERIF, postscript_name.contains("Serif"));
        flags.set(FontFlags::FIXED_PITCH, self.font.is_monospaced());
        flags.set(FontFlags::ITALIC, self.font.italic_angle() != 0.0);
        flags.insert(FontFlags::SYMBOLIC);
        flags.insert(FontFlags::SMALL_CAP);

        let bbox = self.font.bbox().to_pdf_rect();

        let italic_angle = self.font.italic_angle();
        let ascender = self.to_pdf_font_units(self.font.ascent());
        let descender = self.to_pdf_font_units(self.font.descent());
        let cap_height = self
            .font
            .cap_height()
            .map(|h| self.to_pdf_font_units(h))
            .unwrap_or(ascender);
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
            for (g, text) in self.cmap_entries.iter() {
                if !text.is_empty() {
                    cmap.pair_with_multiple(*g, text.chars());
                }
            }

            cmap
        };

        chunk.cmap(cmap_ref, &cmap.finish());

        let mut stream = chunk.stream(data_ref, &subsetted_font);
        stream.filter(filter);
        if is_cff {
            stream.pair(Name(b"Subtype"), Name(b"CIDFontType0C"));
        }

        stream.finish();

        chunk
    }
}

/// Subset a font with the given glyphs.
fn subset_font(
    sc: &SerializerContext,
    font: &Font,
    index: u32,
    glyph_remapper: &GlyphRemapper,
) -> (Vec<u8>, Filter) {
    let font_data = font.font_data();
    let subsetted = subsetter::subset(font_data.as_ref().as_ref(), index, glyph_remapper).unwrap();
    let mut data = subsetted.as_slice();

    // If we have a CFF font, only embed the standalone CFF program.
    let subsetted_ref = skrifa::FontRef::new(data).unwrap();
    if let Some(cff) = subsetted_ref.data_for_tag(Cff::TAG) {
        data = cff.as_bytes();
    }

    sc.get_binary_stream(data)
}

/// Create a tag for a font subset.
fn subset_tag(subsetted_font: &[u8]) -> String {
    const LEN: usize = 6;
    const BASE: u128 = 26;
    let mut hash = subsetted_font.sip_hash();
    let mut letter = [b'A'; LEN];
    for l in letter.iter_mut() {
        *l = b'A' + (hash % BASE) as u8;
        hash /= BASE;
    }
    std::str::from_utf8(&letter).unwrap().to_string()
}

#[cfg(test)]
mod tests {
    use crate::font::Font;

    use crate::serialize::{SerializeSettings, SerializerContext};
    use crate::test_utils::{check_snapshot, load_font};
    use krilla_macros::snapshot;
    use skrifa::instance::Location;
    use skrifa::GlyphId;
    use std::sync::Arc;

    #[snapshot(cid_font)]
    fn noto_sans_two_glyphs(sc: &mut SerializerContext) {
        let font_data = Arc::new(load_font("NotoSans-Regular.ttf"));
        let font = Font::new(font_data, 0, Location::default()).unwrap();
        sc.create_or_get_font_container(font.clone())
            .borrow_mut()
            .add_glyph(GlyphId::new(36));
        sc.create_or_get_font_container(font.clone())
            .borrow_mut()
            .add_glyph(GlyphId::new(37));
    }

    #[snapshot(cid_font)]
    fn latin_modern_four_glyphs(sc: &mut SerializerContext) {
        let font_data = Arc::new(load_font("LatinModernRoman-Regular.otf"));
        let font = Font::new(font_data, 0, Location::default()).unwrap();
        sc.create_or_get_font_container(font.clone())
            .borrow_mut()
            .add_glyph(GlyphId::new(58));
        sc.create_or_get_font_container(font.clone())
            .borrow_mut()
            .add_glyph(GlyphId::new(54));
        sc.create_or_get_font_container(font.clone())
            .borrow_mut()
            .add_glyph(GlyphId::new(69));
        sc.create_or_get_font_container(font.clone())
            .borrow_mut()
            .add_glyph(GlyphId::new(71));
    }
}
