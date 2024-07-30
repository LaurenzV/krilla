use crate::object::mask::Mask;
use crate::serialize::{SerializeSettings, SerializerContext};
use crate::stream::StreamBuilder;
use crate::svg::group;
use crate::svg::group::masked;
use crate::svg::util::{convert_fill_rule, convert_transform};
use crate::{FillRule, MaskType};
use pdf_writer::Finish;
use std::sync::Arc;
use tiny_skia_path::{Path, PathBuilder, PathSegment, Size, Transform};

pub enum SvgClipPath {
    SimpleClip(Vec<(Path, FillRule)>),
    ComplexClip(Mask),
}

pub fn get_clip_path(
    group: &usvg::Group,
    clip_path: &usvg::ClipPath,
    serializer_context: &mut SerializerContext,
) -> SvgClipPath {
    // Unfortunately, clip paths are a bit tricky to deal with, the reason being that clip paths in
    // SVGs can be much more complex than in PDF. In SVG, clip paths can have transforms, as well as
    // nested clip paths. The objects inside of the clip path can have transforms as well, making it
    // even more difficult to deal with. Because in PDF, once you start a clip path operation you
    // cannot interrupt it, because once you pop the current graphics state, the clip path will be
    // lost since it is part of the current graphics state. However, if we have various transforms
    // on the children, we need to be able to push/pop the graphics state, so that the children's
    // transforms don't affect each other. Initially, because of this, clip paths were only implemented
    // using soft masks, but Safari has a couple of issues with rendering them properly. Not to mention
    // the fact that soft masks are obviously also more expensive. Because of this, we proceed the following
    // way: We first check whether the clip path itself is too "complex" (complex being that it fulfills
    // some attributes that make it impossible to represent them in our current setup using just native
    // PDF clip paths. If it is too complex, we fall back to using soft masks. However, if it is simple
    // enough, we just use the normal clip path operator in PDF. It should be noted that in reality,
    // only very few SVGs seem to have such complex clipping paths (they are not even rendered correctly
    // by all online converters that were tested), so in most real-life scenarios, the simple version
    // should suffice. But in order to conform with the SVG specification, we also handle the case
    // of more complex clipping paths, even if this means that Safari will in some cases not
    // display them correctly.

    let is_simple_clip_path = is_simple_clip_path(clip_path.root());
    let clip_rules = collect_clip_rules(clip_path.root());

    if is_simple_clip_path
        && (clip_rules.iter().all(|f| *f == usvg::FillRule::NonZero)
        // For even odd, there must be at most one shape in the group, because
        // overlapping shapes with evenodd render differently in PDF
        || (clip_rules.iter().all(|f| *f == usvg::FillRule::EvenOdd)
        && clip_rules.len() == 1))
    {
        let clips = create_simple_clip_path(
            clip_path,
            clip_rules
                .first()
                .copied()
                .unwrap_or(usvg::FillRule::NonZero),
        );
        SvgClipPath::SimpleClip(clips)
    } else {
        SvgClipPath::ComplexClip(create_complex_clip_path(
            group,
            clip_path,
            serializer_context,
        ))
    }
}

fn create_simple_clip_path(
    clip_path: &usvg::ClipPath,
    clip_rule: usvg::FillRule,
) -> Vec<(Path, FillRule)> {
    let mut clips = vec![];
    if let Some(clip_path) = clip_path.clip_path() {
        clips.extend(create_simple_clip_path(clip_path, clip_rule));
    }

    // Just a dummy operation, so that in case the clip path only has hidden children the clip
    // path will still be applied and everything will be hidden.
    let mut path_builder = PathBuilder::new();
    path_builder.move_to(0.0, 0.0);

    let base_transform = clip_path.transform();
    extend_segments_from_group(clip_path.root(), &base_transform, &mut path_builder);

    clips.push((
        path_builder.finish().unwrap(),
        convert_fill_rule(&clip_rule),
    ));
    clips
}

fn extend_segments_from_group(
    group: &usvg::Group,
    transform: &Transform,
    path_builder: &mut PathBuilder,
) {
    for child in group.children() {
        match child {
            usvg::Node::Path(ref path) => {
                if path.is_visible() {
                    path.data().segments().for_each(|segment| match segment {
                        PathSegment::MoveTo(mut p) => {
                            transform.map_point(&mut p);
                            path_builder.move_to(p.x, p.y);
                        }
                        PathSegment::LineTo(mut p) => {
                            transform.map_point(&mut p);
                            path_builder.line_to(p.x, p.y)
                        }
                        PathSegment::QuadTo(p1, p2) => {
                            let mut points = [p1, p2];
                            transform.map_points(&mut points);
                            path_builder.quad_to(
                                points[0].x,
                                points[0].y,
                                points[1].x,
                                points[1].y,
                            );
                        }
                        PathSegment::CubicTo(p1, p2, p3) => {
                            let mut points = [p1, p2, p3];
                            transform.map_points(&mut points);
                            path_builder.cubic_to(
                                points[0].x,
                                points[0].y,
                                points[1].x,
                                points[1].y,
                                points[2].x,
                                points[2].y,
                            );
                        }
                        PathSegment::Close => path_builder.close(),
                    })
                }
            }
            usvg::Node::Group(ref group) => {
                let group_transform = transform.pre_concat(group.transform());
                extend_segments_from_group(group, &group_transform, path_builder);
            }
            usvg::Node::Text(ref text) => {
                // We could in theory preserve text in clip paths by using the appropriate
                // rendering mode, but for now we just use the flattened version.
                extend_segments_from_group(text.flattened(), transform, path_builder);
            }
            // Images are not valid in a clip path.
            _ => {}
        }
    }
}

fn is_simple_clip_path(group: &usvg::Group) -> bool {
    group.children().iter().all(|n| {
        match n {
            usvg::Node::Group(ref group) => {
                // We can only intersect one clipping path with another one, meaning that we
                // can convert nested clip paths if a second clip path is defined on the clip
                // path itself, but not if it is defined on a child.
                group.clip_path().is_none() && is_simple_clip_path(group)
            }
            _ => true,
        }
    })
}

fn collect_clip_rules(group: &usvg::Group) -> Vec<usvg::FillRule> {
    let mut clip_rules = vec![];
    group.children().iter().for_each(|n| match n {
        usvg::Node::Path(ref path) => {
            if let Some(fill) = &path.fill() {
                clip_rules.push(fill.rule());
            }
        }
        usvg::Node::Text(ref text) => clip_rules.extend(collect_clip_rules(text.flattened())),
        usvg::Node::Group(ref group) => {
            clip_rules.extend(collect_clip_rules(group));
        }
        _ => {}
    });

    clip_rules
}

fn create_complex_clip_path(
    parent: &usvg::Group,
    clip_path: &usvg::ClipPath,
    serializer_context: &mut SerializerContext,
) -> Mask {
    let mut stream_builder = StreamBuilder::new(serializer_context);

    if let Some(svg_clip_path) = clip_path
        .clip_path()
        .map(|c| get_clip_path(parent, clip_path, stream_builder.serializer_context()))
    {
        match svg_clip_path {
            SvgClipPath::SimpleClip(rules) => {
                for rule in &rules {
                    stream_builder.push_clip_path(&rule.0, &rule.1);
                }

                transformed(clip_path, &mut stream_builder);

                for _ in rules {
                    stream_builder.pop_clip_path();
                }
            }
            SvgClipPath::ComplexClip(mask) => {
                let mut sub_stream_builder =
                    StreamBuilder::new(stream_builder.serializer_context());
                transformed(clip_path, &mut sub_stream_builder);
                let sub_stream = sub_stream_builder.finish();
                stream_builder.draw_masked(mask, Arc::new(sub_stream));
            }
        }
    }

    let stream = stream_builder.finish();

    Mask::new(Arc::new(stream), MaskType::Alpha)
}

fn transformed(clip_path: &usvg::ClipPath, stream_builder: &mut StreamBuilder) {
    stream_builder.save_graphics_state();
    stream_builder.concat_transform(&convert_transform(&clip_path.transform()));
    group::render(clip_path.root(), stream_builder);
    stream_builder.restore_graphics_state();
}
