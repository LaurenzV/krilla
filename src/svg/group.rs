use crate::canvas::Surface;
use crate::object::image::Image;
use crate::svg::clip_path::{get_clip_path, SvgClipPath};
use crate::svg::mask::get_mask;
use crate::svg::util::{convert_blend_mode, convert_transform};
use crate::svg::{group, path};
use crate::util::RectExt;
use crate::FillRule;
use image::ImageFormat;
use tiny_skia_path::{Rect, Size, Transform};
use usvg::{ImageKind, Node};

pub fn render(group: &usvg::Group, surface: &mut dyn Surface) {
    if !group.filters().is_empty() {
        unimplemented!();
    }

    isolated(group, surface);
}

pub fn isolated(group: &usvg::Group, surface: &mut dyn Surface) {
    let isolated = if group.isolate() {
        &mut surface.isolated()
    } else {
        surface
    };

    transformed(group, isolated);
}

pub fn transformed(group: &usvg::Group, surface: &mut dyn Surface) {
    let mut transformed = surface.transformed(convert_transform(&group.transform()));
    clipped(group, &mut transformed);
}

pub fn clipped(group: &usvg::Group, surface: &mut dyn Surface) {
    let clipped: &mut dyn Surface = if let Some(clip_path) = group.clip_path() {
        let converted = get_clip_path(group, clip_path);
        match converted {
            SvgClipPath::SimpleClip(rules) => &mut surface.clipped_many(rules),
            SvgClipPath::ComplexClip(mask) => &mut surface.masked(mask),
        }
    } else {
        surface
    };

    blended_and_opacified(group, clipped);
}

pub fn blended_and_opacified(group: &usvg::Group, surface: &mut dyn Surface) {
    let masked = if let Some(mask) = group.mask() {
        &mut surface.masked(get_mask(mask))
    } else {
        surface
    };

    let mut blended = masked.blended(convert_blend_mode(&group.blend_mode()));
    let mut opacified = blended.opacified(group.opacity());

    for child in group.children() {
        match child {
            Node::Group(g) => render(g, &mut opacified),
            Node::Path(p) => path::render(p, &mut opacified),
            Node::Image(i) => match i.kind() {
                ImageKind::JPEG(d) => {
                    let dynamic_image =
                        image::load_from_memory_with_format(d.as_slice(), ImageFormat::Jpeg)
                            .unwrap();
                    let image = Image::new(&dynamic_image);
                    opacified.draw_image(image, i.size(), Transform::default());
                }
                ImageKind::PNG(d) => {
                    let dynamic_image =
                        image::load_from_memory_with_format(d.as_slice(), ImageFormat::Png)
                            .unwrap();
                    let image = Image::new(&dynamic_image);
                    opacified.draw_image(image, i.size(), Transform::default());
                }
                ImageKind::GIF(d) => {
                    let dynamic_image =
                        image::load_from_memory_with_format(d.as_slice(), ImageFormat::Gif)
                            .unwrap();
                    let image = Image::new(&dynamic_image);
                    opacified.draw_image(image, i.size(), Transform::default());
                }
                ImageKind::SVG(t) => {
                    let mut clipped = opacified.clipped(
                        Rect::from_xywh(0.0, 0.0, t.size().width(), t.size().height())
                            .unwrap()
                            .to_clip_path(),
                        FillRule::NonZero,
                    );
                    render(t.root(), &mut clipped);
                }
            },
            Node::Text(t) => unimplemented!(),
        }
    }
}
