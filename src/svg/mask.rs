use crate::object::mask::Mask;
use crate::surface::StreamBuilder;
use crate::svg::util::convert_mask_type;
use crate::svg::{group, ProcessContext};
use crate::util::RectExt;
use crate::FillRule;

pub fn render(
    mask: &usvg::Mask,
    mut stream_builder: StreamBuilder,
    process_context: &mut ProcessContext,
) -> Mask {
    let mut surface = stream_builder.surface();
    let mut pop_count = 0;
    if let Some(sub_usvg_mask) = mask.mask() {
        let sub_mask = render(sub_usvg_mask, surface.stream_surface(), process_context);
        surface.push_mask(sub_mask);
        pop_count += 1;
    }

    let clip_path = mask.rect().to_rect().to_clip_path();
    surface.push_clip_path(&clip_path, &FillRule::NonZero);
    pop_count += 1;
    group::render(mask.root(), &mut surface, process_context);

    for _ in 0..pop_count {
        surface.pop();
    }

    surface.finish();
    let stream = stream_builder.finish();

    Mask::new(stream, convert_mask_type(&mask.kind()))
}
