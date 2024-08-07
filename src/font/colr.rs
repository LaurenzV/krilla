use crate::canvas::CanvasBuilder;
use crate::font::{Font, OutlineBuilder};
use crate::object::color_space::srgb;
use crate::object::color_space::srgb::Srgb;
use crate::paint::{LinearGradient, Paint, RadialGradient, SpreadMethod, Stop, SweepGradient};
use crate::{Fill, FillRule};
use pdf_writer::types::BlendMode;
use skrifa::color::{Brush, ColorPainter, ColorStop, CompositeMode};
use skrifa::outline::DrawSettings;
use skrifa::prelude::LocationRef;
use skrifa::raw::types::BoundingBox;
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::{NormalizedF32, Path, PathBuilder, Transform};

pub fn draw_glyph(font: Font, glyph: GlyphId, canvas_builder: &mut CanvasBuilder) -> Option<()> {
    let colr_glyphs = font.font_ref().color_glyphs();
    if let Some(colr_glyph) = colr_glyphs.get(glyph) {
        canvas_builder.push_transform(&Transform::from_scale(1.0, -1.0));
        let mut colr_canvas = ColrCanvas::new(font.clone(), canvas_builder);
        let _ = colr_glyph.paint(font.location_ref(), &mut colr_canvas);
        canvas_builder.pop_transform();
        return Some(());
    } else {
        return None;
    }
}

struct ColrCanvas<'a, 'b> {
    font: Font,
    clips: Vec<Vec<Path>>,
    canvas_builder: &'b mut CanvasBuilder<'a>,
    transforms: Vec<Transform>,
}

impl<'a, 'b> ColrCanvas<'a, 'b> {
    pub fn new(font: Font, canvas_builder: &'b mut CanvasBuilder<'a>) -> Self {
        Self {
            font,
            canvas_builder,
            transforms: vec![Transform::identity()],
            clips: vec![vec![]],
        }
    }
}

impl<'a, 'b> ColrCanvas<'a, 'b> {
    fn palette_index_to_color(
        &self,
        palette_index: u16,
        alpha: f32,
    ) -> (srgb::Color, NormalizedF32) {
        if palette_index != u16::MAX {
            let color = self
                .font
                .font_ref()
                .cpal()
                .unwrap()
                .color_records_array()
                .unwrap()
                .unwrap()[palette_index as usize];

            (
                Srgb::new_rgb(color.red, color.green, color.blue),
                NormalizedF32::new(alpha * color.alpha as f32 / 255.0).unwrap(),
            )
        } else {
            (Srgb::new_rgb(0, 0, 0), NormalizedF32::new(alpha).unwrap())
        }
    }

    fn stops(&self, stops: &[ColorStop]) -> Vec<Stop<Srgb>> {
        stops
            .iter()
            .map(|s| {
                let (color, alpha) = self.palette_index_to_color(s.palette_index, s.alpha);

                Stop {
                    offset: NormalizedF32::new(s.offset).unwrap(),
                    color,
                    opacity: alpha,
                }
            })
            .collect::<Vec<_>>()
    }
}

trait ExtendExt {
    fn to_spread_method(&self) -> SpreadMethod;
}

impl ExtendExt for skrifa::color::Extend {
    fn to_spread_method(&self) -> SpreadMethod {
        match self {
            skrifa::color::Extend::Pad => SpreadMethod::Pad,
            skrifa::color::Extend::Repeat => SpreadMethod::Repeat,
            skrifa::color::Extend::Reflect => SpreadMethod::Reflect,
            skrifa::color::Extend::Unknown => SpreadMethod::Pad,
        }
    }
}

impl<'a, 'b> ColorPainter for ColrCanvas<'a, 'b> {
    fn push_transform(&mut self, transform: skrifa::color::Transform) {
        let new_transform = self
            .transforms
            .last()
            .unwrap()
            .pre_concat(Transform::from_row(
                transform.xx,
                transform.yx,
                transform.xy,
                transform.yy,
                transform.dx,
                transform.dy,
            ));
        self.transforms.push(new_transform);
    }

    fn pop_transform(&mut self) {
        self.transforms.pop();
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        let mut old = self.clips.last().unwrap().clone();

        let mut glyph_builder = OutlineBuilder(PathBuilder::new());
        let outline_glyphs = self.font.font_ref().outline_glyphs();
        let outline_glyph = outline_glyphs.get(glyph_id).unwrap();
        outline_glyph
            .draw(
                DrawSettings::unhinted(skrifa::instance::Size::unscaled(), LocationRef::default()),
                &mut glyph_builder,
            )
            .unwrap();
        let path = glyph_builder
            .finish()
            .unwrap()
            .transform(*self.transforms.last().unwrap())
            .unwrap();

        old.push(path);

        self.clips.push(old);
    }

    fn push_clip_box(&mut self, clip_box: BoundingBox<f32>) {
        let mut old = self.clips.last().unwrap().clone();

        let mut path_builder = PathBuilder::new();
        path_builder.move_to(clip_box.x_min, clip_box.y_min);
        path_builder.line_to(clip_box.x_min, clip_box.y_max);
        path_builder.line_to(clip_box.x_max, clip_box.y_max);
        path_builder.line_to(clip_box.x_max, clip_box.y_min);
        path_builder.close();

        let path = path_builder
            .finish()
            .unwrap()
            .transform(*self.transforms.last().unwrap())
            .unwrap();
        old.push(path);

        self.clips.push(old);
    }

    fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn fill(&mut self, brush: Brush<'_>) {
        if let Some(fill) = match brush {
            Brush::Solid {
                palette_index,
                alpha,
            } => {
                let (color, alpha) = self.palette_index_to_color(palette_index, alpha);
                Some(Fill {
                    paint: Paint::Color(color),
                    opacity: alpha,
                    rule: Default::default(),
                })
            }
            Brush::LinearGradient {
                p0,
                p1,
                color_stops,
                extend,
            } => {
                let linear = LinearGradient {
                    x1: p0.x,
                    y1: p0.y,
                    x2: p1.x,
                    y2: p1.y,
                    stops: self.stops(color_stops),
                    spread_method: extend.to_spread_method(),
                    transform: *self.transforms.last().unwrap(),
                };

                Some(Fill {
                    paint: Paint::LinearGradient(linear),
                    opacity: NormalizedF32::ONE,
                    rule: Default::default(),
                })
            }
            Brush::RadialGradient {
                c0,
                r0,
                c1,
                r1,
                color_stops,
                extend,
            } => {
                let radial = RadialGradient {
                    fx: c0.x,
                    fy: c0.y,
                    fr: r0,
                    cx: c1.x,
                    cy: c1.y,
                    cr: r1,
                    stops: self.stops(color_stops),
                    spread_method: extend.to_spread_method(),
                    transform: *self.transforms.last().unwrap(),
                };

                Some(Fill {
                    paint: Paint::RadialGradient(radial),
                    opacity: NormalizedF32::ONE,
                    rule: Default::default(),
                })
            }
            Brush::SweepGradient {
                c0,
                start_angle,
                end_angle,
                color_stops,
                extend,
            } => {
                if start_angle == end_angle
                    && (matches!(
                        extend,
                        skrifa::color::Extend::Reflect | skrifa::color::Extend::Repeat
                    ))
                {
                    None
                } else {
                    let sweep = SweepGradient {
                        cx: c0.x,
                        cy: c0.y,
                        start_angle,
                        end_angle,
                        stops: self.stops(color_stops),
                        spread_method: extend.to_spread_method(),
                        // COLR gradients run in the different direction
                        transform: *self.transforms.last().unwrap(),
                    };

                    Some(Fill {
                        paint: Paint::SweepGradient(sweep),
                        opacity: NormalizedF32::ONE,
                        rule: Default::default(),
                    })
                }
            }
        } {
            // The proper implementation would be to apply all clip paths and then draw the
            // whole "visible" area with the fill. However, this seems to produce artifacts in
            // Google Chrome when zooming. So instead, what we do is that we apply all clip paths except
            // for the last one, and the last one we use to actually perform the fill.
            let mut clips = self
                .clips
                .last()
                .unwrap()
                .iter()
                .map(|p| (p.clone(), FillRule::NonZero))
                .collect::<Vec<_>>();

            let filled = clips.split_off(clips.len() - 1);

            for (path, rule) in &clips {
                self.canvas_builder.push_clip_path(path, rule);
            }

            self.canvas_builder.fill_path(&filled[0].0, &fill);

            for _ in clips {
                self.canvas_builder.pop_clip_path();
            }
        }
    }

    fn push_layer(&mut self, composite_mode: CompositeMode) {
        let mode = match composite_mode {
            CompositeMode::SrcOver => BlendMode::Normal,
            CompositeMode::Screen => BlendMode::Screen,
            CompositeMode::Overlay => BlendMode::Overlay,
            CompositeMode::Darken => BlendMode::Darken,
            CompositeMode::Lighten => BlendMode::Lighten,
            CompositeMode::ColorDodge => BlendMode::ColorDodge,
            CompositeMode::ColorBurn => BlendMode::ColorBurn,
            CompositeMode::HardLight => BlendMode::HardLight,
            CompositeMode::SoftLight => BlendMode::SoftLight,
            CompositeMode::Difference => BlendMode::Difference,
            CompositeMode::Exclusion => BlendMode::Exclusion,
            CompositeMode::Multiply => BlendMode::Multiply,
            CompositeMode::HslHue => BlendMode::Hue,
            CompositeMode::HslColor => BlendMode::Color,
            CompositeMode::HslLuminosity => BlendMode::Luminosity,
            CompositeMode::HslSaturation => BlendMode::Saturation,
            _ => BlendMode::Normal,
        };
        self.canvas_builder.push_isolated();
        self.canvas_builder.push_blend_mode(mode);
    }

    fn pop_layer(&mut self) {
        self.canvas_builder.pop_blend_mode();
        self.canvas_builder.pop_isolated();
    }
}

#[cfg(test)]
mod tests {
    use crate::font::draw;

    use skrifa::GlyphId;

    use std::sync::Arc;

    #[test]
    fn colr_test() {
        let font_data =
            std::fs::read("/Users/lstampfl/Programming/GitHub/krilla/test_glyphs-glyf_colr_1.ttf")
                .unwrap();

        let glyphs = (0..=220)
            .map(|n| (GlyphId::new(n), "".to_string()))
            .collect::<Vec<_>>();

        draw(Arc::new(font_data), Some(glyphs), "colr_test");
    }

    #[test]
    fn noto_color() {
        let font_data = std::fs::read("/Library/Fonts/NotoColorEmoji-Regular.ttf").unwrap();
        draw(Arc::new(font_data), None, "colr_noto");
    }

    #[test]
    fn segoe_emoji() {
        let font_data = std::fs::read("/Library/Fonts/seguiemj.ttf").unwrap();
        draw(Arc::new(font_data), None, "colr_segoe");
    }
}
