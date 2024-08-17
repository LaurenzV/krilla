use crate::font;
use crate::font::{Font, Glyph, GlyphType};
use crate::object::xobject::XObject;
use crate::resource::{Resource, ResourceDictionaryBuilder, XObjectResource};
use crate::serialize::{Object, SerializerContext};
use crate::surface::StreamBuilder;
use crate::util::{NameExt, RectExt, TransformExt};
use pdf_writer::types::{FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Chunk, Content, Finish, Name, Ref, Str};
use skrifa::GlyphId;
use std::collections::BTreeSet;
use tiny_skia_path::{Rect, Transform};

// TODO: Add FontDescriptor, required for Tagged PDF
// TODO: Remove bound on Clone, which (should?) only be needed for cached objects
#[derive(Debug)]
pub struct Type3Font {
    font: Font,
    glyphs: Vec<GlyphId>,
    widths: Vec<f32>,
    strings: Vec<String>,
    glyph_set: BTreeSet<GlyphId>,
}

const CMAP_NAME: Name = Name(b"Custom");
const SYSTEM_INFO: SystemInfo = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

impl Type3Font {
    pub fn new(font: Font) -> Self {
        Self {
            font,
            glyphs: Vec::new(),
            strings: Vec::new(),
            widths: Vec::new(),
            glyph_set: BTreeSet::new(),
        }
    }

    pub fn to_font_units(&self, val: f32) -> f32 {
        val
    }

    pub fn is_full(&self) -> bool {
        self.count() == 256
    }

    pub fn count(&self) -> u16 {
        u16::try_from(self.glyphs.len()).unwrap()
    }

    pub fn covers(&self, glyph: GlyphId) -> bool {
        self.glyph_set.contains(&glyph)
    }

    pub fn add(&mut self, glyph: &Glyph) -> u8 {
        if let Some(pos) = self
            .glyphs
            .iter()
            .position(|g| *g == glyph.glyph_id)
            .and_then(|n| u8::try_from(n).ok())
        {
            self.strings[pos as usize] = glyph.string.clone();
            return pos;
        } else {
            assert!(self.glyphs.len() < 256);

            self.glyphs.push(glyph.glyph_id);
            self.strings.push(glyph.string.clone());
            self.widths
                .push(self.font.advance_width(glyph.glyph_id).unwrap_or(0.0));
            u8::try_from(self.glyphs.len() - 1).unwrap()
        }
    }

    pub fn units_per_em(&self) -> u16 {
        self.font.units_per_em()
    }

    pub fn advance_width(&self, glyph_id: u8) -> Option<f32> {
        self.widths
            .get(glyph_id as usize)
            .copied()
            .map(|n| self.to_font_units(n))
    }
}

impl Object for Type3Font {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();
        let svg_settings = sc.serialize_settings.svg_settings;

        let mut rd_builder = ResourceDictionaryBuilder::new();
        let mut bbox = Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap();

        let glyph_streams = self
            .glyphs
            .iter()
            .enumerate()
            .map(|(index, glyph_id)| {
                let mut stream_surface = StreamBuilder::new(sc);
                let mut surface = stream_surface.surface();
                let glyph_type =
                    font::draw_glyph(self.font.clone(), svg_settings, *glyph_id, &mut surface);
                surface.finish();
                let stream = stream_surface.finish();

                let mut content = Content::new();

                let stream = if glyph_type == Some(GlyphType::Outline) {
                    let bbox = stream.bbox;
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
                    final_stream.extend(stream.content);
                    final_stream
                } else {
                    // I considered writing into the stream directly instead of creating an XObject
                    // and showing that, but it seems like many viewers don't like that, and emojis
                    // look messed up. Using XObjects seems like the best choice here.
                    content.start_color_glyph(self.widths[index]);
                    let x_object = XObject::new(stream, false, false, None);
                    bbox.expand(&x_object.bbox());
                    let x_name = rd_builder
                        .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
                    content.x_object(x_name.to_pdf_name());

                    content.finish()
                };

                let (stream, filter) = sc.get_binary_stream(&stream);

                let stream_ref = sc.new_ref();
                chunk.stream(stream_ref, &stream).filter(filter);

                stream_ref
            })
            .collect::<Vec<_>>();

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
        let ascender = bbox.bottom();
        let descender = bbox.top();

        // Write the font descriptor (contains metrics about the font).
        let mut font_descriptor = chunk.font_descriptor(descriptor_ref);
        font_descriptor
            .name(Name(postscript_name.unwrap_or("unknown").as_bytes()))
            .flags(flags)
            .bbox(bbox.to_pdf_rect())
            .italic_angle(italic_angle)
            .ascent(ascender)
            .descent(descender);

        font_descriptor.finish();

        let mut type3_font = chunk.type3_font(root_ref);
        resource_dictionary.to_pdf_resources(sc, &mut type3_font);

        type3_font.bbox(bbox.to_pdf_rect());
        type3_font.to_unicode(cmap_ref);
        type3_font.matrix(
            Transform::from_scale(
                1.0 / (self.font.units_per_em() as f32),
                1.0 / (self.font.units_per_em() as f32),
            )
            .to_pdf_transform(),
        );
        type3_font.first_char(0);
        type3_font.last_char(u8::try_from(self.glyphs.len() - 1).unwrap());
        type3_font.widths(self.widths);
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
            for (g, text) in self.strings.iter().enumerate() {
                if !text.is_empty() {
                    cmap.pair_with_multiple(g as u8, text.chars());
                }
            }

            cmap
        };
        chunk.cmap(cmap_ref, &cmap.finish());

        chunk
    }
}
