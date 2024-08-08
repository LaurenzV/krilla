use crate::object::mask::Mask;
use crate::surface::{StreamSurface, Surface};
use crate::svg::util::convert_mask_type;
use crate::svg::{group, FontContext};
use crate::util::RectExt;
use crate::FillRule;
use std::sync::Arc;

pub fn get_mask(
    mask: &usvg::Mask,
    mut canvas_builder: StreamSurface,
    font_context: &mut FontContext,
) -> Mask {
    if let Some(sub_usvg_mask) = mask.mask() {
        let sub_mask = get_mask(sub_usvg_mask, canvas_builder.stream_surface(), font_context);
        canvas_builder.push_mask(sub_mask);
    }

    let clip_path = mask.rect().to_rect().to_clip_path();
    canvas_builder.push_clip_path(&clip_path, &FillRule::NonZero);
    group::render(mask.root(), &mut canvas_builder, font_context);
    canvas_builder.pop_clip_path();

    if mask.mask().is_some() {
        canvas_builder.pop_mask();
    }

    let stream = canvas_builder.finish();

    Mask::new(Arc::new(stream), convert_mask_type(&mask.kind()))
}
