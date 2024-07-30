use crate::object::mask::Mask;
use crate::serialize::{SerializeSettings, SerializerContext};
use crate::stream::StreamBuilder;
use crate::svg::group;
use crate::svg::util::convert_mask_type;
use crate::util::RectExt;
use crate::FillRule;
use pdf_writer::Finish;
use std::sync::Arc;

pub fn get_mask(mask: &usvg::Mask) -> Mask {
    // Dummy size. TODO: Improve?
    let mut serializer_context = SerializerContext::new(SerializeSettings::default());
    let mut stream_builder = StreamBuilder::new(&mut serializer_context);

    if let Some(mask) = mask.mask() {
        let mut sub_stream_builder = StreamBuilder::new(stream_builder.serializer_context());
        remaining(mask, &mut sub_stream_builder);
        let sub_stream = sub_stream_builder.finish();
        stream_builder.draw_masked(get_mask(mask), Arc::new(sub_stream));
    } else {
        remaining(mask, &mut stream_builder);
    };

    let stream = stream_builder.finish();

    Mask::new(Arc::new(stream), convert_mask_type(&mask.kind()))
}

fn remaining(mask: &usvg::Mask, stream_builder: &mut StreamBuilder) {
    let clip_path = mask.rect().to_rect().to_clip_path();
    stream_builder.push_clip_path(&clip_path, &FillRule::NonZero);
    group::render(mask.root(), stream_builder);
    stream_builder.pop_clip_path();
}
