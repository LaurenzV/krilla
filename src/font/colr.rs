use crate::canvas::Canvas;
use crate::color::Color;
use crate::font::OutlineBuilder;
use crate::paint::{LinearGradient, Paint, RadialGradient, SpreadMethod, Stop, SweepGradient};
use crate::transform::TransformWrapper;
use crate::{Fill, FillRule};
use pdf_writer::types::BlendMode;
use skrifa::color::{Brush, ColorPainter, ColorStop, CompositeMode};
use skrifa::outline::{DrawSettings, OutlinePen};
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

            let mut clipped =
                canvas.clipped_many(self.clips.last().unwrap().clone(), FillRule::NonZero);

            let mut path_builder = PathBuilder::new();
            path_builder.move_to(0.0, 0.0);
            path_builder.line_to(self.size as f32, 0.0);
            path_builder.line_to(self.size as f32, self.size as f32);
            path_builder.line_to(0.0, self.size as f32);
            path_builder.close();

            clipped.fill_path(path_builder.finish().unwrap(), Transform::identity(), fill);

            clipped.finish();
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
            CompositeMode::SrcAtop => BlendMode::Saturation,
            CompositeMode::HslColor => BlendMode::Color,
            CompositeMode::HslLuminosity => BlendMode::Luminosity,
            CompositeMode::HslSaturation => BlendMode::Saturation,
            _ => BlendMode::Normal,
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
    use crate::canvas::Canvas;
    use crate::font::colr::ColrCanvas;
    use crate::serialize::{PageSerialize, SerializeSettings};
    use skrifa::prelude::LocationRef;
    use skrifa::{FontRef, GlyphId, MetadataProvider};
    use tiny_skia_path::Size;

    fn single_glyph(font_ref: &FontRef, glyph: GlyphId) -> Canvas {
        let mut colr_canvas = ColrCanvas::new(&font_ref);

        let colr_glyphs = font_ref.color_glyphs();
        if let Some(colr_glyph) = colr_glyphs.get(glyph) {
            let _ = colr_glyph.paint(LocationRef::default(), &mut colr_canvas);
        }
        let canvas = colr_canvas.canvases.last().unwrap().clone();
        canvas
    }

    #[test]
    fn try_it() {
        let font_data =
            std::fs::read("/Users/lstampfl/Programming/GitHub/krilla/test_glyphs-glyf_colr_1.ttf")
                .unwrap();
        let font_data = std::fs::read("/Library/Fonts/seguiemj.ttf").unwrap();
        let font_ref = FontRef::from_index(&font_data, 0).unwrap();
        let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), LocationRef::default());

        let glyphs = (0u16..=220).collect::<Vec<_>>();
        let glyphs = vec![2808,2284,2285,2324,2283,2286,2339,2312,2788,2280,2288,2310,2315,2787,2321,2326,2693,2696,2785,2814,2807,3746,3017,3018,2325,3751,3016,3754,2323,2322,2815,2783,3015,3025,2282,2694,3231,3750,2289,2299,2336,3883,2314,2316,2319,2320,3749,3027,2306,2307,2308,2317,2305,2302,2313,2318,2290,2303,2298,2287,2309,2311,3013,2304,2301,2300,2812,2276,1797,1798,3875,1999,2784,1786,1787,1788,1795,1796,2697,2329,2327,2328,2330,2331,2332,2335,2334,2333,2394,2395,2396,1203,2711,2224,3973,2236,1209,2677,2685,3979,2771,3755,2777,2699,2705,1185,1191,1173,2230,1179,3869,3797,1215,1221,3967,1197,2717,2723,1227,2415,1233,3803,2828,3785,3791,3761,3773,2729,3815,3809,2457,3985,1842,2834,2000];

        let num_glyphs = glyphs.len();

        let width = 2000;

        let size = 150u32;
        let num_cols = width / size;
        let height = (num_glyphs as f32 / num_cols as f32).ceil() as u32 * size;
        let units_per_em = metrics.units_per_em as f32;
        let mut cur_point = 0;

        let mut parent_canvas = Canvas::new(Size::from_wh(width as f32, height as f32).unwrap());

        for i in glyphs.iter().copied() {
            let canvas = single_glyph(&font_ref, GlyphId::new(i));

            fn get_transform(
                cur_point: u32,
                size: u32,
                num_cols: u32,
                units_per_em: f32,
            ) -> crate::Transform {
                let el = cur_point / size;
                let col = el % num_cols;
                let row = el / num_cols;

                crate::Transform::from_row(
                    (1.0 / units_per_em) * size as f32,
                    0.0,
                    0.0,
                    (1.0 / units_per_em) * size as f32,
                    col as f32 * size as f32,
                    row as f32 * size as f32,
                )
            }

            let mut transformed = parent_canvas.transformed(
                get_transform(cur_point, size, num_cols, units_per_em).pre_concat(
                    tiny_skia_path::Transform::from_row(
                        1.0,
                        0.0,
                        0.0,
                        -1.0,
                        0.0,
                        units_per_em as f32,
                    ),
                ),
            );
            transformed.draw_canvas(canvas);
            transformed.finish();

            cur_point += size;
        }

        let pdf = parent_canvas.serialize(SerializeSettings::default());
        let finished = pdf.finish();
        let _ = std::fs::write("out/colr.pdf", &finished);
        let _ = std::fs::write("out/colr.txt", &finished);
    }
}
