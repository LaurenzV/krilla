use crate::font::{Font, FontInfo, Glyph};
use crate::serialize::{hash_item, SerializerContext};
use crate::util::{deflate, RectExt};
use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Filter, Finish, Name, Ref, Str};
use skrifa::prelude::Size;
use skrifa::raw::tables::cff::Cff;
use skrifa::raw::types::NameId;
use skrifa::raw::{TableProvider, TopLevelTable};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use std::collections::BTreeMap;
use std::sync::Arc;
use subsetter::GlyphRemapper;

const CMAP_NAME: Name = Name(b"Custom");
const SYSTEM_INFO: SystemInfo = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

#[derive(Debug, Clone)]
pub struct CIDFont<'a> {
    font: Font<'a>,
    glyph_remapper: GlyphRemapper,
    strings: BTreeMap<GlyphId, String>,
}

impl<'a> CIDFont<'a> {
    pub fn new(font: Font<'a>) -> CIDFont<'a> {
        Self {
            glyph_remapper: GlyphRemapper::new(),
            strings: BTreeMap::new(),
            font,
        }
    }

    pub fn remap(&mut self, glyph: &Glyph) -> GlyphId {
        let new_id = GlyphId::new(self.glyph_remapper.remap(glyph.glyph_id.to_u32() as u16) as u32);
        self.strings.insert(new_id, glyph.string.clone());
        new_id
    }

    pub(crate) fn serialize_into(
        self,
        sc: &mut SerializerContext,
        font_ref: &FontRef,
        root_ref: Ref,
    ) {
        let units_per_em = self.font.units_per_em();

        let cid_ref = sc.new_ref();
        let descriptor_ref = sc.new_ref();
        let cmap_ref = sc.new_ref();
        let data_ref = sc.new_ref();

        let glyph_remapper = &self.glyph_remapper;

        let is_cff = font_ref.cff().is_ok();

        let postscript_name = find_name(&font_ref).unwrap_or("unknown".to_string());
        let subset_tag = subset_tag(&self);

        let base_font = format!("{subset_tag}+{postscript_name}");
        let base_font_type0 = if is_cff {
            format!("{base_font}-Identity-H")
        } else {
            base_font.clone()
        };

        sc.chunk_mut()
            .type0_font(root_ref)
            .base_font(Name(base_font_type0.as_bytes()))
            .encoding_predefined(Name(b"Identity-H"))
            .descendant_font(cid_ref)
            .to_unicode(cmap_ref);

        // Write the CID font referencing the font descriptor.
        let mut cid = sc.chunk_mut().cid_font(cid_ref);
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

        let mut widths = vec![];
        for old_gid in glyph_remapper.remapped_gids() {
            let width = font_ref
                .glyph_metrics(Size::unscaled(), self.font.location_ref())
                .advance_width(GlyphId::new(old_gid as u32))
                .unwrap_or(0.0);
            let units = (width as f64 / units_per_em as f64) * 1000.0;
            widths.push(units as f32);
        }

        // Write all non-zero glyph widths.
        let mut first = 0;
        let mut width_writer = cid.widths();
        for (w, group) in widths.group_by_key(|&w| w) {
            let end = first + group.len();
            if w != 0.0 {
                let last = end - 1;
                width_writer.same(first as u16, last as u16, w);
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

        let convert = |val| (val / units_per_em as f32) * 1000.0;

        let bbox = self.font.bbox().to_pdf_rect();

        let italic_angle = self.font.italic_angle();
        let ascender = convert(self.font.ascent());
        let descender = convert(self.font.descent());
        let cap_height = self
            .font
            .cap_height()
            .map(|h| convert(h))
            .unwrap_or(ascender);
        let stem_v = 10.0 + 0.244 * (self.font.weight() - 50.0);

        let cmap = {
            let mut cmap = UnicodeCmap::new(CMAP_NAME, SYSTEM_INFO);
            for (g, text) in self.strings.iter() {
                if !text.is_empty() {
                    cmap.pair_with_multiple(g.to_u32() as u16, text.chars());
                }
            }

            cmap
        };

        sc.chunk_mut().cmap(cmap_ref, &cmap.finish());

        // Write the font descriptor (contains metrics about the font).
        let mut font_descriptor = sc.chunk_mut().font_descriptor(descriptor_ref);
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

        // Subset and write the font's bytes.
        let data = subset_font(font_ref.data().as_bytes(), &glyph_remapper);

        let mut stream = sc.chunk_mut().stream(data_ref, &data);
        stream.filter(Filter::FlateDecode);
        if is_cff {
            stream.pair(Name(b"Subtype"), Name(b"CIDFontType0C"));
        }

        stream.finish();
    }
}

fn subset_font(font_data: &[u8], glyph_remapper: &GlyphRemapper) -> Vec<u8> {
    let subsetted = subsetter::subset(font_data, 0, glyph_remapper).unwrap();
    let mut data = subsetted.as_slice();

    // Extract the standalone CFF font program if applicable.
    let face = skrifa::FontRef::new(data).unwrap();
    if let Some(cff) = face.data_for_tag(Cff::TAG) {
        data = cff.as_bytes();
    }

    deflate(data)
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

fn subset_tag(cid_font: &CIDFont) -> String {
    const LEN: usize = 6;
    const BASE: u128 = 26;
    // TODO: FIXME
    let mut hash = 0;
    let mut letter = [b'A'; LEN];
    for l in letter.iter_mut() {
        *l = b'A' + (hash % BASE) as u8;
        hash /= BASE;
    }
    std::str::from_utf8(&letter).unwrap().to_string()
}

pub fn find_name(font_ref: &FontRef) -> Option<String> {
    if let Ok(name) = font_ref.name() {
        name.name_record().iter().find_map(|n| {
            if n.name_id.get() == NameId::POSTSCRIPT_NAME {
                if let Ok(string) = n.string(name.string_data()) {
                    return Some(string.to_string());
                }
            }

            return None;
        })
    } else {
        return None;
    }
}
