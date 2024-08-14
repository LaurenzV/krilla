use crate::font::{Font, Glyph};
use crate::serialize::{hash_item, SerializerContext};
use crate::util::RectExt;
use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Chunk, Filter, Finish, Name, Ref, Str};
use skrifa::raw::tables::cff::Cff;
use skrifa::raw::{TableProvider, TopLevelTable};
use skrifa::{FontRef, GlyphId};
use std::collections::BTreeMap;
use subsetter::GlyphRemapper;

const CMAP_NAME: Name = Name(b"Custom");
const SYSTEM_INFO: SystemInfo = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

#[derive(Debug, Clone)]
pub struct CIDFont {
    font: Font,
    glyph_remapper: GlyphRemapper,
    strings: BTreeMap<GlyphId, String>,
    widths: Vec<f32>,
}

impl CIDFont {
    pub fn new(font: Font) -> CIDFont {
        let widths = vec![font.advance_width(GlyphId::new(0)).unwrap_or(0.0)];
        Self {
            glyph_remapper: GlyphRemapper::new(),
            strings: BTreeMap::new(),
            widths,
            font,
        }
    }

    pub fn index(&self) -> u32 {
        self.font.index()
    }

    pub fn advance_width(&self, glyph_id: u16) -> Option<f32> {
        self.widths.get(glyph_id as usize).copied()
    }

    pub fn units_per_em(&self) -> u16 {
        self.font.units_per_em()
    }

    pub fn to_font_units(&self, val: f32) -> f32 {
        val / self.font.units_per_em() as f32 * 1000.0
    }

    pub fn remap(&mut self, glyph: &Glyph) -> GlyphId {
        let new_id = GlyphId::new(self.glyph_remapper.remap(glyph.glyph_id.to_u32() as u16) as u32);
        self.strings.insert(new_id, glyph.string.clone());

        // This means that the glyph ID has been newly assigned.
        if new_id.to_u32() >= self.widths.len() as u32 {
            self.widths
                .push(self.font.advance_width(glyph.glyph_id).unwrap_or(0.0));
        }
        new_id
    }

    pub(crate) fn serialize_into(
        self,
        sc: &mut SerializerContext,
        font_ref: &FontRef,
        root_ref: Ref,
    ) -> Chunk {
        let mut chunk = Chunk::new();

        let cid_ref = sc.new_ref();
        let descriptor_ref = sc.new_ref();
        let cmap_ref = sc.new_ref();
        let data_ref = sc.new_ref();

        let glyph_remapper = &self.glyph_remapper;

        let is_cff = font_ref.cff().is_ok();

        // Subset and write the font's bytes.
        let (subsetted_font, filter) = subset_font(sc, font_ref.data().as_bytes(), &glyph_remapper);

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

        // Write the CID font referencing the font descriptor.
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

        // Write all non-zero glyph widths.
        let mut first = 0;
        let mut width_writer = cid.widths();
        for (w, group) in self.widths.group_by_key(|&w| w) {
            let end = first + group.len();
            if w != 0.0 {
                let last = end - 1;
                width_writer.same(first as u16, last as u16, self.to_font_units(w));
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
        let ascender = self.to_font_units(self.font.ascent());
        let descender = self.to_font_units(self.font.descent());
        let cap_height = self
            .font
            .cap_height()
            .map(|h| self.to_font_units(h))
            .unwrap_or(ascender);
        let stem_v = 10.0 + 0.244 * (self.font.weight() - 50.0);

        // Write the font descriptor (contains metrics about the font).
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
            for (g, text) in self.strings.iter() {
                if !text.is_empty() {
                    cmap.pair_with_multiple(g.to_u32() as u16, text.chars());
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

fn subset_font(
    sc: &SerializerContext,
    font_data: &[u8],
    glyph_remapper: &GlyphRemapper,
) -> (Vec<u8>, Filter) {
    let subsetted = subsetter::subset(font_data, 0, glyph_remapper).unwrap();
    let mut data = subsetted.as_slice();

    // Extract the standalone CFF font program if applicable.
    let face = skrifa::FontRef::new(data).unwrap();
    if let Some(cff) = face.data_for_tag(Cff::TAG) {
        data = cff.as_bytes();
    }

    sc.get_binary_stream(data)
}

/// Extra methods for [`[T]`](slice).
pub trait SliceExt<T> {
    /// Split a slice into consecutive runs with the same key and yield for
    /// each such run the key and the slice of elements with that key.
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F>
    where
        F: FnMut(&T) -> K,
        K: PartialEq;
}

impl<T> SliceExt<T> for [T] {
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F> {
        GroupByKey { slice: self, f }
    }
}

/// This struct is created by [`SliceExt::group_by_key`].
pub struct GroupByKey<'a, T, F> {
    slice: &'a [T],
    f: F,
}

impl<'a, T, K, F> Iterator for GroupByKey<'a, T, F>
where
    F: FnMut(&T) -> K,
    K: PartialEq,
{
    type Item = (K, &'a [T]);

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.slice.iter();
        let key = (self.f)(iter.next()?);
        let count = 1 + iter.take_while(|t| (self.f)(t) == key).count();
        let (head, tail) = self.slice.split_at(count);
        self.slice = tail;
        Some((key, head))
    }
}

fn subset_tag(subsetted_font: &[u8]) -> String {
    const LEN: usize = 6;
    const BASE: u128 = 26;
    let mut hash = hash_item(subsetted_font);
    let mut letter = [b'A'; LEN];
    for l in letter.iter_mut() {
        *l = b'A' + (hash % BASE) as u8;
        hash /= BASE;
    }
    std::str::from_utf8(&letter).unwrap().to_string()
}

#[cfg(test)]
mod tests {
    use crate::font::{Font, Glyph};

    use crate::serialize::{SerializeSettings, SerializerContext};
    use crate::test_utils::{check_snapshot, load_font};
    use skrifa::instance::Location;
    use skrifa::GlyphId;
    use std::sync::Arc;

    fn sc() -> SerializerContext {
        let settings = SerializeSettings::default_test();
        SerializerContext::new(settings)
    }

    #[test]
    fn noto_sans_two_glyphs() {
        let mut sc = sc();
        let font_data = Arc::new(load_font("NotoSans-Regular.ttf"));
        let font = Font::new(font_data, 0, Location::default()).unwrap();
        sc.map_glyph(font.clone(), Glyph::new(GlyphId::new(36), "A".to_string()));
        sc.map_glyph(font.clone(), Glyph::new(GlyphId::new(37), "B".to_string()));
        check_snapshot("cid_font/noto_sans_two_glyphs", sc.finish().as_bytes());
    }

    #[test]
    fn latin_modern_four_glyphs() {
        let mut sc = sc();
        let font_data = Arc::new(load_font("LatinModernRoman-Regular.otf"));
        let font = Font::new(font_data, 0, Location::default()).unwrap();
        sc.map_glyph(font.clone(), Glyph::new(GlyphId::new(58), "G".to_string()));
        sc.map_glyph(font.clone(), Glyph::new(GlyphId::new(54), "F".to_string()));
        sc.map_glyph(font.clone(), Glyph::new(GlyphId::new(69), "K".to_string()));
        sc.map_glyph(font.clone(), Glyph::new(GlyphId::new(71), "L".to_string()));
        check_snapshot("cid_font/latin_modern_four_glyphs", sc.finish().as_bytes());
    }
}
