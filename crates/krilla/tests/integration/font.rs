mod bitmap {
    use crate::{all_glyphs_to_pdf, NOTO_COLOR_EMOJI_CBDT};
    use krilla::Document;
    use krilla_macros::visreg2;

    #[visreg2(document, all)]
    fn noto_color_emoji_cbdt(document: &mut Document) {
        let font_data = NOTO_COLOR_EMOJI_CBDT.clone();
        all_glyphs_to_pdf(font_data, None, false, true, document);
    }

    #[cfg(target_os = "macos")]
    #[visreg2(document, all)]
    fn apple_color_emoji(document: &mut Document) {
        let font_data: crate::Data = std::fs::read("/System/Library/Fonts/Apple Color Emoji.ttc")
            .unwrap()
            .into();
        all_glyphs_to_pdf(font_data, None, false, true, document);
    }
}

mod colr {
    use crate::{
        all_glyphs_to_pdf, blue_stroke, purple_fill, COLR_TEST_GLYPHS, NOTO_COLOR_EMOJI_COLR,
    };
    use krilla::font::GlyphId;
    use krilla::path::{Fill, Stroke};
    use krilla::surface::{Surface, TextDirection};
    use krilla::{Document, Font};
    use krilla_macros::visreg2;
    use tiny_skia_path::Point;

    #[visreg2(document)]
    fn colr_test_glyphs(document: &mut Document) {
        let font_data = COLR_TEST_GLYPHS.clone();

        let glyphs = (0..=220)
            .map(|n| (GlyphId::new(n), "".to_string()))
            .collect::<Vec<_>>();

        all_glyphs_to_pdf(font_data, Some(glyphs), false, true, document);
    }

    #[visreg2]
    fn colr_context_color(surface: &mut Surface) {
        let font_data = COLR_TEST_GLYPHS.clone();
        let font = Font::new(font_data, 0, true).unwrap();

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
            TextDirection::Auto,
        );

        surface.fill_text(
            Point::from_xy(0., 50.0),
            purple_fill(1.0),
            font.clone(),
            15.0,
            &[],
            &text,
            false,
            TextDirection::Auto,
        );

        surface.fill_text(
            Point::from_xy(0., 70.0),
            purple_fill(1.0),
            font.clone(),
            15.0,
            &[],
            &text,
            true,
            TextDirection::Auto,
        );

        surface.stroke_text(
            Point::from_xy(0., 130.0),
            Stroke::default(),
            font.clone(),
            15.0,
            &[],
            &text,
            false,
            TextDirection::Auto,
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
            TextDirection::Auto,
        );

        surface.stroke_text(
            Point::from_xy(0., 170.0),
            blue_stroke(1.0),
            font.clone(),
            15.0,
            &[],
            &text,
            true,
            TextDirection::Auto,
        );
    }

    // We don't run on pdf.js because it leads to a high pixel difference in CI
    // for some reason.
    #[visreg2(document, pdfium, mupdf, pdfbox, ghostscript, poppler, quartz)]
    fn noto_color_emoji_colr(document: &mut Document) {
        let font_data = NOTO_COLOR_EMOJI_COLR.clone();
        all_glyphs_to_pdf(font_data, None, false, true, document);
    }
}
