use crate::font::Font;
use crate::serialize::{Object, SerializerContext};
use crate::util::TransformExt;
use pdf_writer::Ref;
use skrifa::GlyphId;
use tiny_skia_path::Transform;
// TODO: Add FontDescriptor, required for Tagged PDF
// TODO: Remove bound on Clone, which (should?) only be needed for cached objects
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Type3Font {
    font: Font,
    glyphs: Vec<GlyphId>,
}

impl Object for Type3Font {
    fn serialize_into(self, sc: &mut SerializerContext, root_ref: Ref) {
        let mut type3_font = sc.chunk_mut().type3_font(root_ref);
        let bbox = self.font.bbox();
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
        todo!()
    }

    fn is_cached(&self) -> bool {
        // In comparison to other PDF objects in krilla, fonts are actually mutated during the
        // serialization process, so we can't leverage the normal caching process. Instead, caching
        // is being taken care of separately, so we don't need it here.
        false
    }
}
