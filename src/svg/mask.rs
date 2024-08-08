use crate::object::mask::Mask;
use crate::surface::{StreamBuilder, Surface};
use crate::svg::util::convert_mask_type;
use crate::svg::{group, FontContext};
use crate::util::RectExt;
use crate::FillRule;
use std::sync::Arc;

pub fn get_mask(
    mask: &usvg::Mask,
    mut canvas_builder: StreamBuilder,
    font_context: &mut FontContext,
) -> Mask {
    let mut surface = canvas_builder.surface();
    if let Some(sub_usvg_mask) = mask.mask() {
        let sub_mask = get_mask(sub_usvg_mask, surface.stream_surface(), font_context);
        surface.push_mask(sub_mask);
    }

    let clip_path = mask.rect().to_rect().to_clip_path();
    surface.push_clip_path(&clip_path, &FillRule::NonZero);
    group::render(mask.root(), &mut surface, font_context);
    surface.pop_clip_path();

    if mask.mask().is_some() {
        surface.pop_mask();
    }

    surface.finish();
    let stream = canvas_builder.finish();

    Mask::new(stream, convert_mask_type(&mask.kind()))
}
