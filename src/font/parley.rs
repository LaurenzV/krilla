#[cfg(test)]
mod tests {
    use crate::canvas::Page;
    use crate::serialize::PageSerialize;
    use crate::Color;
    use parley::layout::Alignment;
    use parley::style::{FontFamily, FontStack, FontWeight, StyleProperty};
    use parley::swash::scale::ScaleContext;
    use parley::{FontContext, Layout, LayoutContext};

    #[test]
    fn parley_integration() {
        // The text we are going to style and lay out
        let text = String::from(
            "Some text here. Let's make it a bit longer so that line wrapping kicks in 😊. And also some اللغة العربية arabic text.",
        );

        // The display scale for HiDPI rendering
        let display_scale = 1.0;

        // The width for line wrapping
        let max_advance = Some(200.0 * display_scale);

        // Colours for rendering
        let text_color = Color::new_rgb(255, 0, 0);

        // Create a FontContext, LayoutContext and ScaleContext
        //
        // These are all intended to be constructed rarely (perhaps even once per app (or once per thread))
        // and provide caches and scratch space to avoid allocations
        let mut font_cx = FontContext::default();
        let mut layout_cx = LayoutContext::new();
        let mut scale_cx = ScaleContext::new();

        // Create a RangedBuilder
        let mut builder = layout_cx.ranged_builder(&mut font_cx, &text, display_scale);

        // Set default text colour styles (set foreground text color)
        let brush_style = StyleProperty::Brush(text_color);
        builder.push_default(&brush_style);

        // Set default font family
        let font_stack = FontStack::List(&[
            FontFamily::Named("Noto Sans"),
            FontFamily::Named("Noto Sans Arabic"),
            FontFamily::Named("Noto Color Emoji"),
        ]);
        let font_stack_style = StyleProperty::FontStack(font_stack);
        builder.push_default(&font_stack_style);
        builder.push_default(&StyleProperty::LineHeight(1.3));
        builder.push_default(&StyleProperty::FontSize(16.0));

        // Set the first 4 characters to bold
        let bold = FontWeight::new(600.0);
        let bold_style = StyleProperty::FontWeight(bold);
        builder.push(&bold_style, 0..4);

        // let color_style = StyleProperty::Brush(Color::new_rgb(255, 0,0 ));
        // builder.push(&color_style, 5..9);

        // Build the builder into a Layout
        let mut layout: Layout<Color> = builder.build();

        // Perform layout (including bidi resolution and shaping) with start alignment
        layout.break_all_lines(max_advance);
        layout.align(max_advance, Alignment::Start);

        let page_size = tiny_skia_path::Size::from_wh(200.0, 200.0).unwrap();
        let page = Page::new(page_size);
        let mut builder = page.builder();

        builder.draw_parley(&layout, &text);

        let stream = builder.finish();
        let sc = page.finish();

        let pdf = stream.serialize(sc, page_size);
        let finished = pdf.finish();
        let _ = std::fs::write(format!("out/parley.pdf"), &finished);
        let _ = std::fs::write(format!("out/parley.txt"), &finished);
    }
}
