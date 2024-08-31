use crate::error::{KrillaError, KrillaResult};
use crate::font::{Font, OutlineBuilder};
use crate::object::color::rgb;
use crate::object::color::rgb::Rgb;
use crate::paint::{LinearGradient, Paint, RadialGradient, SpreadMethod, Stop, SweepGradient};
use crate::path::{Fill, FillRule};
use crate::surface::Surface;
use pdf_writer::types::BlendMode;
use skrifa::color::{Brush, ColorPainter, ColorStop, CompositeMode};
use skrifa::outline::DrawSettings;
use skrifa::prelude::LocationRef;
use skrifa::raw::types::BoundingBox;
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::{NormalizedF32, Path, PathBuilder, Transform};

/// Draw a COLR-based glyph on a surface.
pub fn draw_glyph(font: Font, glyph: GlyphId, surface: &mut Surface) -> KrillaResult<Option<()>> {
    let colr_glyphs = font.font_ref().color_glyphs();

    if let Some(colr_glyph) = colr_glyphs.get(glyph) {
        surface.push_transform(&Transform::from_scale(1.0, -1.0));

        let mut colr_canvas = ColrCanvas::new(font.clone(), surface);
        colr_glyph
            .paint(font.location_ref(), &mut colr_canvas)
            .map_err(|_| KrillaError::GlyphDrawing("failed to draw colr glyph".to_string()))?;

        surface.pop();

        return Ok(Some(()));
    } else {
        return Ok(None);
    }
}

/// The context necessary for drawing a COLR-based glyph.
struct ColrCanvas<'a, 'b> {
    font: Font,
    clips: Vec<Vec<Path>>,
    canvas_builder: &'b mut Surface<'a>,
    transforms: Vec<Transform>,
    error: KrillaResult<()>,
}

impl<'a, 'b> ColrCanvas<'a, 'b> {
    pub fn new(font: Font, canvas_builder: &'b mut Surface<'a>) -> Self {
        Self {
            font,
            canvas_builder,
            transforms: vec![Transform::identity()],
            clips: vec![vec![]],
            error: Ok(()),
        }
    }
}

impl<'a, 'b> ColrCanvas<'a, 'b> {
    fn palette_index_to_color(
        &self,
        palette_index: u16,
        alpha: f32,
    ) -> KrillaResult<(rgb::Color, NormalizedF32)> {
        if palette_index != u16::MAX {
            let color = self
                .font
                .font_ref()
                .cpal()
                .map_err(|_| KrillaError::GlyphDrawing("missing cpal table".to_string()))?
                .color_records_array()
                .ok_or(KrillaError::GlyphDrawing(
                    "missing color records array in cpal table".to_string(),
                ))?
                .map_err(|_| {
                    KrillaError::GlyphDrawing("error while reading cpal table".to_string())
                })?[palette_index as usize];

            Ok((
                rgb::Color::new(color.red, color.green, color.blue),
                NormalizedF32::new(alpha * color.alpha as f32 / 255.0).unwrap(),
            ))
        } else {
            Ok((rgb::Color::new(0, 0, 0), NormalizedF32::new(alpha).unwrap()))
        }
    }

    fn stops(&self, stops: &[ColorStop]) -> KrillaResult<Vec<Stop<Rgb>>> {
        let mut converted_stops = vec![];

        for stop in stops {
            let (color, alpha) = self.palette_index_to_color(stop.palette_index, stop.alpha)?;

            converted_stops.push(Stop {
                offset: NormalizedF32::new(stop.offset).unwrap(),
                color,
                opacity: alpha,
            })
        }

        Ok(converted_stops)
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
        let Some(last_transform) = self.transforms.last() else {
            self.error = Err(KrillaError::GlyphDrawing(
                "encountered imbalanced transform".to_string(),
            ));
            return;
        };

        let new_transform = last_transform.pre_concat(Transform::from_row(
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
        let Some(mut old) = self.clips.last().cloned() else {
            self.error = Err(KrillaError::GlyphDrawing(
                "encountered imbalanced clip".to_string(),
            ));
            return;
        };

        let mut glyph_builder = OutlineBuilder(PathBuilder::new());
        let outline_glyphs = self.font.font_ref().outline_glyphs();
        let Some(outline_glyph) = outline_glyphs.get(glyph_id) else {
            self.error = Err(KrillaError::GlyphDrawing(
                "missing outline glyph".to_string(),
            ));
            return;
        };

        let drawn_outline_glyph = outline_glyph
            .draw(
                DrawSettings::unhinted(skrifa::instance::Size::unscaled(), LocationRef::default()),
                &mut glyph_builder,
            )
            .map_err(|_| KrillaError::GlyphDrawing("failed to draw outline glyph".to_string()));

        match drawn_outline_glyph {
            Ok(_) => {}
            Err(e) => {
                self.error = Err(e);
                return;
            }
        }

        let Some(path) = glyph_builder
            .finish()
            .and_then(|p| p.transform(*self.transforms.last()?))
        else {
            self.error = Err(KrillaError::GlyphDrawing(
                "failed to build glyph path".to_string(),
            ));
            return;
        };

        old.push(path);

        self.clips.push(old);
    }

    fn push_clip_box(&mut self, clip_box: BoundingBox<f32>) {
        let Some(mut old) = self.clips.last().cloned() else {
            self.error = Err(KrillaError::GlyphDrawing(
                "encountered imbalanced clip".to_string(),
            ));
            return;
        };

        let mut path_builder = PathBuilder::new();
        path_builder.move_to(clip_box.x_min, clip_box.y_min);
        path_builder.line_to(clip_box.x_min, clip_box.y_max);
        path_builder.line_to(clip_box.x_max, clip_box.y_max);
        path_builder.line_to(clip_box.x_max, clip_box.y_min);
        path_builder.close();

        let Some(path) = path_builder
            .finish()
            .and_then(|p| p.transform(*self.transforms.last()?))
        else {
            self.error = Err(KrillaError::GlyphDrawing(
                "failed to build glyph path".to_string(),
            ));
            return;
        };
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
                let (color, alpha) = match self.palette_index_to_color(palette_index, alpha) {
                    Ok(c) => c,
                    Err(e) => {
                        self.error = Err(e);
                        return;
                    }
                };
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
                let stops = match self.stops(color_stops) {
                    Ok(s) => s,
                    Err(e) => {
                        self.error = Err(e);
                        return;
                    }
                };

                let Some(transform) = self.transforms.last().copied() else {
                    self.error = Err(KrillaError::GlyphDrawing(
                        "encountered imbalanced transform".to_string(),
                    ));
                    return;
                };

                let linear = LinearGradient {
                    x1: p0.x,
                    y1: p0.y,
                    x2: p1.x,
                    y2: p1.y,
                    stops,
                    spread_method: extend.to_spread_method(),
                    transform,
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
                let stops = match self.stops(color_stops) {
                    Ok(s) => s,
                    Err(e) => {
                        self.error = Err(e);
                        return;
                    }
                };

                let Some(transform) = self.transforms.last().copied() else {
                    self.error = Err(KrillaError::GlyphDrawing(
                        "encountered imbalanced transform".to_string(),
                    ));
                    return;
                };

                let radial = RadialGradient {
                    fx: c0.x,
                    fy: c0.y,
                    fr: r0,
                    cx: c1.x,
                    cy: c1.y,
                    cr: r1,
                    stops,
                    spread_method: extend.to_spread_method(),
                    transform,
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
                let stops = match self.stops(color_stops) {
                    Ok(s) => s,
                    Err(e) => {
                        self.error = Err(e);
                        return;
                    }
                };

                let Some(transform) = self.transforms.last().copied() else {
                    self.error = Err(KrillaError::GlyphDrawing(
                        "encountered imbalanced transform".to_string(),
                    ));
                    return;
                };

                let sweep = SweepGradient {
                    cx: c0.x,
                    cy: c0.y,
                    start_angle,
                    end_angle,
                    stops,
                    spread_method: extend.to_spread_method(),
                    transform,
                };

                Some(Fill {
                    paint: Paint::SweepGradient(sweep),
                    opacity: NormalizedF32::ONE,
                    rule: Default::default(),
                })
            }
        } {
            // The proper implementation would be to apply all clip paths and then draw the
            // whole "visible" area with the fill. However, this seems to produce artifacts in
            // Google Chrome when zooming. So instead, what we do is that we apply all clip paths except
            // for the last one, and the last one we use to actually perform the fill.
            let Some(mut clips) = self.clips.last().map(|paths| {
                paths
                    .iter()
                    .map(|p| (p.clone(), FillRule::NonZero))
                    .collect::<Vec<_>>()
            }) else {
                self.error = Err(KrillaError::GlyphDrawing(
                    "failed to apply fill glyph".to_string(),
                ));
                return;
            };

            let filled = clips.split_off(clips.len() - 1);

            for (path, rule) in &clips {
                self.canvas_builder.push_clip_path(path, rule);
            }

            self.canvas_builder.fill_path(&filled[0].0, fill);

            for _ in clips {
                self.canvas_builder.pop();
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
        self.canvas_builder.push_blend_mode(mode);
        self.canvas_builder.push_isolated();
    }

    fn pop_layer(&mut self) {
        self.canvas_builder.pop();
        self.canvas_builder.pop();
    }
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::tests::{all_glyphs_to_pdf, COLR_TEST_GLYPHS, NOTO_COLOR_EMOJI_COLR};
    use krilla_macros::visreg;
    use skrifa::GlyphId;

    #[visreg(document, settings_3)]
    fn colr_test_glyphs(document: &mut Document) {
        let font_data = COLR_TEST_GLYPHS.clone();

        let glyphs = (0..=220)
            .map(|n| (GlyphId::new(n), "".to_string()))
            .collect::<Vec<_>>();

        all_glyphs_to_pdf(font_data, Some(glyphs), false, document);
    }

    #[visreg(document)]
    fn noto_color_emoji_colr(document: &mut Document) {
        let font_data = NOTO_COLOR_EMOJI_COLR.clone();
        all_glyphs_to_pdf(font_data, None, false, document);
    }
}
