use super::{FontIdentifier, OwnedPaintMode, PaintMode, Type3Identifier};
use crate::font::outline::glyph_path;
use crate::font::Font;
use crate::object::font::cid_font::{CMAP_NAME, IDENTITY_H, SYSTEM_INFO};
use crate::object::xobject::XObject;
use crate::path::Fill;
use crate::resource::ResourceDictionaryBuilder;
use crate::serialize::SerializerContext;
use crate::stream::{FilterStream, StreamBuilder};
use crate::util::{NameExt, RectExt, TransformExt};
use crate::validation::ValidationError;
use crate::version::PdfVersion;
use crate::{font, SvgSettings};
use pdf_writer::types::{FontFlags, UnicodeCmap};
use pdf_writer::writers::WMode;
use pdf_writer::{Chunk, Content, Finish, Name, Ref, Str};
use skrifa::GlyphId;
use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::DerefMut;
use tiny_skia_path::{Rect, Transform};

pub type Gid = u8;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CoveredGlyph<'a> {
    pub glyph_id: GlyphId,
    pub paint_mode: PaintMode<'a>,
}

impl CoveredGlyph<'_> {
    pub fn to_owned(self) -> OwnedCoveredGlyph {
        OwnedCoveredGlyph {
            glyph_id: self.glyph_id,
            paint_mode: self.paint_mode.to_owned(),
        }
    }
}

impl<'a> CoveredGlyph<'a> {
    pub fn new(glyph_id: GlyphId, paint_mode: PaintMode<'a>) -> CoveredGlyph<'a> {
        Self {
            glyph_id,
            paint_mode,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OwnedCoveredGlyph {
    glyph_id: GlyphId,
    paint_mode: OwnedPaintMode,
}

impl Eq for OwnedCoveredGlyph {}

impl Hash for OwnedCoveredGlyph {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.glyph_id.hash(state);
        self.paint_mode.hash(state);
    }
}

#[derive(Debug)]
pub(crate) struct Type3Font {
    font: Font,
    glyphs: Vec<OwnedCoveredGlyph>,
    widths: Vec<f32>,
    cmap_entries: BTreeMap<Gid, String>,
    glyph_set: HashSet<OwnedCoveredGlyph>,
    index: usize,
}

impl Type3Font {
    pub fn new(font: Font, index: usize) -> Self {
        Self {
            font,
            glyphs: Vec::new(),
            cmap_entries: BTreeMap::new(),
            widths: Vec::new(),
            glyph_set: HashSet::new(),
            index,
        }
    }

    // Unlike CID fonts, the units per em of type 3 fonts does not have to be 1000.
    pub fn unit_per_em(&self) -> f32 {
        self.font.units_per_em()
    }

    pub fn is_full(&self) -> bool {
        self.count() == 256
    }

    pub fn count(&self) -> u16 {
        u16::try_from(self.glyphs.len()).unwrap()
    }

    #[inline]
    pub fn covers(&self, glyph: &OwnedCoveredGlyph) -> bool {
        self.glyph_set.contains(glyph)
    }

    #[inline]
    pub fn get_gid(&self, glyph: &OwnedCoveredGlyph) -> Option<u8> {
        self.glyphs
            .iter()
            .position(|g| g == glyph)
            .and_then(|n| u8::try_from(n).ok())
    }

    #[inline]
    pub fn add_glyph(&mut self, glyph: OwnedCoveredGlyph) -> u8 {
        if let Some(pos) = self.get_gid(&glyph) {
            pos
        } else {
            assert!(self.glyphs.len() < 256);

            self.glyph_set.insert(glyph.clone());
            self.glyphs.push(glyph.clone());
            self.widths
                .push(self.font.advance_width(glyph.glyph_id).unwrap_or(0.0));
            u8::try_from(self.glyphs.len() - 1).unwrap()
        }
    }

    #[inline]
    pub fn get_codepoints(&self, gid: Gid) -> Option<&str> {
        self.cmap_entries.get(&gid).map(|s| s.as_str())
    }

    #[inline]
    pub fn set_codepoints(&mut self, gid: Gid, text: String) {
        self.cmap_entries.insert(gid, text);
    }

    #[inline]
    pub fn font(&self) -> Font {
        self.font.clone()
    }

    #[inline]
    pub fn identifier(&self) -> FontIdentifier {
        FontIdentifier::Type3(Type3Identifier(self.font.clone(), self.index))
    }

    pub(crate) fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        let mut rd_builder = ResourceDictionaryBuilder::new();
        let mut font_bbox = Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap();

        let glyph_streams =
            self.glyphs
                .iter()
                .enumerate()
                .map(|(index, glyph)| {
                    let mut stream_surface = StreamBuilder::new(sc);
                    let mut surface = stream_surface.surface();

                    // In case this returns `None`, the surface is guaranteed to be empty.
                    let drawn_color_glyph = font::draw_color_glyph(
                        self.font.clone(),
                        SvgSettings::default(),
                        glyph.glyph_id,
                        glyph.paint_mode.as_ref(),
                        Transform::default(),
                        &mut surface,
                    );

                    if drawn_color_glyph.is_none() {
                        // If this code path is reached, it means we tried to create a Type3
                        // font from a font that does not have any (valid) color table. This
                        // should be avoided because non-color fonts should always be embedded
                        // as TrueType/CFF fonts. The problem is that while PDF has support for
                        // so-called "shape-glyphs" that should allow you to create Type3 fonts
                        // based just on the outline, they have very bad viewer support. For
                        // example, filled glyphs with gradients become broken in some viewers,
                        // and stroking does not work at all consistently. So this code path
                        // should never be reached, but it can for example be reached if someone
                        // tries to write a glyph in a COLR font that doesn't have a corresponding
                        // COLR glyph. As a last resort solution, we try to fill the glyph
                        // outline in the corresponding color, but note that stroking will not
                        // be supported at all.

                        // If this is the case (i.e. no color glyph was drawn, either because no table
                        // exists or an error occurred, the surface is guaranteed to be empty.
                        // So we can just safely draw the outline glyph instead without having to
                        // worry about the surface being "dirty".
                        if let Some((path, fill)) = glyph_path(self.font.clone(), glyph.glyph_id)
                            .map(|p| match &glyph.paint_mode {
                                OwnedPaintMode::Fill(f) => (p, f.clone()),
                                OwnedPaintMode::Stroke(s) => (
                                    p,
                                    Fill {
                                        paint: s.paint.clone(),
                                        opacity: s.opacity,
                                        rule: Default::default(),
                                    },
                                ),
                            })
                        {
                            surface.fill_path(&path, fill);
                        }
                    };

                    surface.finish();
                    let stream = stream_surface.finish();
                    let mut content = Content::new();

                    // I considered writing into the stream directly instead of creating an XObject
                    // and showing that, but it seems like many viewers don't like that, and emojis
                    // look messed up. Using XObjects seems like the best choice here.
                    content.start_color_glyph(self.widths[index]);
                    let x_object = XObject::new(stream, false, false, None);
                    if !x_object.is_empty() {
                        font_bbox.expand(&x_object.bbox());
                        let x_name = rd_builder.register_resource(x_object, sc);
                        content.x_object(x_name.to_pdf_name());
                    }

                    let stream = content.finish();

                    let font_stream = FilterStream::new_from_content_stream(
                        stream.as_slice(),
                        &sc.serialize_settings(),
                    );

                    let stream_ref = sc.new_ref();
                    let mut stream = chunk.stream(stream_ref, font_stream.encoded_data());
                    font_stream.write_filters(stream.deref_mut());

                    stream_ref
                })
                .collect::<Vec<Ref>>();

        let resource_dictionary = rd_builder.finish();

        let descriptor_ref = if sc.serialize_settings().pdf_version >= PdfVersion::Pdf15 {
            Some(sc.new_ref())
        } else {
            None
        };
        let cmap_ref = sc.new_ref();

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

        let italic_angle = self.font.italic_angle();
        let ascender = font_bbox.bottom();
        let descender = font_bbox.top();

        let mut gids = self
            .glyph_set
            .iter()
            .map(|g| g.glyph_id)
            .collect::<Vec<_>>();
        gids.sort();
        let base_font = base_font_name(&self.font);

        if let Some(descriptor_ref) = descriptor_ref {
            // Write the font descriptor (contains metrics about the font).
            let mut font_descriptor = chunk.font_descriptor(descriptor_ref);
            font_descriptor
                .name(Name(base_font.as_bytes()))
                .flags(flags)
                .bbox(font_bbox.to_pdf_rect())
                .italic_angle(italic_angle)
                .ascent(ascender)
                .descent(descender)
                // Adobe recommends these for tagged PDF for 1.5+ (descriptors for Type3 fonts
                // are only written for 1.5+ in the first place, so no additional checks needed)
                // so we write them as well.
                // Unfortunately we have no way of determining the actual family name, so we just
                // take the next best thing
                .family(Str(base_font.as_bytes()))
                .stretch(
                    match skrifa::attribute::Stretch::new(self.font().stretch()) {
                        skrifa::attribute::Stretch::ULTRA_CONDENSED => {
                            pdf_writer::types::FontStretch::UltraCondensed
                        }
                        skrifa::attribute::Stretch::EXTRA_CONDENSED => {
                            pdf_writer::types::FontStretch::ExtraCondensed
                        }
                        skrifa::attribute::Stretch::CONDENSED => {
                            pdf_writer::types::FontStretch::Condensed
                        }
                        skrifa::attribute::Stretch::SEMI_CONDENSED => {
                            pdf_writer::types::FontStretch::SemiCondensed
                        }
                        skrifa::attribute::Stretch::NORMAL => {
                            pdf_writer::types::FontStretch::Normal
                        }
                        skrifa::attribute::Stretch::SEMI_EXPANDED => {
                            pdf_writer::types::FontStretch::SemiExpanded
                        }
                        skrifa::attribute::Stretch::EXPANDED => {
                            pdf_writer::types::FontStretch::Expanded
                        }
                        skrifa::attribute::Stretch::EXTRA_EXPANDED => {
                            pdf_writer::types::FontStretch::ExtraExpanded
                        }
                        skrifa::attribute::Stretch::ULTRA_EXPANDED => {
                            pdf_writer::types::FontStretch::UltraExpanded
                        }
                        // Fallback
                        _ => pdf_writer::types::FontStretch::Normal,
                    },
                )
                .weight(match self.font.weight() as i32 {
                    150..250 => 200,
                    250..350 => 300,
                    350..450 => 400,
                    450..550 => 500,
                    550..650 => 600,
                    650..750 => 700,
                    750..850 => 800,
                    other => {
                        if other < 150 {
                            100
                        } else {
                            900
                        }
                    }
                });

            font_descriptor.finish();
        }

        let mut type3_font = chunk.type3_font(root_ref);
        resource_dictionary.to_pdf_resources(&mut type3_font);

        // See https://github.com/typst/typst/issues/5067 as to why we write this.
        type3_font.name(Name(base_font.as_bytes()));
        type3_font.bbox(font_bbox.to_pdf_rect());
        type3_font.to_unicode(cmap_ref);
        type3_font.matrix(
            Transform::from_scale(
                1.0 / (self.font.units_per_em()),
                1.0 / (self.font.units_per_em()),
            )
            .to_pdf_transform(),
        );
        type3_font.first_char(0);
        type3_font.last_char(u8::try_from(self.glyphs.len() - 1).unwrap());
        type3_font.widths(self.widths.iter().copied());
        if let Some(descriptor_ref) = descriptor_ref {
            type3_font.font_descriptor(descriptor_ref);
        }

        let mut char_procs = type3_font.char_procs();
        for (gid, ref_) in glyph_streams.iter().enumerate() {
            char_procs.pair(format!("g{gid}").to_pdf_name(), *ref_);
        }
        char_procs.finish();

        let names = (0..self.glyphs.len() as u16)
            .map(|gid| format!("g{gid}"))
            .collect::<Vec<_>>();

        type3_font
            .encoding_custom()
            .differences()
            .consecutive(0, names.iter().map(|n| n.to_pdf_name()));

        type3_font.finish();

        let cmap = {
            let mut cmap = UnicodeCmap::new(CMAP_NAME, SYSTEM_INFO);

            for g in 0..self.glyphs.len() {
                let g = u8::try_from(g).unwrap();
                match self.cmap_entries.get(&g) {
                    None => sc.register_validation_error(ValidationError::InvalidCodepointMapping(
                        self.font.clone(),
                        GlyphId::new(g as u32),
                    )),
                    Some(text) => {
                        // Note: Keep in sync with CIDFont
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
                            ))
                        }

                        if private_unicode {
                            sc.register_validation_error(ValidationError::UnicodePrivateArea(
                                self.font.clone(),
                                GlyphId::new(g as u32),
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

        chunk
    }
}

pub type Type3ID = usize;

// A font can have multiple Type3 fonts, if more than 256 glyphs are used.
// We use this struct to keep track of them.
#[derive(Debug)]
pub(crate) struct Type3FontMapper {
    font: Font,
    fonts: Vec<Type3Font>,
}

impl Type3FontMapper {
    pub fn new(font: Font) -> Type3FontMapper {
        Self {
            font,
            fonts: Vec::new(),
        }
    }
}

impl Type3FontMapper {
    /// Given a requested glyph coverage, find the corresponding font identifier of the
    /// font that contains it.
    pub fn id_from_glyph(&self, glyph: &OwnedCoveredGlyph) -> Option<FontIdentifier> {
        self.fonts
            .iter()
            .position(|f| f.covers(glyph))
            .map(|p| self.fonts[p].identifier())
    }

    /// Given a font identifier, return the corresponding Type3 font if it is part of
    /// that type 3 font mapper.
    pub fn font_from_id(&self, identifier: FontIdentifier) -> Option<&Type3Font> {
        let pos = self
            .fonts
            .iter()
            .position(|f| f.identifier() == identifier)?;
        self.fonts.get(pos)
    }

    pub fn font_mut_from_id(&mut self, identifier: FontIdentifier) -> Option<&mut Type3Font> {
        let pos = self
            .fonts
            .iter()
            .position(|f| f.identifier() == identifier)?;
        self.fonts.get_mut(pos)
    }

    pub fn fonts(&self) -> &[Type3Font] {
        &self.fonts
    }

    pub fn add_glyph(&mut self, glyph: OwnedCoveredGlyph) -> (FontIdentifier, Gid) {
        // If the glyph has already been added, return the font identifier of
        // the type 3 font as well as the Type3 gid in that font.
        if let Some(id) = self.id_from_glyph(&glyph) {
            let gid = self
                .font_from_id(id.clone())
                .unwrap()
                .get_gid(&glyph)
                .unwrap();
            return (id, gid);
        }

        if let Some(last_font) = self.fonts.last_mut() {
            if last_font.is_full() {
                // If the last Type3 font is full, create a new one.
                let mut font = Type3Font::new(self.font.clone(), self.fonts.len());
                let id = font.identifier();
                let gid = font.add_glyph(glyph);
                self.fonts.push(font);
                (id, gid)
            } else {
                // Otherwise, insert it into the last Type3 font.
                let id = last_font.identifier();
                (id, last_font.add_glyph(glyph))
            }
        } else {
            // If not Type3 font exists yet, create a new one.
            let mut font = Type3Font::new(self.font.clone(), self.fonts.len());
            let id = font.identifier();
            let gid = font.add_glyph(glyph);
            self.fonts.push(font);
            (id, gid)
        }
    }
}

pub(crate) fn base_font_name(font: &Font) -> String {
    const REST_LEN: usize = 1 + 1 + IDENTITY_H.len();
    let postscript_name = font.postscript_name().unwrap_or("unknown");

    let max_len = 127 - REST_LEN;

    let trimmed = &postscript_name[..postscript_name.len().min(max_len)];

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use crate::font::Font;

    use crate::object::font::type3_font::OwnedCoveredGlyph;
    use crate::object::font::{FontContainer, OwnedPaintMode};
    use crate::page::Page;
    use crate::path::Fill;
    use crate::serialize::{SerializeSettings, SerializerContext};
    use crate::surface::TextDirection;
    use crate::tests::TWITTER_COLOR_EMOJI;
    use krilla_macros::snapshot;
    use skrifa::GlyphId;
    use tiny_skia_path::Point;

    impl OwnedCoveredGlyph {
        pub fn new(glyph_id: GlyphId, paint_mode: OwnedPaintMode) -> Self {
            Self {
                glyph_id,
                paint_mode,
            }
        }
    }

    #[test]
    fn type3_more_than_256_glyphs() {
        let mut sc = SerializerContext::new(SerializeSettings::settings_1());
        let font = Font::new(TWITTER_COLOR_EMOJI.clone(), 0).unwrap();
        let container = sc.create_or_get_font_container(font.clone());
        let mut font_container = container.borrow_mut();

        match &mut *font_container {
            FontContainer::Type3(t3) => {
                for i in 2..258 {
                    t3.add_glyph(OwnedCoveredGlyph::new(
                        GlyphId::new(i),
                        Fill::default().into(),
                    ));
                }

                assert_eq!(t3.fonts.len(), 1);
                assert_eq!(
                    t3.fonts[0].add_glyph(OwnedCoveredGlyph::new(
                        GlyphId::new(20),
                        Fill::default().into(),
                    )),
                    18
                );

                t3.add_glyph(OwnedCoveredGlyph::new(
                    GlyphId::new(512),
                    Fill::default().into(),
                ));
                assert_eq!(t3.fonts.len(), 2);
            }
            FontContainer::CIDFont(_) => panic!("expected type 3 font"),
        }
    }

    #[snapshot(single_page, settings_1)]
    fn type3_color_glyphs(page: &mut Page) {
        let font = Font::new(TWITTER_COLOR_EMOJI.clone(), 0).unwrap();
        let mut surface = page.surface();

        surface.fill_text(
            Point::from_xy(0.0, 25.0),
            Fill::default(),
            font.clone(),
            25.0,
            &[],
            "😀😃",
            false,
            TextDirection::Auto,
        );
    }

    #[snapshot(single_page, settings_17)]
    fn type3_pdf_14(page: &mut Page) {
        let font = Font::new(TWITTER_COLOR_EMOJI.clone(), 0).unwrap();
        let mut surface = page.surface();

        surface.fill_text(
            Point::from_xy(0.0, 25.0),
            Fill::default(),
            font.clone(),
            25.0,
            &[],
            "😀",
            false,
            TextDirection::Auto,
        );
    }
}
