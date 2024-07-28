use crate::font::Font;
use crate::serialize::{Object, SerializerContext};
use crate::util::{NameExt, TransformExt};
use pdf_writer::{Finish, Ref};
use skrifa::prelude::Size;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::Transform;
// TODO: Add FontDescriptor, required for Tagged PDF
// TODO: Remove bound on Clone, which (should?) only be needed for cached objects
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Type3Font {
    font: Font,
    glyphs: Vec<GlyphId>,
}

impl Type3Font {
    pub fn new(font: Font) -> Self {
        Self {
            font,
            glyphs: Vec::new(),
        }
    }

    pub fn is_full(&self) -> bool {
        self.count() == 256
    }

    pub fn count(&self) -> u16 {
        u16::try_from(self.glyphs.len()).unwrap()
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
            u8::try_from(self.glyphs.len() - 1).unwrap()
        }
    }
}

impl Object for Type3Font {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let mut type3_font = sc.chunk_mut().type3_font(root_ref);
        let bbox = self.font.bbox();
        let widths = self.glyphs.iter().map(|g| {
            self.font
                .font_ref()
                .glyph_metrics(Size::unscaled(), self.font.location_ref())
                .advance_width(*g)
                .unwrap_or(0.0)
        });

        type3_font.bbox(pdf_writer::Rect::new(
            bbox.left(),
            bbox.top(),
            bbox.right(),
            bbox.bottom(),
        ));
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

        let names = (0..self.count())
            .map(|gid| format!("g{gid}"))
            .collect::<Vec<_>>();

        type3_font
            .encoding_custom()
            .differences()
            .consecutive(0, names.iter().map(|n| n.to_pdf_name()));

        type3_font.finish();
        // todo!()
    }

    fn is_cached(&self) -> bool {
        // In comparison to other PDF objects in krilla, fonts are actually mutated during the
        // serialization process, so we can't leverage the normal caching process. Instead, caching
        // is being taken care of separately, so we don't need it here.
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::font::Font;
    use crate::object::type3_font::Type3Font;
    use crate::serialize::{Object, SerializeSettings, SerializerContext};
    use pdf_writer::Ref;
    use skrifa::instance::Location;
    use skrifa::GlyphId;
    use std::sync::Arc;

    #[test]
    fn basic_type3() {
        let data = std::fs::read("/Library/Fonts/NotoColorEmoji-Regular.ttf").unwrap();
        let font = Font::new(Arc::new(data), Location::default()).unwrap();
        let mut type3 = Type3Font::new(font);

        for g in [2397, 2400, 2401, 2398, 2403, 2402, 2399, 3616] {
            type3.add(GlyphId::new(g));
        }

        let mut serializer_context = SerializerContext::new(SerializeSettings::default());
        type3.serialize_into(&mut serializer_context, Ref::new(1));

        let chunk = serializer_context.chunk();
        std::fs::write("out.txt", chunk.as_bytes());
    }
}
