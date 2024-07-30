use crate::canvas::Canvas;
use crate::font::{bitmap, colr, outline, svg, Font};
use crate::object::xobject::XObject;
use crate::resource::{Resource, ResourceDictionaryBuilder, XObjectResource};
use crate::serialize::{Object, SerializerContext};
use crate::util::{NameExt, RectExt, TransformExt};
use pdf_writer::{Chunk, Content, Finish, Ref};
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
    glyph_canvases: Vec<Canvas>,
    glyph_set: BTreeSet<GlyphId>,
}

impl Type3Font {
    pub fn new(font: Font) -> Self {
        Self {
            font,
            glyphs: Vec::new(),
            glyph_canvases: Vec::new(),
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

    pub fn add(&mut self, glyph_id: GlyphId) -> u8 {
        if let Some(pos) = self
            .glyphs
            .iter()
            .position(|g| *g == glyph_id)
            .and_then(|n| u8::try_from(n).ok())
        {
            return pos;
        } else {
            assert!(self.glyphs.len() < 256);

            self.glyphs.push(glyph_id);
            self.glyph_canvases.push(
                colr::draw_glyph(&self.font, glyph_id)
                    .or_else(|| svg::draw_glyph(&self.font, glyph_id))
                    .or_else(|| bitmap::draw_glyph(&self.font, glyph_id))
                    .or_else(|| outline::draw_glyph(&self.font, glyph_id))
                    .unwrap(),
            );
            u8::try_from(self.glyphs.len() - 1).unwrap()
        }
    }
}

impl Object for Type3Font {
    fn serialize_into(mut self, sc: &mut SerializerContext, root_ref: Ref) {
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

        let mut resource_dictionary = ResourceDictionaryBuilder::new();
        let mut global_bbox = Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap();

        let glyph_streams = self
            .glyphs
            .iter()
            .enumerate()
            .map(|(index, glyph)| {
                let canvas = std::mem::take(&mut self.glyph_canvases[index]);

                let mut content = Content::new();
                content.start_color_glyph(widths[index]);

                let x_object = XObject::new(Arc::new(canvas.byte_code), false, false, None);
                let x_name = resource_dictionary
                    .register_resource(Resource::XObject(XObjectResource::XObject(x_object)));
                content.x_object(x_name.to_pdf_name());

                let stream = content.finish();

                let stream_ref = sc.new_ref();
                sc.chunk_mut().stream(stream_ref, &stream);

                stream_ref
            })
            .collect::<Vec<_>>();

        let mut chunk = Chunk::new();
        let mut type3_font = chunk.type3_font(root_ref);
        resource_dictionary.to_pdf_resources(sc, &mut type3_font.resources());

        type3_font.bbox(global_bbox.to_pdf_rect());
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
        sc.chunk_mut().extend(&chunk);
    }
}

#[cfg(test)]
mod tests {
    use crate::font::Font;
    use crate::object::type3_font::Type3Font;
    use crate::serialize::{Object, SerializeSettings, SerializerContext};
    use skrifa::instance::Location;
    use skrifa::GlyphId;
    use std::sync::Arc;

    #[test]
    fn basic_type3() {
        let data =
            std::fs::read("/Users/lstampfl/Programming/GitHub/krilla/test_glyphs-glyf_colr_1.ttf")
                .unwrap();
        let font = Font::new(Arc::new(data), Location::default()).unwrap();
        let mut type3 = Type3Font::new(font);

        for g in [10, 11, 12] {
            type3.add(GlyphId::new(g));
        }

        let mut serializer_context = SerializerContext::new(SerializeSettings::default());
        let root_ref = serializer_context.new_ref();
        type3.serialize_into(&mut serializer_context, root_ref);

        // No need to write fonts here.

        let chunk = serializer_context.chunk();
        std::fs::write("out.txt", chunk.as_bytes());
    }
}
