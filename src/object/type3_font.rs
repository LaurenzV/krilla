use crate::font::Font;
use crate::serialize::{Object, SerializerContext};
use crate::util::TransformExt;
use pdf_writer::Ref;
use skrifa::prelude::Size;
use skrifa::{GlyphId, MetadataProvider};
use std::collections::BTreeSet;
use tiny_skia_path::{FiniteF32, Transform};
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
        self.glyphs.len() == 256
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
        todo!()
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
    use skrifa::instance::Location;
    use std::sync::Arc;

    fn basic_type3() {
        let data = std::fs::read("/Library/Fonts/NotoColorEmoji-Regular.ttf").unwrap();
        let font = Font::new(Arc::new(data), Location::default()).unwrap();
        let mut type3 = Type3Font::new(font);
    }
}
