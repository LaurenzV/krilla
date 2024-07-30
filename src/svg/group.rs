use crate::stream::StreamBuilder;
use crate::svg::clip_path::{get_clip_path, SvgClipPath};
use crate::svg::mask::get_mask;
use crate::svg::util::{convert_blend_mode, convert_transform};
// use crate::svg::{filter, image, path};
use crate::svg::filter;
use pdf_writer::Finish;
use std::sync::Arc;
use usvg::{Node, NormalizedF32};

pub fn render(group: &usvg::Group, stream_builder: &mut StreamBuilder) {
    if !group.filters().is_empty() {
        filter::render(group, stream_builder);
        return;
    }

    isolated(group, stream_builder);
}

pub fn isolated(group: &usvg::Group, stream_builder: &mut StreamBuilder) {
    if group.isolate() {
        let mut sub_stream_builder = StreamBuilder::new(stream_builder.serializer_context());
        transformed(group, &mut sub_stream_builder);
        let sub_stream = sub_stream_builder.finish();

        stream_builder.draw_isolated(sub_stream);
    } else {
        transformed(group, stream_builder);
    }
}

pub fn transformed(group: &usvg::Group, stream_builder: &mut StreamBuilder) {
    stream_builder.save_graphics_state();
    stream_builder.concat_transform(&convert_transform(&group.transform()));
    clipped(group, stream_builder);
    stream_builder.restore_graphics_state();
}

pub fn clipped(group: &usvg::Group, stream_builder: &mut StreamBuilder) {
    if let Some(clip_path) = group.clip_path() {
        let converted = get_clip_path(group, clip_path, stream_builder.serializer_context());
        // TODO: Improve and deduplicate
        match converted {
            SvgClipPath::SimpleClip(rules) => {
                for rule in &rules {
                    stream_builder.push_clip_path(&rule.0, &rule.1);
                }

                masked(group, stream_builder);

                for _ in rules {
                    stream_builder.pop_clip_path();
                }
            }
            SvgClipPath::ComplexClip(mask) => {
                let mut sub_stream_builder =
                    StreamBuilder::new(stream_builder.serializer_context());
                masked(group, &mut sub_stream_builder);
                let sub_stream = sub_stream_builder.finish();
                stream_builder.draw_masked(mask, Arc::new(sub_stream));
            }
        }
    } else {
        masked(group, stream_builder);
    };
}

pub fn masked(group: &usvg::Group, stream_builder: &mut StreamBuilder) {
    if let Some(mask) = group.mask() {
        let mut sub_stream_builder = StreamBuilder::new(stream_builder.serializer_context());
        blended_and_opacified(group, &mut sub_stream_builder);
        let sub_stream = sub_stream_builder.finish();
        let mask = get_mask(mask, stream_builder.serializer_context());
        stream_builder.draw_masked(mask, Arc::new(sub_stream));
    } else {
        blended_and_opacified(group, stream_builder);
    }
}

pub fn blended_and_opacified(group: &usvg::Group, stream_builder: &mut StreamBuilder) {
    stream_builder.save_graphics_state();
    stream_builder.set_blend_mode(convert_blend_mode(&group.blend_mode()));

    if group.opacity() == NormalizedF32::ONE {
        for child in group.children() {
            render_node(child, stream_builder);
        }
    } else {
        let mut sub_stream_builder = StreamBuilder::new(stream_builder.serializer_context());
        for child in group.children() {
            render_node(child, &mut sub_stream_builder);
        }
        let sub_stream = sub_stream_builder.finish();
        stream_builder.draw_opacified(group.opacity(), Arc::new(sub_stream));
    }

    stream_builder.restore_graphics_state();
}

pub fn render_node(node: &Node, stream_builder: &mut StreamBuilder) {
    match node {
        Node::Group(g) => render(g, stream_builder),
        // Node::Path(p) => path::render(p, stream_builder),
        Node::Path(p) => unimplemented!(),
        // Node::Image(i) => image::render(i, stream_builder),
        Node::Image(i) => unimplemented!(),
        Node::Text(t) => unimplemented!(),
    }
}
