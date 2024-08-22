// #[cfg(test)]
// mod tests {
//     use crate::document::Document;
//     use crate::font::Font;
//     use crate::rgb::{Color, Rgb};
//     use crate::serialize::SerializeSettings;
//     use crate::stream::TestGlyph;
//     use crate::{rgb, Fill, Paint};
//     use parley::layout::{Alignment, Cluster, PositionedLayoutItem};
//     use parley::style::{FontFamily, FontStack, FontWeight, StyleProperty};
//     use parley::swash::scale::ScaleContext;
//     use parley::{FontContext, Layout, LayoutContext};
//     use skrifa::instance::Location;
//     use skrifa::GlyphId;
//     use std::rc::Rc;
//     use usvg::NormalizedF32;
//
//     #[test]
//     fn parley_integration() {
//         // The text we are going to style and lay out
//         let text = String::from(
//             "Some text here. Let's make it a bit longer so that line wrapping kicks in 😊. And also some اللغة العربية arabic text.",
//         );
//
//         // The display scale for HiDPI rendering
//         let display_scale = 1.0;
//
//         // The width for line wrapping
//         let max_advance = Some(200.0 * display_scale);
//
//         // Colours for rendering
//         let text_color = rgb::Color::new(0, 0, 0);
//
//         // Create a FontContext, LayoutContext and ScaleContext
//         //
//         // These are all intended to be constructed rarely (perhaps even once per app (or once per thread))
//         // and provide caches and scratch space to avoid allocations
//         let mut font_cx = FontContext::default();
//         let mut layout_cx = LayoutContext::new();
//         let scale_cx = ScaleContext::new();
//
//         // Create a RangedBuilder
//         let mut builder = layout_cx.ranged_builder(&mut font_cx, &text, display_scale);
//
//         // Set default text colour styles (set foreground text color)
//         let brush_style = StyleProperty::Brush(text_color);
//         builder.push_default(&brush_style);
//
//         // Set default font family
//         let font_stack = FontStack::List(&[
//             FontFamily::Named("Noto Sans"),
//             FontFamily::Named("Noto Sans Arabic"),
//             FontFamily::Named("Noto Color Emoji"),
//         ]);
//         let font_stack_style = StyleProperty::FontStack(font_stack);
//         builder.push_default(&font_stack_style);
//         builder.push_default(&StyleProperty::LineHeight(1.3));
//         builder.push_default(&StyleProperty::FontSize(16.0));
//
//         // Set the first 4 characters to bold
//         let bold = FontWeight::new(600.0);
//         let bold_style = StyleProperty::FontWeight(bold);
//         builder.push(&bold_style, 0..4);
//
//         let color_style = StyleProperty::Brush(rgb::Color::new(255, 0,0 ));
//         builder.push(&color_style, 5..9);
//
//         // Build the builder into a Layout
//         let mut layout: Layout<rgb::Color> = builder.build();
//
//         // Perform layout (including bidi resolution and shaping) with start alignment
//         layout.break_all_lines(max_advance);
//         layout.align(max_advance, Alignment::Start);
//
//         // eprintln!("{:?}", layout.)
//
//         let mut last_cluster_range = None;
//
//         let mut db = Document::new(SerializeSettings::default());
//         let mut page = db.start_page(tiny_skia_path::Size::from_wh(200.0, 200.0).unwrap());
//         let mut surface = page.surface();
//         for line in layout.lines() {
//             for item in line.items() {
//                 match item {
//                     PositionedLayoutItem::GlyphRun(glyph_run) => {
//                         let style = glyph_run.style();
//                         let run = glyph_run.run();
//                         let mut run_x = glyph_run.offset();
//                         let run_y = glyph_run.baseline();
//
//                         if last_cluster_range == Some(run.cluster_range()) {
//                             continue;
//                         }
//
//                         last_cluster_range = Some(run.cluster_range());
//
//                         let font = run.font();
//                         let font_size = run.font_size();
//
//                         let (font_data, _) = font.data.clone().into_raw_parts();
//                         let krilla_font =
//                             Font::new(font_data, font.index, Location::default()).unwrap();
//
//                         for cluster in run.visual_clusters() {
//                             let text = &text[cluster.text_range()];
//
//                             for glyph in cluster.glyphs() {
//                                 let glyph_x = run_x + glyph.x;
//                                 let glyph_y = run_y - glyph.y;
//
//                                 run_x += glyph.advance;
//
//                                 surface.fill_glyph_run(
//                                     glyph_x,
//                                     glyph_y,
//                                     Fill {
//                                         paint: Paint::<rgb::Rgb>::Color(style.brush),
//                                         opacity: NormalizedF32::ONE,
//                                         rule: Default::default(),
//                                     },
//                                     [TestGlyph::new(
//                                         krilla_font.clone(),
//                                         GlyphId::new(glyph.id as u32),
//                                         glyph.advance,
//                                         glyph.x,
//                                         font_size,
//                                         text.to_string(),
//                                     )]
//                                     .into_iter()
//                                     .peekable(),
//                                 );
//                             }
//                         }
//                     }
//                     PositionedLayoutItem::InlineBox(_) => {}
//                 }
//             }
//         }
//
//         surface.finish();
//         page.finish();
//
//         let pdf = db.finish();
//         let _ = std::fs::write(format!("out/parley.pdf"), &pdf);
//         let _ = std::fs::write(format!("out/parley.txt"), &pdf);
//     }
// }
