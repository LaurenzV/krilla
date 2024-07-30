use crate::serialize::SerializerContext;
use crate::stream::StreamBuilder;
use crate::svg::group;
use crate::transform::TransformWrapper;
use crate::{
    Color, Fill, FillRule, LineCap, LineJoin, LinearGradient, MaskType, Paint, Pattern,
    RadialGradient, SpreadMethod, Stop, Stroke, StrokeDash,
};
use pdf_writer::types::BlendMode;
use std::sync::Arc;
use tiny_skia_path::{FiniteF32, NormalizedF32, Transform};
use usvg::NonZeroPositiveF32;

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

pub fn convert_stop(s: &usvg::Stop) -> Stop {
    Stop {
        offset: s.offset(),
        color: Color::new_rgb(s.color().red, s.color().green, s.color().blue),
        opacity: NormalizedF32::new(s.opacity().get()).unwrap(),
    }
}

pub fn convert_paint(paint: &usvg::Paint, serializer_context: &mut SerializerContext) -> Paint {
    match paint {
        usvg::Paint::Color(c) => Paint::Color(Color::new_rgb(c.red, c.green, c.blue)),
        usvg::Paint::LinearGradient(lg) => Paint::LinearGradient(LinearGradient {
            x1: FiniteF32::new(lg.x1()).unwrap(),
            y1: FiniteF32::new(lg.y1()).unwrap(),
            x2: FiniteF32::new(lg.x2()).unwrap(),
            y2: FiniteF32::new(lg.y2()).unwrap(),
            transform: TransformWrapper(convert_transform(&lg.transform())),
            spread_method: convert_spread_mode(&lg.spread_method()),
            stops: lg
                .stops()
                .iter()
                .map(|s| convert_stop(s))
                .collect::<Vec<_>>(),
        }),
        usvg::Paint::RadialGradient(rg) => Paint::RadialGradient(RadialGradient {
            cx: FiniteF32::new(rg.cx()).unwrap(),
            cy: FiniteF32::new(rg.cy()).unwrap(),
            cr: FiniteF32::new(rg.r().get()).unwrap(),
            fx: FiniteF32::new(rg.fx()).unwrap(),
            fy: FiniteF32::new(rg.fy()).unwrap(),
            fr: FiniteF32::new(0.0).unwrap(),
            transform: TransformWrapper(convert_transform(&rg.transform())),
            spread_method: convert_spread_mode(&rg.spread_method()),
            stops: rg
                .stops()
                .iter()
                .map(|s| convert_stop(s))
                .collect::<Vec<_>>(),
        }),
        usvg::Paint::Pattern(pat) => {
            let mut stream_builder = StreamBuilder::new(serializer_context);
            group::render(pat.root(), &mut stream_builder);
            let stream = stream_builder.finish();

            Paint::Pattern(Arc::new(Pattern {
                stream: Arc::new(stream),
                transform: TransformWrapper(
                    pat.transform()
                        .pre_concat(Transform::from_translate(pat.rect().x(), pat.rect().y())),
                ),
                width: FiniteF32::new(pat.rect().width()).unwrap(),
                height: FiniteF32::new(pat.rect().height()).unwrap(),
            }))
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

pub fn convert_fill(fill: &usvg::Fill, serializer_context: &mut SerializerContext) -> Fill {
    Fill {
        paint: convert_paint(fill.paint(), serializer_context),
        opacity: fill.opacity(),
        rule: convert_fill_rule(&fill.rule()),
    }
}

pub fn convert_stroke(stroke: &usvg::Stroke, serializer_context: &mut SerializerContext) -> Stroke {
    let dash = if let Some(dash_array) = stroke.dasharray() {
        Some(StrokeDash {
            offset: FiniteF32::new(stroke.dashoffset()).unwrap(),
            array: dash_array
                .iter()
                .map(|d| FiniteF32::new(*d).unwrap())
                .collect::<Vec<_>>(),
        })
    } else {
        None
    };

    Stroke {
        paint: convert_paint(stroke.paint(), serializer_context),
        width: stroke.width(),
        miter_limit: NonZeroPositiveF32::new(stroke.miterlimit().get()).unwrap(),
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
