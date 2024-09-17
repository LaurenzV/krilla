//! Drawing COLR-based glyphs to a surface.

use crate::font::{Font, OutlineBuilder, OutlineMode};
use crate::object::color::rgb;
use crate::paint::{LinearGradient, RadialGradient, SpreadMethod, Stop, SweepGradient};
use crate::path::{Fill, FillRule};
use crate::surface::Surface;
use crate::util::{F32Wrapper, TransformWrapper};
use pdf_writer::types::BlendMode;
use skrifa::color::{Brush, ColorPainter, ColorStop, CompositeMode};
use skrifa::outline::DrawSettings;
use skrifa::prelude::LocationRef;
use skrifa::raw::types::BoundingBox;
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider};
use tiny_skia_path::{NormalizedF32, Path, PathBuilder, Transform};

/// Draw a COLR-based glyph on a surface.
pub fn draw_glyph(
    font: Font,
    glyph: GlyphId,
    outline_mode: &OutlineMode,
    surface: &mut Surface,
) -> Option<()> {
    // Drawing COLR glyphs is a bit tricky, because it's possible that an error
    // occurs while we are drawing, in which case we cannot revert it anymore since
    // we already drew the instructions onto the surface. Because of this, we first
    // convert the glyph into a more accessible bytecode representation and only
    // if that succeeds do we iterate over the bytecode to draw onto the canvas.

    // TODO: Also support CMYK?
    let context_color = match outline_mode {
        OutlineMode::Fill(f) => f.paint.as_rgb(),
        OutlineMode::Stroke(s) => s.paint.as_rgb(),
    }
    .unwrap_or(rgb::Color::black());

    let colr_glyphs = font.font_ref().color_glyphs();
    let colr_glyph = colr_glyphs.get(glyph)?;

    let mut colr_canvas = ColrBuilder::new(font.clone(), context_color);
    colr_glyph
        .paint(font.location_ref(), &mut colr_canvas)
        .ok()?;
    let instructions = colr_canvas.finish()?;

    surface.push_transform(&Transform::from_scale(1.0, -1.0));
    interpret(instructions, surface);
    surface.pop();

    Some(())
}

// Interpret the glyph bytecode
fn interpret(instructions: Vec<Instruction>, surface: &mut Surface) {
    for instruction in instructions {
        match instruction {
            Instruction::Layer(blend, instructions) => {
                surface.push_blend_mode(blend);
                surface.push_isolated();
                interpret(instructions, surface);
                surface.pop();
                surface.pop();
            }
            Instruction::Filled(fill, mut clips) => {
                let filled = clips.split_off(clips.len() - 1);

                for path in &clips {
                    surface.push_clip_path(path, &FillRule::NonZero);
                }

                surface.fill_path(&filled[0], *fill);

                for _ in clips {
                    surface.pop();
                }
            }
        }
    }
}

/// The context necessary for creating the bytecode of a COLR-based glyph.
struct ColrBuilder {
    font: Font,
    context_color: rgb::Color,
    clips: Vec<Vec<Path>>,
    stack: Vec<Vec<Instruction>>,
    layers: Vec<BlendMode>,
    transforms: Vec<Transform>,
    error: bool,
}

/// A bytecode instruction for drawing a COLR glyph.
enum Instruction {
    Layer(BlendMode, Vec<Instruction>),
    Filled(Box<Fill>, Vec<Path>),
}

impl ColrBuilder {
    pub fn new(font: Font, context_color: rgb::Color) -> Self {
        Self {
            font,
            context_color,
            stack: vec![vec![]],
            transforms: vec![Transform::identity()],
            clips: vec![vec![]],
            layers: vec![],
            error: false,
        }
    }

    pub fn finish(mut self) -> Option<Vec<Instruction>> {
        if self.error {
            return None;
        } else {
            if let Some(instructions) = self.stack.pop() {
                return Some(instructions);
            }
        }

        None
    }
}

impl ColrBuilder {
    fn palette_index_to_color(
        &self,
        palette_index: u16,
        alpha: f32,
    ) -> Option<(rgb::Color, NormalizedF32)> {
        if palette_index != u16::MAX {
            let color = self
                .font
                .font_ref()
                .cpal()
                .ok()?
                .color_records_array()?
                .ok()?[palette_index as usize];

            Some((
                rgb::Color::new(color.red, color.green, color.blue),
                NormalizedF32::new(alpha * color.alpha as f32 / 255.0).unwrap(),
            ))
        } else {
            Some((self.context_color, NormalizedF32::new(alpha).unwrap()))
        }
    }

    fn stops(&self, stops: &[ColorStop]) -> Option<Vec<Stop<rgb::Color>>> {
        let mut converted_stops = vec![];

        for stop in stops {
            let (color, alpha) = self.palette_index_to_color(stop.palette_index, stop.alpha)?;

            converted_stops.push(Stop {
                offset: NormalizedF32::new(stop.offset).unwrap(),
                color,
                opacity: alpha,
            })
        }

        Some(converted_stops)
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

impl ColorPainter for ColrBuilder {
    fn push_transform(&mut self, transform: skrifa::color::Transform) {
        let Some(last_transform) = self.transforms.last() else {
            self.error = true;
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
        let Some(_) = self.transforms.pop() else {
            self.error = true;
            return;
        };
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        let Some(mut old) = self.clips.last().cloned() else {
            self.error = true;
            return;
        };

        let mut glyph_builder = OutlineBuilder(PathBuilder::new());
        let outline_glyphs = self.font.font_ref().outline_glyphs();
        let Some(outline_glyph) = outline_glyphs.get(glyph_id) else {
            self.error = true;
            return;
        };

        let Ok(_) = outline_glyph.draw(
            DrawSettings::unhinted(skrifa::instance::Size::unscaled(), LocationRef::default()),
            &mut glyph_builder,
        ) else {
            self.error = true;
            return;
        };

        let Some(path) = glyph_builder
            .finish()
            .and_then(|p| p.transform(*self.transforms.last()?))
        else {
            self.error = true;
            return;
        };

        old.push(path);

        self.clips.push(old);
    }

    fn push_clip_box(&mut self, clip_box: BoundingBox<f32>) {
        let Some(mut old) = self.clips.last().cloned() else {
            self.error = true;
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
            self.error = true;
            return;
        };
        old.push(path);

        self.clips.push(old);
    }

    fn pop_clip(&mut self) {
        let Some(_) = self.clips.pop() else {
            self.error = true;
            return;
        };
    }

    fn fill(&mut self, brush: Brush<'_>) {
        if let Some(fill) = match brush {
            Brush::Solid {
                palette_index,
                alpha,
            } => {
                let (color, alpha) = match self.palette_index_to_color(palette_index, alpha) {
                    Some(c) => c,
                    None => {
                        self.error = true;
                        return;
                    }
                };
                Some(Fill {
                    paint: color.into(),
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
                    Some(s) => s,
                    None => {
                        self.error = true;
                        return;
                    }
                };

                let Some(transform) = self.transforms.last().copied() else {
                    self.error = true;
                    return;
                };

                let linear = LinearGradient {
                    x1: F32Wrapper(p0.x),
                    y1: F32Wrapper(p0.y),
                    x2: F32Wrapper(p1.x),
                    y2: F32Wrapper(p1.y),
                    stops: stops.into(),
                    spread_method: extend.to_spread_method(),
                    transform: TransformWrapper(transform),
                };

                Some(Fill {
                    paint: linear.into(),
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
                    Some(s) => s,
                    None => {
                        self.error = true;
                        return;
                    }
                };

                let Some(transform) = self.transforms.last().copied() else {
                    self.error = true;
                    return;
                };

                let radial = RadialGradient {
                    fx: F32Wrapper(c0.x),
                    fy: F32Wrapper(c0.y),
                    fr: F32Wrapper(r0),
                    cx: F32Wrapper(c1.x),
                    cy: F32Wrapper(c1.y),
                    cr: F32Wrapper(r1),
                    stops: stops.into(),
                    spread_method: extend.to_spread_method(),
                    transform: TransformWrapper(transform),
                };

                Some(Fill {
                    paint: radial.into(),
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
                    Some(s) => s,
                    None => {
                        self.error = true;
                        return;
                    }
                };

                let Some(transform) = self.transforms.last().copied() else {
                    self.error = true;
                    return;
                };

                let sweep = SweepGradient {
                    cx: F32Wrapper(c0.x),
                    cy: F32Wrapper(c0.y),
                    start_angle: F32Wrapper(start_angle),
                    end_angle: F32Wrapper(end_angle),
                    stops: stops.into(),
                    spread_method: extend.to_spread_method(),
                    transform: TransformWrapper(transform),
                };

                Some(Fill {
                    paint: sweep.into(),
                    opacity: NormalizedF32::ONE,
                    rule: Default::default(),
                })
            }
        } {
            // The proper implementation would be to apply all clip paths and then draw the
            // whole "visible" area with the fill. However, this seems to produce artifacts in
            // Google Chrome when zooming. So instead, what we do is that we apply all clip paths except
            // for the last one, and the last one we use to actually perform the fill.
            let Some(clips) = self
                .clips
                .last()
                .map(|paths| paths.iter().map(|p| p.clone()).collect::<Vec<_>>())
            else {
                self.error = true;
                return;
            };

            let Some(stack) = self.stack.last_mut() else {
                self.error = true;
                return;
            };

            stack.push(Instruction::Filled(Box::new(fill), clips));
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

        self.layers.push(mode);
        self.stack.push(vec![]);
    }

    fn pop_layer(&mut self) {
        let (Some(blend), Some(instructions)) = (self.layers.pop(), self.stack.pop()) else {
            self.error = true;
            return;
        };

        let Some(stack) = self.stack.last_mut() else {
            self.error = true;
            return;
        };

        stack.push(Instruction::Layer(blend, instructions));
    }
}

#[cfg(test)]
mod tests {

    use crate::document::Document;
    use crate::font::Font;
    use crate::path::{Fill, Stroke};
    use crate::surface::Surface;
    use crate::tests::{
        all_glyphs_to_pdf, blue_stroke, purple_fill, COLR_TEST_GLYPHS, NOTO_COLOR_EMOJI_COLR,
    };
    use krilla_macros::visreg;
    use skrifa::GlyphId;
    use tiny_skia_path::Point;

    #[visreg(document)]
    fn colr_test_glyphs(document: &mut Document) {
        let font_data = COLR_TEST_GLYPHS.clone();

        let glyphs = (0..=220)
            .map(|n| (GlyphId::new(n), "".to_string()))
            .collect::<Vec<_>>();

        all_glyphs_to_pdf(font_data, Some(glyphs), false, document);
    }

    #[visreg]
    fn colr_context_color(surface: &mut Surface) {
        let font_data = COLR_TEST_GLYPHS.clone();
        let font = Font::new(font_data, 0, vec![]).unwrap();

        let text = [
            0xf0b00, 0xf0b01, 0xf0b02, 0xf0b03, 0xf0b04, 0xf0b05, 0xf0b06, 0xf0b07,
        ]
        .into_iter()
        .map(|n| char::from_u32(n).unwrap().to_string())
        .collect::<Vec<_>>()
        .join(" ");

        surface.fill_text(
            Point::from_xy(0., 30.0),
            Fill::default(),
            font.clone(),
            15.0,
            &[],
            &text,
            false,
            None,
        );

        surface.fill_text(
            Point::from_xy(0., 50.0),
            purple_fill(1.0),
            font.clone(),
            15.0,
            &[],
            &text,
            false,
            None,
        );

        surface.fill_text(
            Point::from_xy(0., 70.0),
            purple_fill(1.0),
            font.clone(),
            15.0,
            &[],
            &text,
            true,
            None,
        );

        surface.stroke_text(
            Point::from_xy(0., 130.0),
            Stroke::default(),
            font.clone(),
            15.0,
            &[],
            &text,
            false,
            None,
        );

        // Since it a COLR glyph, it will still be filled, but the color should be taken from
        // the stroke.
        surface.stroke_text(
            Point::from_xy(0., 150.0),
            blue_stroke(1.0),
            font.clone(),
            15.0,
            &[],
            &text,
            false,
            None,
        );

        surface.stroke_text(
            Point::from_xy(0., 170.0),
            blue_stroke(1.0),
            font.clone(),
            15.0,
            &[],
            &text,
            true,
            None,
        );
    }

    // We don't run on pdf.js because it leads to a high pixel difference in CI
    // for some reason.
    #[visreg(document, pdfium, mupdf, pdfbox, ghostscript, poppler, quartz)]
    fn noto_color_emoji_colr(document: &mut Document) {
        let font_data = NOTO_COLOR_EMOJI_COLR.clone();
        all_glyphs_to_pdf(font_data, None, false, document);
    }
}
