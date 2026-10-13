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
    let spans = text.layouted();
    let mut consumed = 0;
    for (index, span) in spans.iter().enumerate() {
        if index < consumed {
            continue;
        }
        if !span.visible {
            continue;
        }

        if let Some(overline) = &span.overline {
            path::render(overline, surface, process_context);
        }

        if let Some(underline) = &span.underline {
            path::render(underline, surface, process_context);
        }

        let mut end = index + 1;
        while end < spans.len() && compatible_solid_spans(span, &spans[end]) {
            end += 1;
        }
        if render_solid_text_run(&spans[index..end], surface, process_context) {
            consumed = end;
            if let Some(line_through) = &span.line_through {
                path::render(line_through, surface, process_context);
            }
            continue;
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

fn compatible_solid_spans(first: &usvg::layout::Span, next: &usvg::layout::Span) -> bool {
    next.visible
        && first.stroke.is_none()
        && next.stroke.is_none()
        && first.overline.is_none()
        && next.overline.is_none()
        && first.underline.is_none()
        && next.underline.is_none()
        && first.line_through.is_none()
        && next.line_through.is_none()
        && first.font_size == next.font_size
        && first.font_optical_sizing == next.font_optical_sizing
        && first.variations == next.variations
        && match (first.fill.as_ref(), next.fill.as_ref()) {
            (Some(a), Some(b)) => {
                matches!((a.paint(), b.paint()), (usvg::Paint::Color(c), usvg::Paint::Color(d)) if c == d)
                    && a.opacity() == b.opacity()
                    && a.rule() == b.rule()
            }
            _ => false,
        }
}

/// Coalesce compatible consecutive solid-fill glyphs in one PDF text operation. Separate transformed
/// operations make extractors insert spaces inside otherwise contiguous words.
/// Stroked/complex paint retains its per-glyph paint order.
fn render_solid_text_run(
    spans: &[usvg::layout::Span],
    surface: &mut Surface,
    process_context: &mut ProcessContext,
) -> bool {
    let span = &spans[0];
    if span.stroke.is_some()
        || !span
            .fill
            .as_ref()
            .is_some_and(|fill| matches!(fill.paint(), usvg::Paint::Color(_)))
    {
        return false;
    }
    let source: Vec<_> = spans
        .iter()
        .flat_map(|span| &span.positioned_glyphs)
        .collect();
    let size = span.font_size.get();
    let mut index = 0;
    while index < source.len() {
        let first = &source[index];
        let Some(font) = process_context.fonts.retrieve(span, first.font) else {
            index += 1;
            continue;
        };
        let scale = Transform::from_scale(font.units_per_em() / size, font.units_per_em() / size);
        let base = first.transform().pre_concat(scale);
        let Some(inverse) = base.invert() else {
            index += 1;
            continue;
        };
        let mut end = index + 1;
        while end < source.len() && source[end].font == first.font {
            let next = source[end].transform().pre_concat(scale);
            if (next.sx, next.kx, next.ky, next.sy) != (base.sx, base.kx, base.ky, base.sy) {
                break;
            }
            end += 1;
        }
        let positions: Vec<_> = source[index..end]
            .iter()
            .map(|glyph| {
                let transform = glyph.transform().pre_concat(scale);
                (
                    inverse.sx * transform.tx + inverse.kx * transform.ty + inverse.tx,
                    inverse.ky * transform.tx + inverse.sy * transform.ty + inverse.ty,
                )
            })
            .collect();
        let mut text = String::new();
        let glyphs: Vec<_> = source[index..end]
            .iter()
            .enumerate()
            .map(|(i, glyph)| {
                let start = text.len();
                text.push_str(&glyph.text);
                let advance = positions
                    .get(i + 1)
                    .map_or(0.0, |next| (next.0 - positions[i].0) / size);
                KrillaGlyph::new(
                    GlyphId::new(glyph.id.0 as u32),
                    advance,
                    0.0,
                    -positions[i].1 / size,
                    0.0,
                    start..text.len(),
                    None,
                )
            })
            .collect();
        let fill = span
            .fill
            .as_ref()
            .map(|fill| convert_fill(fill, surface.stream_builder(), process_context, inverse));
        surface.push_transform(&base.to_krilla());
        surface.set_fill(fill);
        surface.set_stroke(None);
        surface.draw_glyphs(
            Point::from_xy(positions[0].0, 0.0),
            &glyphs,
            font,
            &text,
            size,
            !process_context.svg_settings.embed_text,
        );
        surface.pop();
        index = end;
    }
    true
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use krilla::geom::Size;
    use krilla::page::PageSettings;
    use krilla::Document;

    use crate::{SurfaceExt, SvgSettings};

    #[test]
    fn svg_words_remain_contiguous_in_extracted_text() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80">
            <text x="10" y="30" font-family="Noto Sans" font-size="18">Vector table 0.125</text>
        </svg>"#;
        let text = extract_svg_text(svg);
        assert!(text.contains("Vector table 0.125"), "{text:?}");
    }

    #[test]
    fn svg_horizontally_positioned_and_rotated_words_remain_contiguous() {
        for content in [
            r#"<text x="10 22 34 46 58 70" y="30">Amount</text>"#,
            r#"<text x="10" y="30" transform="rotate(12 10 30)">Amount</text>"#,
        ] {
            let svg = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80"><g font-family="Noto Sans" font-size="18">{content}</g></svg>"#
            );
            let text = extract_svg_text(svg.as_bytes());
            assert!(text.contains("Amount"), "{content}: {text:?}");
        }
    }

    #[test]
    fn svg_unicode_words_keep_their_original_text() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80"><text x="10" y="30" font-family="Noto Sans" font-size="18">Καλημέρα Привет</text></svg>"#;
        let text = extract_svg_text(svg.as_bytes());
        assert!(text.contains("Καλημέρα Привет"), "{text:?}");
    }

    fn extract_svg_text(svg: &[u8]) -> String {
        let mut db = fontdb::Database::new();
        db.load_font_data(
            include_bytes!("../../../assets/svg_fonts/NotoSans-Regular.ttf").to_vec(),
        );

        let tree = usvg::Tree::from_data(
            svg,
            &usvg::Options {
                fontdb: Arc::new(db),
                ..Default::default()
            },
        )
        .unwrap();
        let mut document = Document::new();
        let size = Size::from_wh(300.0, 80.0).unwrap();
        let mut page = document.start_page_with(PageSettings::new(size));
        let mut surface = page.surface();
        surface.draw_svg(&tree, size, SvgSettings::default());
        surface.finish();
        page.finish();
        let pdf = document.finish().unwrap();
        pdf_extract::extract_text_from_mem(&pdf).unwrap()
    }
}
