use crate::blend_mode::BlendMode;
use crate::canvas::{Canvas, Surface};
use crate::color::Color;
use crate::font::OutlineBuilder;
use crate::paint::{LinearGradient, Paint, RadialGradient, SpreadMethod, Stop, SweepGradient};
use crate::transform::TransformWrapper;
use crate::{Fill, FillRule};
use skrifa::color::{Brush, ColorPainter, ColorStop, CompositeMode};
use skrifa::outline::DrawSettings;
use skrifa::prelude::LocationRef;
use skrifa::raw::types::BoundingBox;
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use tiny_skia_path::{FiniteF32, NormalizedF32, Path, PathBuilder, Size, Transform};

struct ColrCanvas<'a> {
    font: &'a FontRef<'a>,
    clips: Vec<Vec<Path>>,
    transforms: Vec<Transform>,
    canvases: Vec<Canvas>,
    blend_modes: Vec<BlendMode>,
    size: u16,
}

impl<'a> ColrCanvas<'a> {
    pub fn new(font_ref: &'a FontRef<'a>) -> Self {
        let size = font_ref
            .metrics(skrifa::instance::Size::unscaled(), LocationRef::default())
            .units_per_em;
        let canvas = Canvas::new(Size::from_wh(size as f32, size as f32).unwrap());

        Self {
            font: font_ref,
            transforms: vec![Transform::identity()],
            clips: vec![vec![]],
            canvases: vec![canvas],
            blend_modes: vec![],
            size,
        }
    }
}

impl ColrCanvas<'_> {
    fn palette_index_to_color(&self, palette_index: u16, alpha: f32) -> (Color, NormalizedF32) {
        if palette_index != u16::MAX {
            let color = self
                .font
                .cpal()
                .unwrap()
                .color_records_array()
                .unwrap()
                .unwrap()[palette_index as usize];

            (
                Color::new_rgb(color.red, color.green, color.blue),
                NormalizedF32::new(alpha * color.alpha as f32 / 255.0).unwrap(),
            )
        } else {
            (Color::new_rgb(0, 0, 0), NormalizedF32::new(alpha).unwrap())
        }
    }

    fn stops(&self, stops: &[ColorStop]) -> Vec<Stop> {
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

impl ColorPainter for ColrCanvas<'_> {
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
        let outline_glyphs = self.font.outline_glyphs();
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
                    x1: FiniteF32::new(p0.x).unwrap(),
                    y1: FiniteF32::new(p0.y).unwrap(),
                    x2: FiniteF32::new(p1.x).unwrap(),
                    y2: FiniteF32::new(p1.y).unwrap(),
                    stops: self.stops(color_stops),
                    spread_method: extend.to_spread_method(),
                    transform: TransformWrapper(*self.transforms.last().unwrap()),
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
                    fx: FiniteF32::new(c0.x).unwrap(),
                    fy: FiniteF32::new(c0.y).unwrap(),
                    fr: FiniteF32::new(r0).unwrap(),
                    cx: FiniteF32::new(c1.x).unwrap(),
                    cy: FiniteF32::new(c1.y).unwrap(),
                    cr: FiniteF32::new(r1).unwrap(),
                    stops: self.stops(color_stops),
                    spread_method: extend.to_spread_method(),
                    transform: TransformWrapper(*self.transforms.last().unwrap()),
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
                        cx: FiniteF32::new(c0.x).unwrap(),
                        cy: FiniteF32::new(c0.y).unwrap(),
                        start_angle: FiniteF32::new(start_angle).unwrap(),
                        end_angle: FiniteF32::new(end_angle).unwrap(),
                        stops: self.stops(color_stops),
                        spread_method: extend.to_spread_method(),
                        // COLR gradients run in the different direction
                        transform: TransformWrapper(*self.transforms.last().unwrap()),
                    };

                    Some(Fill {
                        paint: Paint::SweepGradient(sweep),
                        opacity: NormalizedF32::ONE,
                        rule: Default::default(),
                    })
                }
            }
        } {
            let canvas = self.canvases.last_mut().unwrap();

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

            let mut clipped = canvas.clipped_many(clips);

            clipped.fill_path(filled[0].0.clone(), Transform::identity(), fill);

            clipped.finish();
        }
    }

    fn push_layer(&mut self, composite_mode: CompositeMode) {
        let mode = match composite_mode {
            CompositeMode::SrcOver => BlendMode::SourceOver,
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
            CompositeMode::SrcAtop => BlendMode::SourceAtop,
            CompositeMode::DestAtop => BlendMode::DestinationAtop,
            CompositeMode::HslColor => BlendMode::Color,
            CompositeMode::HslLuminosity => BlendMode::Luminosity,
            CompositeMode::HslSaturation => BlendMode::Saturation,
            CompositeMode::Clear => BlendMode::Clear,
            CompositeMode::Src => BlendMode::Source,
            CompositeMode::Dest => BlendMode::Destination,
            CompositeMode::DestOver => BlendMode::DestinationOver,
            CompositeMode::DestIn => BlendMode::DestinationIn,
            CompositeMode::SrcIn => BlendMode::SourceIn,
            CompositeMode::SrcOut => BlendMode::SourceOut,
            CompositeMode::DestOut => BlendMode::DestinationOut,
            CompositeMode::Xor => BlendMode::Xor,
            CompositeMode::Plus => BlendMode::Plus,
            CompositeMode::Unknown => BlendMode::SourceOver,
        };
        let canvas = Canvas::new(Size::from_wh(self.size as f32, self.size as f32).unwrap());
        self.blend_modes.push(mode);
        self.canvases.push(canvas);
    }

    fn pop_layer(&mut self) {
        let draw_canvas = self.canvases.pop().unwrap();

        let canvas = self.canvases.last_mut().unwrap();
        let mut blended = canvas.blended(self.blend_modes.pop().unwrap());
        let mut isolated = blended.isolated();
        isolated.draw_canvas(draw_canvas);
        isolated.finish();
        blended.finish();
    }
}

#[cfg(test)]
mod tests {
    use crate::canvas::{Canvas, Surface};
    use crate::font::colr::ColrCanvas;
    use crate::font::draw;
    use crate::serialize::{PageSerialize, SerializeSettings};
    use skrifa::prelude::LocationRef;
    use skrifa::raw::TableProvider;
    use skrifa::{FontRef, GlyphId, MetadataProvider};

    fn single_glyph(
        font_ref: &FontRef,
        location_ref: LocationRef,
        glyph: GlyphId,
    ) -> Option<Canvas> {
        let mut colr_canvas = ColrCanvas::new(&font_ref);

        let colr_glyphs = font_ref.color_glyphs();
        if let Some(colr_glyph) = colr_glyphs.get(glyph) {
            let _ = colr_glyph.paint(location_ref, &mut colr_canvas);
        } else {
            return None;
        }
        let canvas = colr_canvas.canvases.last().unwrap().clone();
        Some(canvas)
    }

    fn draw_colr(font_ref: &FontRef, glyphs: &[u32], name: &str) {
        draw(font_ref, LocationRef::default(), glyphs, name, single_glyph);
    }

    #[test]
    fn colr_test() {
        let font_data =
            std::fs::read("/Users/lstampfl/Programming/GitHub/krilla/test_glyphs-glyf_colr_1.ttf")
                .unwrap();
        let font_ref = FontRef::from_index(&font_data, 0).unwrap();

        let glyphs = (0..=220).collect::<Vec<_>>();

        draw_colr(&font_ref, &glyphs, "colr_test");
    }

    #[test]
    fn noto_color() {
        let font_data = std::fs::read("/Library/Fonts/NotoColorEmoji-Regular.ttf").unwrap();
        let font_ref = FontRef::from_index(&font_data, 0).unwrap();

        let glyphs = (2100..2400).collect::<Vec<_>>();
        // let glyphs = (0..font_ref.maxp().unwrap().num_glyphs() as u32).collect::<Vec<_>>();

        draw_colr(&font_ref, &glyphs, "colr_noto");
    }

    #[test]
    fn segoe_emoji() {
        let font_data = std::fs::read("/Library/Fonts/seguiemj.ttf").unwrap();
        let font_ref = FontRef::from_index(&font_data, 0).unwrap();

        let glyphs = (2100..2400).collect::<Vec<_>>();
        // let glyphs = (0..=5000).collect::<Vec<_>>();

        draw_colr(&font_ref, &glyphs, "colr_segoe");
    }
}
