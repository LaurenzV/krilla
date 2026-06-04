use std::collections::hash_map::Entry;
use std::collections::HashMap;

use fontdb::Database;
use krilla::color::rgb;
use krilla::geom::Point;
use krilla::num::NormalizedF32;
use krilla::paint::{Fill, Stroke};
use krilla::surface::Surface;
use krilla::text::{Font, GlyphId, KrillaGlyph};
use skrifa::{FontRef, MetadataProvider};
use smallvec::SmallVec;
use usvg::tiny_skia_path::Transform;
use usvg::{FontOpticalSizing, FontVariation, PaintOrder};

use crate::util::{convert_fill, convert_stroke, UsvgTransformExt};
use crate::{path, ProcessContext};

/// Render a text into a surface.
pub(crate) fn render(
    text: &usvg::Text,
    surface: &mut Surface,
    process_context: &mut ProcessContext,
) {
    for span in text.layouted() {
        if !span.visible {
            continue;
        }

        if let Some(overline) = &span.overline {
            path::render(overline, surface, process_context);
        }

        if let Some(underline) = &span.underline {
            path::render(underline, surface, process_context);
        }

        for glyph in &span.positioned_glyphs {
            // Ignore glyph if font can't be fetched.
            let Some(font) = process_context.fonts.retrieve(span, glyph.font) else {
                continue;
            };

            let upem = font.units_per_em();

            // The text transform contains the scale transform `font_size / upem`, we need to invert that
            // so we only get the raw transform to account for the glyph position, and the font size
            // is being taken care of by krilla.
            let transform = glyph.transform().pre_concat(Transform::from_scale(
                upem / span.font_size.get(),
                upem / span.font_size.get(),
            ));

            let Some(inverted) = transform.invert() else {
                continue;
            };

            // We need to apply the inverse transform to fill/stroke because we don't
            // want the paint to be affected by the transform applied to the glyph. See docs
            // of `convert_paint`.
            let fill = span
                .fill
                .as_ref()
                .map(|f| convert_fill(f, surface.stream_builder(), process_context, inverted));

            let stroke = span
                .stroke
                .as_ref()
                .map(|s| convert_stroke(s, surface.stream_builder(), process_context, inverted));

            let draw_op = |s: &mut Surface,
                           fill: Option<Fill>,
                           stroke: Option<Stroke>,
                           font: Font,
                           embed_text: bool| {
                s.set_fill(fill);
                s.set_stroke(stroke);

                s.draw_glyphs(
                    Point::from_xy(0.0, 0.0),
                    &[KrillaGlyph::new(
                        GlyphId::new(glyph.id.0 as u32),
                        // Don't care about those, since we render only one glyph.
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        0..glyph.text.len(),
                        None,
                    )],
                    font,
                    &glyph.text,
                    span.font_size.get(),
                    !embed_text,
                );
            };

            surface.push_transform(&transform.to_krilla());

            if fill.is_none() && stroke.is_none() {
                // Emulate invisible glyph by drawing it with an opacity of zero.
                draw_op(
                    surface,
                    Some(Fill {
                        paint: rgb::Color::new(0, 0, 0).into(),
                        opacity: NormalizedF32::ZERO,
                        rule: Default::default(),
                    }),
                    None,
                    font,
                    process_context.svg_settings.embed_text,
                )
            } else if matches!(span.paint_order, PaintOrder::FillAndStroke)
                || fill.is_none()
                || stroke.is_none()
            {
                draw_op(
                    surface,
                    fill,
                    stroke,
                    font.clone(),
                    process_context.svg_settings.embed_text,
                );
            } else {
                // Paint order stroke and fill, and we have BOTH, a fill and
                // stroke.

                // We always draw the text outlined in this case, so that
                // text won't be embedded twice.
                draw_op(surface, None, stroke, font.clone(), false);

                draw_op(
                    surface,
                    fill,
                    None,
                    font.clone(),
                    process_context.svg_settings.embed_text,
                );
            }

            surface.pop();
        }

        if let Some(line_through) = &span.line_through {
            path::render(line_through, surface, process_context);
        }
    }
}

/// Manages the krilla fonts used by an SVG.
pub(crate) struct Fonts<'a> {
    db: &'a mut Database,
    fonts: HashMap<FontInstance, Option<Font>>,
    supported_axes: HashMap<fontdb::ID, SmallVec<[[u8; 4]; 2]>>,
}

/// Identifies a font at specific variation coordinates.
type FontInstance = (fontdb::ID, FontVariations);
type FontVariations = SmallVec<[FontVariation; 2]>;

impl<'a> Fonts<'a> {
    pub(crate) fn new(db: &'a mut Database) -> Self {
        Self {
            db,
            fonts: HashMap::new(),
            supported_axes: HashMap::new(),
        }
    }

    /// Retrieves the font identified by `id` from the cache or loads it.
    fn retrieve(&mut self, span: &usvg::layout::Span, id: fontdb::ID) -> Option<Font> {
        let variations = self.resolve_variations(span, id);
        match self.fonts.entry((id, variations)) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let font = if let Some((font_data, index)) =
                    unsafe { self.db.make_shared_face_data(id) }
                {
                    let variations = &entry.key().1;
                    let coords: SmallVec<[_; 2]> = variations
                        .iter()
                        .map(|var| (krilla::text::Tag::new(&var.tag), var.value))
                        .collect();
                    Font::new_variable(font_data.into(), index, &coords)
                } else {
                    None
                };

                entry.insert(font.clone());
                font
            }
        }
    }

    /// Resolves which variations are applicable for the given font (filtering out undefined axis
    /// values and taking into account optical sizing).
    fn resolve_variations(&mut self, span: &usvg::layout::Span, id: fontdb::ID) -> FontVariations {
        let supported_axes = self.supported_axes.entry(id).or_insert_with(|| {
            self.db
                .with_face_data(id, |data, index| {
                    FontRef::from_index(data, index)
                        .into_iter()
                        .flat_map(|font| font.axes().iter())
                        .map(|axis| axis.tag().to_be_bytes())
                        .collect()
                })
                .unwrap_or_default()
        });
        let is_supported = |tag| supported_axes.contains(tag);

        let mut variations: FontVariations = span
            .variations
            .iter()
            .filter(|var| is_supported(&var.tag))
            .copied()
            .collect();

        // Set the font size for optical sizing if desired, supported, and not manually set.
        const OPSZ: &[u8; 4] = b"opsz";
        if span.font_optical_sizing == FontOpticalSizing::Auto
            && is_supported(OPSZ)
            && !variations.iter().any(|v| v.tag == *OPSZ)
        {
            variations.push(FontVariation {
                tag: *OPSZ,
                value: span.font_size.get(),
            });
        }

        variations
    }
}
