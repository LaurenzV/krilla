use crate::font::{bitmap, colr, outline, svg, Font, Glyph};
use crate::object::xobject::XObject;
use crate::resource::{Resource, ResourceDictionaryBuilder, XObjectResource};
use crate::serialize::{Object, SerializerContext};
use crate::stream::StreamBuilder;
use crate::util::{NameExt, RectExt, TransformExt};
use pdf_writer::types::{SystemInfo, UnicodeCmap};
use pdf_writer::{Chunk, Content, Finish, Name, Ref, Str};
use skrifa::prelude::Size;
use skrifa::{GlyphId, MetadataProvider};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
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

    pub fn serialize_into(mut self, sc: Rc<RefCell<SerializerContext>>, root_ref: Ref) {
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
        let mut global_bbox = self.font.bbox();

        let glyph_streams = self
            .glyphs
            .iter()
            .enumerate()
            .map(|(index, glyph_id)| {
                let mut stream_builder = StreamBuilder::new(sc.clone());
                colr::draw_glyph(&self.font, *glyph_id, &mut stream_builder)
                    .or_else(|| svg::draw_glyph(&self.font, *glyph_id, &mut stream_builder))
                    .or_else(|| bitmap::draw_glyph(&self.font, *glyph_id, &mut stream_builder))
                    .or_else(|| outline::draw_glyph(&self.font, *glyph_id, &mut stream_builder));

                let stream = stream_builder.finish();

                let mut content = Content::new();
                content.start_color_glyph(widths[index]);

                let x_object = XObject::new(Arc::new(stream), false, false, None);
                let x_name = rd_builder
                    .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
                content.x_object(x_name.to_pdf_name());

                let stream = content.finish();

                let stream_ref = sc.borrow_mut().new_ref();
                sc.borrow_mut().chunk_mut().stream(stream_ref, &stream);

                stream_ref
            })
            .collect::<Vec<_>>();

        let resource_dictionary = rd_builder.finish();

        let cmap_ref = sc.borrow_mut().new_ref();

        let mut chunk = Chunk::new();
        let mut type3_font = chunk.type3_font(root_ref);
        resource_dictionary.to_pdf_resources(&mut sc.borrow_mut(), &mut type3_font.resources());

        type3_font.bbox(global_bbox.to_pdf_rect());
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

        sc.borrow_mut().chunk_mut().extend(&chunk);
    }
}

#[cfg(test)]
mod tests {
    use crate::font::{Font, Glyph};
    use crate::object::type3_font::Type3Font;
    use crate::serialize::{Object, SerializeSettings, SerializerContext};
    use skrifa::instance::Location;
    use skrifa::GlyphId;
    use std::cell::RefCell;
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

        let mut serializer_context = Rc::new(RefCell::new(SerializerContext::new(
            SerializeSettings::default(),
        )));
        let root_ref = serializer_context.borrow_mut().new_ref();
        type3.serialize_into(serializer_context.clone(), root_ref);

        // No need to write fonts here.

        let borrowed = serializer_context.borrow();
        let chunk = borrowed.chunk();
        std::fs::write("out.txt", chunk.as_bytes());
    }
}
