use skrifa::color::{Brush, ColorPainter, ColorStop, CompositeMode};
use skrifa::outline::DrawSettings;
use skrifa::raw::types::BoundingBox;
use skrifa::raw::TableProvider;
use skrifa::MetadataProvider;
use tiny_skia_path::{Path, PathBuilder};

use crate::geom::Transform;
use crate::graphics::blend::BlendMode;
use crate::graphics::color::rgb;
use crate::graphics::paint::{
    Fill, FillRule, LinearGradient, RadialGradient, SpreadMethod, Stop, SweepGradient,
};
use crate::num::NormalizedF32;
use crate::surface::Surface;
use crate::text::outline::OutlineBuilder;
use crate::text::Font;
use crate::text::GlyphId;

pub(crate) fn has_colr_data(font: &Font, glyph: GlyphId) -> bool {
    font.font_ref()
        .color_glyphs()
        .get(glyph.to_skrifa())
        .is_some()
}

/// Draw a COLR-based glyph on a surface.
pub(crate) fn draw_glyph(
    font: Font,
    context_color: rgb::Color,
    glyph: GlyphId,
    surface: &mut Surface,
) -> Option<()> {
    // Drawing COLR glyphs is a bit tricky, because it's possible that an error
    // occurs while we are drawing, in which case we cannot revert it anymore since
    // we already drew the instructions onto the surface. Because of this, we first
    // convert the glyph into a more accessible bytecode representation and only
    // if that succeeds do we iterate over the bytecode to draw onto the canvas.

    let colr_glyphs = font.font_ref().color_glyphs();
    let colr_glyph = colr_glyphs.get(glyph.to_skrifa())?;

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

                let num_clips = clips.len();
                for path in clips {
                    surface.push_clip_path(&crate::geom::Path(path), &FillRule::NonZero);
                }

                let old_fill = surface.get_fill().cloned();
                let old_stroke = surface.get_stroke().cloned();

                surface.set_fill(Some(*fill));
                surface.set_stroke(None);

                surface.draw_path(&crate::geom::Path(filled[0].clone()));

                surface.set_fill(old_fill);
                surface.set_stroke(old_stroke);

                for _ in 0..num_clips {
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
        } else if let Some(instructions) = self.stack.pop() {
            return Some(instructions);
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
                NormalizedF32::new(alpha * color.alpha as f32 / 255.0)?,
            ))
        } else {
            Some((self.context_color, NormalizedF32::new(alpha)?))
        }
    }

    fn stops(&self, stops: &[ColorStop]) -> Option<Vec<Stop>> {
        let mut converted_stops = vec![];

        for stop in stops {
            let (color, alpha) = self.palette_index_to_color(stop.palette_index, stop.alpha)?;

            converted_stops.push(Stop {
                offset: NormalizedF32::new(stop.offset)?,
                color: color.into(),
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

    fn push_clip_glyph(&mut self, glyph_id: skrifa::GlyphId) {
        let Some(mut old) = self.clips.last().cloned() else {
            self.error = true;
            return;
        };

        let mut glyph_builder = OutlineBuilder::new();
        let outline_glyphs = self.font.outline_glyphs();
        let Some(outline_glyph) = outline_glyphs.get(glyph_id) else {
            self.error = true;
            return;
        };

        let Ok(_) = outline_glyph.draw(
            DrawSettings::unhinted(skrifa::instance::Size::unscaled(), self.font.location_ref()),
            &mut glyph_builder,
        ) else {
            self.error = true;
            return;
        };

        let Some(path) = glyph_builder
            .finish()
            .and_then(|p| p.transform(self.transforms.last()?.to_tsp()))
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
            .and_then(|p| p.transform(self.transforms.last()?.to_tsp()))
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
                    x1: p0.x,
                    y1: p0.y,
                    x2: p1.x,
                    y2: p1.y,
                    stops,
                    spread_method: extend.to_spread_method(),
                    transform,
                    anti_alias: false,
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
                    fx: c0.x,
                    fy: c0.y,
                    fr: r0,
                    cx: c1.x,
                    cy: c1.y,
                    cr: r1,
                    stops,
                    spread_method: extend.to_spread_method(),
                    transform,
                    anti_alias: false,
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

                let Some(mut transform) = self.transforms.last().copied() else {
                    self.error = true;
                    return;
                };

                // krilla sweep gradients go in a different direction than COLR, so we need
                // to invert y-axis.

                transform = transform.pre_concat(Transform::from_scale(1.0, -1.0));

                let sweep = SweepGradient {
                    cx: c0.x,
                    cy: -c0.y,
                    start_angle,
                    end_angle,
                    stops,
                    spread_method: extend.to_spread_method(),
                    transform,
                    anti_alias: false,
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
            let Some(clips) = self.clips.last().map(|paths| paths.to_vec()) else {
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
