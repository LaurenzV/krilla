mod bitmap {
    use krilla::Document;
    use krilla_macros::visreg2;

    use crate::{all_glyphs_to_pdf, NOTO_COLOR_EMOJI_CBDT};

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
    use krilla::font::GlyphId;
    use krilla::path::{Fill, Stroke};
    use krilla::surface::{Surface, TextDirection};
    use krilla::{Document, Font};
    use krilla_macros::visreg2;
    use tiny_skia_path::Point;

    use crate::{
        all_glyphs_to_pdf, blue_stroke, purple_fill, COLR_TEST_GLYPHS, NOTO_COLOR_EMOJI_COLR,
    };

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

mod svg {
    use krilla::surface::{Surface, TextDirection};
    use krilla::{Document, Font};
    use krilla_macros::visreg2;
    use tiny_skia_path::Point;

    use crate::{all_glyphs_to_pdf, purple_fill, red_fill, SVG_EXTRA, TWITTER_COLOR_EMOJI};

    #[visreg2(document, all)]
    fn twitter_color_emoji(document: &mut Document) {
        let font_data = TWITTER_COLOR_EMOJI.clone();
        all_glyphs_to_pdf(font_data, None, false, true, document);
    }

    #[visreg2(document)]
    fn twitter_color_emoji_no_color(document: &mut Document) {
        let font_data = TWITTER_COLOR_EMOJI.clone();
        all_glyphs_to_pdf(font_data, None, false, false, document);
    }

    #[visreg2]
    fn svg_extra(surface: &mut Surface) {
        let font_data = SVG_EXTRA.clone();
        let font = Font::new(font_data, 0, true).unwrap();

        surface.fill_text(
            Point::from_xy(0., 30.0),
            purple_fill(1.0),
            font.clone(),
            30.0,
            &[],
            "😀",
            false,
            TextDirection::Auto,
        );

        surface.fill_text(
            Point::from_xy(30., 30.0),
            red_fill(1.0),
            font.clone(),
            30.0,
            &[],
            "😀",
            false,
            TextDirection::Auto,
        );
    }
}

mod cid {
    use krilla::path::Fill;
    use krilla::surface::{Surface, TextDirection};
    use krilla::Font;
    use krilla_macros::{visreg, visreg2};
    use tiny_skia_path::Point;

    use crate::{LATIN_MODERN_ROMAN, NOTO_SANS, NOTO_SANS_ARABIC};

    #[visreg2(all)]
    fn cid_font_noto_sans_simple_text(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS.clone(), 0, true).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            32.0,
            &[],
            "hello world",
            false,
            TextDirection::Auto,
        );
    }

    #[visreg2(all)]
    fn cid_font_latin_modern_simple_text(surface: &mut Surface) {
        let font = Font::new(LATIN_MODERN_ROMAN.clone(), 0, true).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            32.0,
            &[],
            "hello world",
            false,
            TextDirection::Auto,
        );
    }

    #[visreg2(all)]
    fn cid_font_noto_arabic_simple_text(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS_ARABIC.clone(), 0, true).unwrap();
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font,
            32.0,
            &[],
            "مرحبا بالعالم",
            false,
            TextDirection::Auto,
        );
    }

    #[cfg(target_os = "macos")]
    #[visreg2(macos)]
    fn cid_font_true_type_collection(surface: &mut Surface) {
        let font_data: crate::Data = std::fs::read("/System/Library/Fonts/Supplemental/Songti.ttc")
            .unwrap()
            .into();
        let font_1 = Font::new(font_data.clone(), 0, true).unwrap();
        let font_2 = Font::new(font_data.clone(), 3, true).unwrap();
        let font_3 = Font::new(font_data, 6, true).unwrap();

        surface.fill_text(
            Point::from_xy(0.0, 75.0),
            Fill::default(),
            font_1.clone(),
            20.0,
            &[],
            "这是一段测试文字。",
            false,
            TextDirection::Auto,
        );
        surface.fill_text(
            Point::from_xy(0.0, 100.0),
            Fill::default(),
            font_2.clone(),
            20.0,
            &[],
            "这是一段测试文字。",
            false,
            TextDirection::Auto,
        );
        surface.fill_text(
            Point::from_xy(0.0, 125.0),
            Fill::default(),
            font_3.clone(),
            20.0,
            &[],
            "这是一段测试文字。",
            false,
            TextDirection::Auto,
        );
    }
}

mod type3 {
    use krilla::path::Fill;
    use krilla::surface::TextDirection;
    use krilla::{Font, Page};
    use krilla_macros::snapshot2;
    use tiny_skia_path::Point;

    use crate::TWITTER_COLOR_EMOJI;

    #[snapshot2(single_page, settings_1)]
    fn type3_color_glyphs(page: &mut Page) {
        let font = Font::new(TWITTER_COLOR_EMOJI.clone(), 0, true).unwrap();
        let mut surface = page.surface();

        surface.fill_text(
            Point::from_xy(0.0, 25.0),
            Fill::default(),
            font.clone(),
            25.0,
            &[],
            "😀😃",
            false,
            TextDirection::Auto,
        );
    }

    #[snapshot2(single_page, settings_17)]
    fn type3_pdf_14(page: &mut Page) {
        let font = Font::new(TWITTER_COLOR_EMOJI.clone(), 0, true).unwrap();
        let mut surface = page.surface();

        surface.fill_text(
            Point::from_xy(0.0, 25.0),
            Fill::default(),
            font.clone(),
            25.0,
            &[],
            "😀",
            false,
            TextDirection::Auto,
        );
    }
}
