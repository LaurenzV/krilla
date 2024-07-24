use crate::canvas::{Canvas, Surface};
use crate::object::mask::Mask;
use crate::svg::group;
use crate::svg::util::convert_mask_type;
use crate::FillRule;
use pdf_writer::Finish;
use std::sync::Arc;
use tiny_skia_path::{PathBuilder, Size};

pub fn get_mask(mask: &usvg::Mask) -> Mask {
    // Dummy size. TODO: Improve?
    let mut canvas = Canvas::new(Size::from_wh(1.0, 1.0).unwrap());

    {
        let masked: &mut dyn Surface = if let Some(mask) = mask.mask() {
            &mut canvas.masked(get_mask(mask))
        } else {
            &mut canvas
        };

        let clip_path = {
            let mut path_builder = PathBuilder::new();
            let rect = mask.rect();
            path_builder.move_to(rect.left(), rect.top());
            path_builder.line_to(rect.right(), rect.top());
            path_builder.line_to(rect.right(), rect.bottom());
            path_builder.line_to(rect.left(), rect.bottom());
            path_builder.close();
            path_builder.finish().unwrap()
        };

        let mut clipped = masked.clipped(clip_path, FillRule::NonZero);
        group::render(mask.root(), &mut clipped);
        clipped.finish();
        masked.finish();
    }

    Mask::new(
        Arc::new(canvas.byte_code.clone()),
        convert_mask_type(&mask.kind()),
    )
}
