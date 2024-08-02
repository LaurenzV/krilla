use crate::canvas::CanvasBuilder;
use crate::font::{bitmap, colr, outline, Font, Glyph};
use crate::object::cid_font::find_name;
use crate::object::xobject::XObject;
use crate::resource::{Resource, ResourceDictionaryBuilder, XObjectResource};
use crate::serialize::SerializerContext;
use crate::util::{NameExt, RectExt, TransformExt};
use pdf_writer::types::{FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Chunk, Content, Finish, Name, Ref, Str};
use skrifa::prelude::Size;
use skrifa::{GlyphId, MetadataProvider};
use std::collections::BTreeSet;
use std::sync::Arc;
use tiny_skia_path::{Rect, Transform};

// TODO: Add FontDescriptor, required for Tagged PDF
// TODO: Remove bound on Clone, which (should?) only be needed for cached objects
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Type3Font {
    font: Font,
    glyphs: Vec<GlyphId>,
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
            glyph_set: BTreeSet::new(),
        }
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
            u8::try_from(self.glyphs.len() - 1).unwrap()
        }
    }

    pub fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let widths = self
            .glyphs
            .iter()
            .map(|g| {
                self.font
                    .font_ref()
                    .glyph_metrics(Size::unscaled(), self.font.location_ref())
                    .advance_width(*g)
                    .unwrap_or(0.0)
            })
            .collect::<Vec<_>>();

        let mut rd_builder = ResourceDictionaryBuilder::new();
        let mut bbox = Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap();

        let glyph_streams = self
            .glyphs
            .iter()
            .enumerate()
            .map(|(index, glyph_id)| {
                let mut canvas_builder = CanvasBuilder::new(sc);
                let mut is_outline = false;

                colr::draw_glyph(self.font.clone(), *glyph_id, &mut canvas_builder)
                    // .or_else(|| svg::draw_glyph(&self.font, *glyph_id, &mut stream_builder))
                    .or_else(|| bitmap::draw_glyph(&self.font, *glyph_id, &mut canvas_builder))
                    .or_else(|| {
                        is_outline = true;
                        outline::draw_glyph(&self.font, *glyph_id, &mut canvas_builder)
                    });

                let stream = canvas_builder.finish();
                let mut content = Content::new();

                let stream = if is_outline {
                    let bbox = stream.bbox();
                    content.start_shape_glyph(
                        widths[index],
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
                } else {
                    content.start_color_glyph(widths[index]);
                    let x_object = XObject::new(Arc::new(stream), false, false, None);
                    bbox.expand(&x_object.bbox());
                    let x_name = rd_builder
                        .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
                    content.x_object(x_name.to_pdf_name());

                    content.finish()
                };

                let stream_ref = sc.new_ref();
                sc.chunk_mut().stream(stream_ref, &stream);

                stream_ref
            })
            .collect::<Vec<_>>();

        let resource_dictionary = rd_builder.finish();

        let descriptor_ref = sc.new_ref();
        let cmap_ref = sc.new_ref();

        let mut chunk = Chunk::new();

        let font_ref = self.font.font_ref();
        let postscript_name = find_name(&font_ref);

        let mut flags = FontFlags::empty();
        flags.set(
            FontFlags::SERIF,
            postscript_name
                .clone()
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
            .name(Name(
                postscript_name.unwrap_or("unknown".to_string()).as_bytes(),
            ))
            .flags(flags)
            .bbox(bbox.to_pdf_rect())
            .italic_angle(italic_angle)
            .ascent(ascender)
            .descent(descender);

        font_descriptor.finish();

        let mut type3_font = chunk.type3_font(root_ref);
        resource_dictionary.to_pdf_resources(sc, &mut type3_font.resources());

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
        type3_font.widths(widths);
        type3_font.font_descriptor(descriptor_ref);

        let mut char_procs = type3_font.char_procs();
        for (gid, ref_) in glyph_streams.iter().enumerate() {
            char_procs.pair(format!("g{gid}").to_pdf_name(), *ref_);
        }
        char_procs.finish();

        let names = (0..self.count())
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

        sc.chunk_mut().extend(&chunk);
    }
}

#[cfg(test)]
mod tests {
    use crate::font::{Font, Glyph};
    use crate::object::type3_font::Type3Font;
    use crate::serialize::{SerializeSettings, SerializerContext};
    use skrifa::instance::Location;
    use skrifa::GlyphId;
    use std::rc::Rc;

    #[test]
    fn basic_type3() {
        let data =
            std::fs::read("/Users/lstampfl/Programming/GitHub/krilla/test_glyphs-glyf_colr_1.ttf")
                .unwrap();
        let font = Font::new(Rc::new(data), Location::default()).unwrap();
        let mut type3 = Type3Font::new(font);

        for g in [10, 11, 12] {
            type3.add(&Glyph::new(GlyphId::new(g), "".to_string()));
        }

        let mut sc = SerializerContext::new(SerializeSettings::default());
        let root_ref = sc.new_ref();
        type3.serialize_into(&mut sc, root_ref);

        // No need to write fonts here.

        let chunk = sc.chunk();
        std::fs::write("out.txt", chunk.as_bytes());
    }
}
