use crate::error::KrillaResult;
use crate::font::outline::glyph_path;
use crate::font::{Font, FontIdentifier, OwnedPaintMode, PaintMode, Type3Identifier};
use crate::object::xobject::XObject;
use crate::path::Fill;
use crate::resource::{Resource, ResourceDictionaryBuilder};
use crate::serialize::{FilterStream, SerializerContext};
use crate::stream::StreamBuilder;
use crate::util::{NameExt, RectExt, TransformExt};
use crate::{font, SvgSettings};
use pdf_writer::types::{FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Chunk, Content, Finish, Name, Ref, Str};
use skrifa::GlyphId;
use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::DerefMut;
use tiny_skia_path::{PathStroker, Rect, Transform};

pub type Gid = u8;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CoveredGlyph<'a> {
    pub glyph_id: GlyphId,
    pub paint_mode: PaintMode<'a>,
    pub font_size: f32,
}

impl CoveredGlyph<'_> {
    pub fn to_owned(&self) -> OwnedCoveredGlyph {
        OwnedCoveredGlyph {
            glyph_id: self.glyph_id,
            paint_mode: self.paint_mode.to_owned(),
            font_size: self.font_size,
        }
    }
}

impl<'a> CoveredGlyph<'a> {
    pub fn new(
        glyph_id: GlyphId,
        paint_mode: PaintMode<'a>,
        mut font_size: f32,
    ) -> CoveredGlyph<'a> {
        if matches!(paint_mode, PaintMode::Fill(_)) {
            // The only reason we need the font size is that
            // when drawing a stroked glyph as a Type3 glyph, we stroke
            // it using tiny-skia and then draw it as a filled glyph instead.
            // This is because stroking pretty much doesn't work with type 3 fonts.
            // For fills, we don't care about the font size, so we always set it to one.
            // so that we don't allocate new glyphs for each font size it is used at.
            font_size = 1.0;
        }

        Self {
            glyph_id,
            paint_mode,
            font_size,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OwnedCoveredGlyph {
    glyph_id: GlyphId,
    paint_mode: OwnedPaintMode,
    font_size: f32,
}

impl Eq for OwnedCoveredGlyph {}

impl Hash for OwnedCoveredGlyph {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.glyph_id.hash(state);
        self.paint_mode.hash(state);
        self.font_size.to_bits().hash(state);
    }
}

// TODO: Add FontDescriptor, required for Tagged PDF
#[derive(Debug)]
pub(crate) struct Type3Font {
    font: Font,
    glyphs: Vec<OwnedCoveredGlyph>,
    widths: Vec<f32>,
    cmap_entries: BTreeMap<Gid, String>,
    glyph_set: HashSet<OwnedCoveredGlyph>,
    index: usize,
}

const CMAP_NAME: Name = Name(b"Custom");
const SYSTEM_INFO: SystemInfo = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

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

    pub fn unit_per_em(&self) -> f32 {
        self.font.units_per_em()
    }

    pub fn is_full(&self) -> bool {
        self.count() == 256
    }

    pub fn count(&self) -> u16 {
        u16::try_from(self.glyphs.len()).unwrap()
    }

    // TODO: Can we used COveredGlyphRef instead?
    pub fn covers(&self, glyph: &OwnedCoveredGlyph) -> bool {
        self.glyph_set.contains(glyph)
    }

    pub fn get_gid(&self, glyph: &OwnedCoveredGlyph) -> Option<u8> {
        self.glyphs
            .iter()
            .position(|g| g == glyph)
            .and_then(|n| u8::try_from(n).ok())
    }

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

    pub fn get_codepoints(&self, gid: Gid) -> Option<&str> {
        self.cmap_entries.get(&gid).map(|s| s.as_str())
    }

    pub fn set_codepoints(&mut self, gid: Gid, text: String) {
        self.cmap_entries.insert(gid, text);
    }

    pub fn font(&self) -> Font {
        self.font.clone()
    }

    pub fn identifier(&self) -> FontIdentifier {
        FontIdentifier::Type3(Type3Identifier(self.font.clone(), self.index))
    }

    pub(crate) fn serialize(
        &self,
        sc: &mut SerializerContext,
        root_ref: Ref,
    ) -> KrillaResult<Chunk> {
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

                    let drawn_color_glyph = font::draw_color_glyph(
                        self.font.clone(),
                        SvgSettings::default(),
                        glyph.glyph_id,
                        glyph.paint_mode.as_ref(),
                        Transform::default(),
                        &mut surface,
                    );

                    let stream =
                        if drawn_color_glyph.is_some() {
                            surface.finish();
                            let stream = stream_surface.finish();
                            let mut content = Content::new();

                            // I considered writing into the stream directly instead of creating an XObject
                            // and showing that, but it seems like many viewers don't like that, and emojis
                            // look messed up. Using XObjects seems like the best choice here.
                            content.start_color_glyph(self.widths[index]);
                            let x_object = XObject::new(stream, false, false, None);
                            font_bbox.expand(&x_object.bbox());
                            let x_name = rd_builder.register_resource(x_object, sc);
                            content.x_object(x_name.to_pdf_name());

                            content.finish()
                        } else {
                            // If this is the case (i.e. no glyph was drawn, either because no table
                            // exists or an error occurred, the surface is guaranteed to be empty.
                            // So we can just safely draw the outline glyph instead.
                            if let Some(path) = glyph_path(self.font.clone(), glyph.glyph_id)
                                .and_then(|p| match &glyph.paint_mode {
                                    OwnedPaintMode::Fill(_) => Some(p),
                                    OwnedPaintMode::Stroke(s) => {
                                        let mut stroker = PathStroker::new();
                                        let stroke_dash = s.dash.clone().and_then(|s| {
                                            tiny_skia_path::StrokeDash::new(s.array, s.offset)
                                        });

                                        let stroke = tiny_skia_path::Stroke {
                                            width: (s.width / glyph.font_size)
                                                * self.font.units_per_em(),
                                            miter_limit: s.miter_limit,
                                            line_cap: s.line_cap.into(),
                                            line_join: s.line_join.into(),
                                            dash: stroke_dash,
                                        };

                                        stroker.stroke(&p, &stroke, 1.0)
                                    }
                                })
                            {
                                // Just use a dummy fill. The Type3 glyph description is a shape glyph
                                // so it doesn't contain any fill. Instead, it will be taken from
                                // context where it is drawn.
                                surface.fill_path_impl(&path, Fill::default(), false);
                            }

                            surface.finish();
                            let stream = stream_surface.finish();
                            let mut content = Content::new();

                            // Use shape glyph for outline-based Type3 fonts.
                            let bbox = stream.bbox();
                            font_bbox.expand(&bbox);
                            content.start_shape_glyph(
                                self.widths[index],
                                bbox.left(),
                                bbox.top(),
                                bbox.right(),
                                bbox.bottom(),
                            );

                            // TODO: Find a type-safe way of doing this.
                            let mut final_stream = content.finish();
                            final_stream.push(b'\n');
                            final_stream.extend(stream.content());
                            final_stream
                        };

                    let font_stream =
                        FilterStream::new_from_content_stream(&stream, &sc.serialize_settings);

                    let stream_ref = sc.new_ref();
                    let mut stream = chunk.stream(stream_ref, font_stream.encoded_data());
                    font_stream.write_filters(stream.deref_mut());

                    Ok(stream_ref)
                })
                .collect::<KrillaResult<Vec<Ref>>>()?;

        let resource_dictionary = rd_builder.finish();

        let descriptor_ref = sc.new_ref();
        let cmap_ref = sc.new_ref();

        let postscript_name = self.font.postscript_name();

        let mut flags = FontFlags::empty();
        flags.set(
            FontFlags::SERIF,
            postscript_name
                .map(|n| n.contains("Serif"))
                .unwrap_or(false),
        );
        flags.set(FontFlags::FIXED_PITCH, self.font.is_monospaced());
        flags.set(FontFlags::ITALIC, self.font.italic_angle() != 0.0);
        flags.insert(FontFlags::SYMBOLIC);
        flags.insert(FontFlags::SMALL_CAP);

        let italic_angle = self.font.italic_angle();
        let ascender = font_bbox.bottom();
        let descender = font_bbox.top();

        // Write the font descriptor (contains metrics about the font).
        let mut font_descriptor = chunk.font_descriptor(descriptor_ref);
        font_descriptor
            .name(Name(postscript_name.unwrap_or("unknown").as_bytes()))
            .flags(flags)
            .bbox(font_bbox.to_pdf_rect())
            .italic_angle(italic_angle)
            .ascent(ascender)
            .descent(descender);

        font_descriptor.finish();

        let mut type3_font = chunk.type3_font(root_ref);
        resource_dictionary.to_pdf_resources(&mut type3_font)?;

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
        type3_font.font_descriptor(descriptor_ref);

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
            for (g, text) in &self.cmap_entries {
                if !text.is_empty() {
                    cmap.pair_with_multiple(*g, text.chars());
                }
            }

            cmap
        };
        chunk.cmap(cmap_ref, &cmap.finish());

        Ok(chunk)
    }
}

pub type Type3ID = usize;

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
    pub fn id_from_glyph(&self, glyph: &OwnedCoveredGlyph) -> Option<FontIdentifier> {
        self.fonts
            .iter()
            .position(|f| f.covers(glyph))
            .map(|p| self.fonts[p].identifier())
    }

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

#[cfg(test)]
mod tests {
    use crate::font::{Font, FontIdentifier, OwnedPaintMode, Type3Identifier};

    use crate::color::rgb;

    use crate::object::type3_font::OwnedCoveredGlyph;
    use crate::path::Fill;
    use crate::serialize::{FontContainer, SerializeSettings, SerializerContext};
    use crate::surface::Surface;
    use crate::tests::{
        red_fill, LATIN_MODERN_ROMAN, NOTO_SANS, NOTO_SANS_ARABIC, NOTO_SANS_VARIABLE,
    };
    use krilla_macros::{snapshot, visreg};
    use skrifa::GlyphId;
    use tiny_skia_path::Point;

    impl OwnedCoveredGlyph {
        pub fn new(glyph_id: GlyphId, paint_mode: OwnedPaintMode, font_size: f32) -> Self {
            Self {
                glyph_id,
                paint_mode,
                font_size,
            }
        }
    }

    #[snapshot(settings_4)]
    fn type3_noto_sans_two_glyphs(sc: &mut SerializerContext) {
        let font = Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap();
        let container = sc.create_or_get_font_container(font.clone());
        let mut font_container = container.borrow_mut();

        match &mut *font_container {
            FontContainer::Type3(t3) => {
                t3.add_glyph(OwnedCoveredGlyph::new(
                    GlyphId::new(36),
                    Fill::default().into(),
                    1.0,
                ));
                t3.add_glyph(OwnedCoveredGlyph::new(
                    GlyphId::new(37),
                    Fill::default().into(),
                    1.0,
                ));
                let t3_font = t3
                    .font_mut_from_id(FontIdentifier::Type3(Type3Identifier(font.clone(), 0)))
                    .unwrap();
                t3_font.set_codepoints(1, "A".to_string());
                t3_font.set_codepoints(2, "B".to_string());
            }
            FontContainer::CIDFont(_) => panic!("expected type 3 font"),
        }
    }

    #[visreg(all, settings_4)]
    fn type3_noto_sans_simple_text(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            32.0,
            &[],
            "hello world",
            false,
            None,
        );
    }

    #[visreg(all, settings_4)]
    fn type3_latin_modern_simple_text(surface: &mut Surface) {
        let font = Font::new(LATIN_MODERN_ROMAN.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            32.0,
            &[],
            "hello world",
            false,
            None,
        );
    }

    #[visreg(all, settings_4)]
    fn type3_with_color(surface: &mut Surface) {
        let font = Font::new(LATIN_MODERN_ROMAN.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            red_fill(0.8),
            font,
            32.0,
            &[],
            "hello world",
            false,
            None,
        );
    }

    #[visreg]
    fn variable_font(surface: &mut Surface) {
        let font1 = Font::new(
            NOTO_SANS_VARIABLE.clone(),
            0,
            vec![("wght".to_string(), 100.0), ("wdth".to_string(), 62.5)],
        )
        .unwrap();
        let font2 = Font::new(
            NOTO_SANS_VARIABLE.clone(),
            0,
            vec![("wght".to_string(), 900.0), ("wdth".to_string(), 100.0)],
        )
        .unwrap();

        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill {
                paint: rgb::Color::black().into(),
                ..Default::default()
            },
            font1.clone(),
            20.0,
            &[],
            "Variable fonts rock!",
            false,
            None,
        );

        surface.fill_text(
            Point::from_xy(0.0, 120.0),
            Fill {
                paint: rgb::Color::black().into(),
                ..Default::default()
            },
            font2.clone(),
            20.0,
            &[],
            "Variable fonts rock!",
            false,
            None,
        );
    }

    #[visreg(all, settings_4)]
    fn type3_noto_arabic_simple_text(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS_ARABIC.clone(), 0, vec![]).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            32.0,
            &[],
            "مرحبا بالعالم",
            false,
            None,
        );
    }

    #[snapshot(settings_4)]
    fn type3_latin_modern_four_glyphs(sc: &mut SerializerContext) {
        let font = Font::new(LATIN_MODERN_ROMAN.clone(), 0, vec![]).unwrap();
        let container = sc.create_or_get_font_container(font.clone());
        let mut font_container = container.borrow_mut();

        match &mut *font_container {
            FontContainer::Type3(t3) => {
                t3.add_glyph(OwnedCoveredGlyph::new(
                    GlyphId::new(58),
                    Fill::default().into(),
                    1.0,
                ));
                t3.add_glyph(OwnedCoveredGlyph::new(
                    GlyphId::new(54),
                    Fill::default().into(),
                    1.0,
                ));
                t3.add_glyph(OwnedCoveredGlyph::new(
                    GlyphId::new(69),
                    Fill::default().into(),
                    1.0,
                ));
                t3.add_glyph(OwnedCoveredGlyph::new(
                    GlyphId::new(71),
                    Fill::default().into(),
                    1.0,
                ));
                let t3_font = t3
                    .font_mut_from_id(FontIdentifier::Type3(Type3Identifier(font.clone(), 0)))
                    .unwrap();
                t3_font.set_codepoints(1, "G".to_string());
                t3_font.set_codepoints(2, "F".to_string());
                t3_font.set_codepoints(3, "K".to_string());
                t3_font.set_codepoints(4, "L".to_string());
            }
            FontContainer::CIDFont(_) => panic!("expected type 3 font"),
        }
    }

    #[test]
    fn type3_more_than_256_glyphs() {
        let mut sc = SerializerContext::new(SerializeSettings::settings_4());
        let font = Font::new(NOTO_SANS.clone(), 0, vec![]).unwrap();
        let container = sc.create_or_get_font_container(font.clone());
        let mut font_container = container.borrow_mut();

        match &mut *font_container {
            FontContainer::Type3(t3) => {
                for i in 2..258 {
                    t3.add_glyph(OwnedCoveredGlyph::new(
                        GlyphId::new(i),
                        Fill::default().into(),
                        1.0,
                    ));
                }

                assert_eq!(t3.fonts.len(), 1);
                assert_eq!(
                    t3.fonts[0].add_glyph(OwnedCoveredGlyph::new(
                        GlyphId::new(20),
                        Fill::default().into(),
                        1.0
                    )),
                    18
                );

                t3.add_glyph(OwnedCoveredGlyph::new(
                    GlyphId::new(512),
                    Fill::default().into(),
                    1.0,
                ));
                assert_eq!(t3.fonts.len(), 2);
            }
            FontContainer::CIDFont(_) => panic!("expected type 3 font"),
        }
    }
}
