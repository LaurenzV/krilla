use crate::object::color_space::rgb::Srgb;
use crate::surface::StreamBuilder;
use crate::svg::{group, FontContext};
use crate::{
    Fill, FillRule, LineCap, LineJoin, LinearGradient, MaskType, Paint, Pattern, RadialGradient,
    SpreadMethod, Stop, Stroke, StrokeDash,
};
use pdf_writer::types::BlendMode;
use tiny_skia_path::{NormalizedF32, Transform};

pub fn convert_transform(transform: &usvg::Transform) -> Transform {
    Transform {
        sx: transform.sx,
        kx: transform.kx,
        ky: transform.ky,
        sy: transform.sy,
        tx: transform.tx,
        ty: transform.ty,
    }
}

pub fn convert_spread_mode(s: &usvg::SpreadMethod) -> SpreadMethod {
    match s {
        usvg::SpreadMethod::Pad => SpreadMethod::Pad,
        usvg::SpreadMethod::Reflect => SpreadMethod::Reflect,
        usvg::SpreadMethod::Repeat => SpreadMethod::Repeat,
    }
}

pub fn convert_stop(s: &usvg::Stop) -> Stop<Srgb> {
    Stop {
        offset: s.offset(),
        color: Srgb::new_rgb(s.color().red, s.color().green, s.color().blue).into(),
        opacity: NormalizedF32::new(s.opacity().get()).unwrap(),
    }
}

pub fn convert_paint(
    paint: &usvg::Paint,
    mut sub_builder: StreamBuilder,
    font_context: &mut FontContext,
    additional_transform: Transform,
) -> Paint<Srgb> {
    match paint {
        usvg::Paint::Color(c) => Paint::Color(Srgb::new_rgb(c.red, c.green, c.blue).into()),
        usvg::Paint::LinearGradient(lg) => Paint::LinearGradient(LinearGradient {
            x1: lg.x1(),
            y1: lg.y1(),
            x2: lg.x2(),
            y2: lg.y2(),
            transform: additional_transform.pre_concat(convert_transform(&lg.transform())),
            spread_method: convert_spread_mode(&lg.spread_method()),
            stops: lg
                .stops()
                .iter()
                .map(|s| convert_stop(s))
                .collect::<Vec<_>>(),
        }),
        usvg::Paint::RadialGradient(rg) => Paint::RadialGradient(RadialGradient {
            cx: rg.cx(),
            cy: rg.cy(),
            cr: rg.r().get(),
            fx: rg.fx(),
            fy: rg.fy(),
            fr: 0.0,
            transform: additional_transform.pre_concat(convert_transform(&rg.transform())),
            spread_method: convert_spread_mode(&rg.spread_method()),
            stops: rg
                .stops()
                .iter()
                .map(|s| convert_stop(s))
                .collect::<Vec<_>>(),
        }),
        usvg::Paint::Pattern(pat) => {
            let mut surface = sub_builder.surface();
            group::render(pat.root(), &mut surface, font_context);
            surface.finish();
            let stream = sub_builder.finish();

            Paint::Pattern(Pattern {
                stream,
                transform: additional_transform
                    .pre_concat(pat.transform())
                    .pre_concat(Transform::from_translate(pat.rect().x(), pat.rect().y())),
                width: pat.rect().width(),
                height: pat.rect().height(),
            })
        }
    }
}

pub fn convert_line_cap(linecap: &usvg::LineCap) -> LineCap {
    match linecap {
        usvg::LineCap::Butt => LineCap::Butt,
        usvg::LineCap::Round => LineCap::Round,
        usvg::LineCap::Square => LineCap::Square,
    }
}

pub fn convert_line_join(line_join: &usvg::LineJoin) -> LineJoin {
    match line_join {
        usvg::LineJoin::Miter => LineJoin::Miter,
        usvg::LineJoin::MiterClip => LineJoin::Miter,
        usvg::LineJoin::Round => LineJoin::Round,
        usvg::LineJoin::Bevel => LineJoin::Bevel,
    }
}

pub fn convert_fill_rule(rule: &usvg::FillRule) -> FillRule {
    match rule {
        usvg::FillRule::NonZero => FillRule::NonZero,
        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
    }
}

pub fn convert_fill(
    fill: &usvg::Fill,
    sub_builder: StreamBuilder,
    font_context: &mut FontContext,
    additional_transform: Transform,
) -> Fill<Srgb> {
    Fill {
        paint: convert_paint(
            fill.paint(),
            sub_builder,
            font_context,
            additional_transform,
        ),
        opacity: fill.opacity(),
        rule: convert_fill_rule(&fill.rule()),
    }
}

pub fn convert_stroke(
    stroke: &usvg::Stroke,
    sub_builder: StreamBuilder,
    font_context: &mut FontContext,
    additional_transform: Transform,
) -> Stroke<Srgb> {
    let dash = if let Some(dash_array) = stroke.dasharray() {
        Some(StrokeDash {
            offset: stroke.dashoffset(),
            array: dash_array.to_vec(),
        })
    } else {
        None
    };

    Stroke {
        paint: convert_paint(
            stroke.paint(),
            sub_builder,
            font_context,
            additional_transform,
        ),
        width: stroke.width().get(),
        miter_limit: stroke.miterlimit().get(),
        line_cap: convert_line_cap(&stroke.linecap()),
        line_join: convert_line_join(&stroke.linejoin()),
        opacity: stroke.opacity(),
        dash,
    }
}

pub fn convert_blend_mode(blend_mode: &usvg::BlendMode) -> BlendMode {
    match blend_mode {
        usvg::BlendMode::Normal => BlendMode::Normal,
        usvg::BlendMode::Multiply => BlendMode::Multiply,
        usvg::BlendMode::Screen => BlendMode::Screen,
        usvg::BlendMode::Overlay => BlendMode::Overlay,
        usvg::BlendMode::Darken => BlendMode::Darken,
        usvg::BlendMode::Lighten => BlendMode::Lighten,
        usvg::BlendMode::ColorDodge => BlendMode::ColorDodge,
        usvg::BlendMode::ColorBurn => BlendMode::ColorBurn,
        usvg::BlendMode::HardLight => BlendMode::HardLight,
        usvg::BlendMode::SoftLight => BlendMode::SoftLight,
        usvg::BlendMode::Difference => BlendMode::Difference,
        usvg::BlendMode::Exclusion => BlendMode::Exclusion,
        usvg::BlendMode::Hue => BlendMode::Hue,
        usvg::BlendMode::Saturation => BlendMode::Saturation,
        usvg::BlendMode::Color => BlendMode::Color,
        usvg::BlendMode::Luminosity => BlendMode::Luminosity,
    }
}

pub fn convert_mask_type(mask_type: &usvg::MaskType) -> MaskType {
    match mask_type {
        usvg::MaskType::Luminance => MaskType::Luminosity,
        usvg::MaskType::Alpha => MaskType::Alpha,
    }
}
