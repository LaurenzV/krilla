use crate::mask::MaskType;
use crate::object::color::rgb;
use crate::object::color::rgb::Rgb;
use crate::paint::{LinearGradient, Paint, Pattern, RadialGradient, SpreadMethod, Stop};
use crate::path::{Fill, FillRule, LineCap, LineJoin, Stroke, StrokeDash};
use crate::stream::StreamBuilder;
use crate::svg::{group, ProcessContext};
use pdf_writer::types::BlendMode;
use tiny_skia_path::{NormalizedF32, Transform};

/// Convert a usvg `Transform` into a krilla `Transform`.
pub fn convert_transform(transform: &Transform) -> Transform {
    Transform {
        sx: transform.sx,
        kx: transform.kx,
        ky: transform.ky,
        sy: transform.sy,
        tx: transform.tx,
        ty: transform.ty,
    }
}

/// Convert a usvg `SpreadMethod` into a krilla `SpreadMethod`.
pub fn convert_spread_method(spread_method: &usvg::SpreadMethod) -> SpreadMethod {
    match spread_method {
        usvg::SpreadMethod::Pad => SpreadMethod::Pad,
        usvg::SpreadMethod::Reflect => SpreadMethod::Reflect,
        usvg::SpreadMethod::Repeat => SpreadMethod::Repeat,
    }
}

/// Convert a usvg `Stop` into a krilla `Stop`.
pub fn convert_stop(stop: &usvg::Stop) -> Stop<Rgb> {
    Stop {
        offset: stop.offset(),
        color: rgb::Color::new(stop.color().red, stop.color().green, stop.color().blue),
        opacity: NormalizedF32::new(stop.opacity().get()).unwrap(),
    }
}

/// Convert a usvg `Paint` into a krilla `Paint`.
pub fn convert_paint(
    paint: &usvg::Paint,
    mut stream_builder: StreamBuilder,
    process_context: &mut ProcessContext,
    // The additional transform is needed because in krilla, a transform to a shape will also apply
    // the transform to the paint server. However, in the case of SVG glyphs, we don't want the transform
    // to be shifted for each glyph we draw (since we draw them separately instead of in a glyph run),
    // so we need to apply an additional inverse transform to counter that effect.
    additional_transform: Transform,
) -> Paint {
    match paint {
        usvg::Paint::Color(c) => rgb::Color::new(c.red, c.green, c.blue).into(),
        usvg::Paint::LinearGradient(lg) => LinearGradient {
            x1: lg.x1(),
            y1: lg.y1(),
            x2: lg.x2(),
            y2: lg.y2(),
            transform: additional_transform.pre_concat(convert_transform(&lg.transform())),
            spread_method: convert_spread_method(&lg.spread_method()),
            stops: lg.stops().iter().map(convert_stop).collect::<Vec<_>>(),
        }.into(),
        usvg::Paint::RadialGradient(rg) => RadialGradient {
            cx: rg.cx(),
            cy: rg.cy(),
            cr: rg.r().get(),
            fx: rg.fx(),
            fy: rg.fy(),
            fr: 0.0,
            transform: additional_transform.pre_concat(convert_transform(&rg.transform())),
            spread_method: convert_spread_method(&rg.spread_method()),
            stops: rg.stops().iter().map(convert_stop).collect::<Vec<_>>(),
        }.into(),
        usvg::Paint::Pattern(pat) => {
            let mut surface = stream_builder.surface();
            group::render(pat.root(), &mut surface, process_context);
            surface.finish();
            let stream = stream_builder.finish();

            Pattern {
                stream,
                transform: additional_transform
                    .pre_concat(pat.transform())
                    .pre_concat(Transform::from_translate(pat.rect().x(), pat.rect().y())),
                width: pat.rect().width(),
                height: pat.rect().height(),
            }.into()
        }
    }
}

/// Convert a usvg `LineCap` into a krilla `LineCap`.
pub fn convert_line_cap(line_cap: &usvg::LineCap) -> LineCap {
    match line_cap {
        usvg::LineCap::Butt => LineCap::Butt,
        usvg::LineCap::Round => LineCap::Round,
        usvg::LineCap::Square => LineCap::Square,
    }
}

/// Convert a usvg `LineJoin` into a krilla `LineJoin`.
pub fn convert_line_join(line_join: &usvg::LineJoin) -> LineJoin {
    match line_join {
        usvg::LineJoin::Miter => LineJoin::Miter,
        usvg::LineJoin::MiterClip => LineJoin::Miter,
        usvg::LineJoin::Round => LineJoin::Round,
        usvg::LineJoin::Bevel => LineJoin::Bevel,
    }
}

/// Convert a usvg `FillRule` into a krilla `FillRule`.
pub fn convert_fill_rule(fill_rule: &usvg::FillRule) -> FillRule {
    match fill_rule {
        usvg::FillRule::NonZero => FillRule::NonZero,
        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
    }
}

/// Convert a usvg `Fill` into a krilla `Fill`.
pub fn convert_fill(
    fill: &usvg::Fill,
    stream_builder: StreamBuilder,
    process_context: &mut ProcessContext,
    additional_transform: Transform,
) -> Fill {
    Fill {
        paint: convert_paint(
            fill.paint(),
            stream_builder,
            process_context,
            additional_transform,
        ),
        opacity: fill.opacity(),
        rule: convert_fill_rule(&fill.rule()),
    }
}

/// Convert a usvg `Stroke` into a krilla `Stroke`.
pub fn convert_stroke(
    stroke: &usvg::Stroke,
    stream_builder: StreamBuilder,
    process_context: &mut ProcessContext,
    additional_transform: Transform,
) -> Stroke {
    let dash = stroke.dasharray().map(|dash_array| StrokeDash {
        offset: stroke.dashoffset(),
        array: dash_array.to_vec(),
    });

    Stroke {
        paint: convert_paint(
            stroke.paint(),
            stream_builder,
            process_context,
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

/// Convert a usvg `BlendMode` into a krilla `BlendMode`.
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

/// Convert a usvg `MaskType` into a krilla `MaskType`.
pub fn convert_mask_type(mask_type: &usvg::MaskType) -> MaskType {
    match mask_type {
        usvg::MaskType::Luminance => MaskType::Luminosity,
        usvg::MaskType::Alpha => MaskType::Alpha,
    }
}
