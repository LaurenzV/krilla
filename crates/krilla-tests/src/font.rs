mod bitmap {
    use krilla::Document;
    use krilla_macros::visreg;

    use crate::{all_glyphs_to_pdf, NOTO_COLOR_EMOJI_CBDT};

    #[visreg(document, all)]
    fn font_noto_color_emoji_cbdt(document: &mut Document) {
        let font_data = NOTO_COLOR_EMOJI_CBDT.clone();
        all_glyphs_to_pdf(font_data, None, false, document);
    }

    #[cfg(target_os = "macos")]
    #[visreg(document, all)]
    fn font_apple_color_emoji(document: &mut Document) {
        let font_data: crate::Data = std::fs::read("/System/Library/Fonts/Apple Color Emoji.ttc")
            .unwrap()
            .into();
        all_glyphs_to_pdf(font_data, None, false, document);
    }
}

mod colr {
    use krilla::geom::Point;
    use krilla::paint::Stroke;
    use krilla::surface::Surface;
    use krilla::text::TextDirection;
    use krilla::text::{Font, GlyphId};
    use krilla::Document;
    use krilla_macros::visreg;

    use crate::{
        all_glyphs_to_pdf, blue_stroke, purple_fill, COLR_TEST_GLYPHS, NOTO_COLOR_EMOJI_COLR,
    };

    #[visreg(document)]
    fn font_colr_test_glyphs(document: &mut Document) {
        let font_data = COLR_TEST_GLYPHS.clone();

        let glyphs = (0..=220)
            .map(|n| (GlyphId::new(n), "".to_string()))
            .collect::<Vec<_>>();

        all_glyphs_to_pdf(font_data, Some(glyphs), false, document);
    }

    #[visreg]
    fn font_colr_context_color(surface: &mut Surface) {
        let font_data = COLR_TEST_GLYPHS.clone();
        let font = Font::new(font_data, 0).unwrap();

        let text = [
            0xf0b00, 0xf0b01, 0xf0b02, 0xf0b03, 0xf0b04, 0xf0b05, 0xf0b06, 0xf0b07,
        ]
        .into_iter()
        .map(|n| char::from_u32(n).unwrap().to_string())
        .collect::<Vec<_>>()
        .join(" ");

        surface.draw_text(
            Point::from_xy(0., 30.0),
            font.clone(),
            15.0,
            &text,
            false,
            TextDirection::Auto,
        );

        surface.set_fill(Some(purple_fill(1.0)));
        surface.draw_text(
            Point::from_xy(0., 50.0),
            font.clone(),
            15.0,
            &text,
            false,
            TextDirection::Auto,
        );

        surface.draw_text(
            Point::from_xy(0., 70.0),
            font.clone(),
            15.0,
            &text,
            true,
            TextDirection::Auto,
        );

        surface.set_fill(None);
        surface.set_stroke(Some(Stroke::default()));
        surface.draw_text(
            Point::from_xy(0., 130.0),
            font.clone(),
            15.0,
            &text,
            false,
            TextDirection::Auto,
        );

        // Since it a COLR glyph, it will still be filled, but the color should be taken from
        // the stroke.
        surface.set_stroke(Some(blue_stroke(1.0)));
        surface.draw_text(
            Point::from_xy(0., 150.0),
            font.clone(),
            15.0,
            &text,
            false,
            TextDirection::Auto,
        );

        surface.draw_text(
            Point::from_xy(0., 170.0),
            font.clone(),
            15.0,
            &text,
            true,
            TextDirection::Auto,
        );
    }

    // We don't run on pdf.js because it leads to a high pixel difference in CI
    // for some reason.
    #[visreg(document, pdfium, mupdf, pdfbox, ghostscript, poppler, quartz)]
    fn font_noto_color_emoji_colr(document: &mut Document) {
        let font_data = NOTO_COLOR_EMOJI_COLR.clone();
        all_glyphs_to_pdf(font_data, None, false, document);
    }
}

mod svg {
    use krilla::geom::Point;
    use krilla::surface::Surface;
    use krilla::text::{Font, TextDirection};
    use krilla::Document;
    use krilla_macros::visreg;

    use crate::{all_glyphs_to_pdf, purple_fill, red_fill, SVG_EXTRA, TWITTER_COLOR_EMOJI};

    #[visreg(document, all)]
    fn font_twitter_color_emoji(document: &mut Document) {
        let font_data = TWITTER_COLOR_EMOJI.clone();
        all_glyphs_to_pdf(font_data, None, false, document);
    }

    #[visreg]
    fn font_svg_extra(surface: &mut Surface) {
        let font_data = SVG_EXTRA.clone();
        let font = Font::new(font_data, 0).unwrap();

        surface.set_fill(Some(purple_fill(1.0)));
        surface.draw_text(
            Point::from_xy(0., 30.0),
            font.clone(),
            30.0,
            "😀",
            false,
            TextDirection::Auto,
        );

        surface.set_fill(Some(red_fill(1.0)));
        surface.draw_text(
            Point::from_xy(30., 30.0),
            font.clone(),
            30.0,
            "😀",
            false,
            TextDirection::Auto,
        );
    }
}

mod cid {
    use krilla::geom::Point;
    use krilla::page::Page;
    use krilla::surface::Surface;
    use krilla::text::{Font, TextDirection};
    use krilla_macros::{snapshot, visreg};

    use crate::{
        ASSETS_PATH, DEJAVU_SANS_MONO, FONT_PATH, LATIN_MODERN_ROMAN, NOTO_SANS, NOTO_SANS_ARABIC,
    };

    #[visreg(all)]
    fn font_ttf_simple_text(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS.clone(), 0).unwrap();
        surface.draw_text(
            Point::from_xy(0.0, 100.0),
            font,
            32.0,
            "hello world",
            false,
            TextDirection::Auto,
        );
    }

    #[visreg(all)]
    fn font_cff_simple_text(surface: &mut Surface) {
        let font = Font::new(LATIN_MODERN_ROMAN.clone(), 0).unwrap();
        surface.draw_text(
            Point::from_xy(0.0, 100.0),
            font,
            32.0,
            "hello world",
            false,
            TextDirection::Auto,
        );
    }

    #[visreg(all)]
    fn font_arabic_simple_text(surface: &mut Surface) {
        let font = Font::new(NOTO_SANS_ARABIC.clone(), 0).unwrap();
        surface.draw_text(
            Point::from_xy(0.0, 100.0),
            font,
            32.0,
            "مرحبا بالعالم",
            false,
            TextDirection::Auto,
        );
    }

    #[cfg(target_os = "macos")]
    #[visreg]
    fn font_ttc(surface: &mut Surface) {
        let font_data: crate::Data = std::fs::read("/System/Library/Fonts/Supplemental/Songti.ttc")
            .unwrap()
            .into();
        let font_1 = Font::new(font_data.clone(), 0).unwrap();
        let font_2 = Font::new(font_data.clone(), 3).unwrap();
        let font_3 = Font::new(font_data, 6).unwrap();

        surface.draw_text(
            Point::from_xy(0.0, 75.0),
            font_1.clone(),
            20.0,
            "这是一段测试文字。",
            false,
            TextDirection::Auto,
        );
        surface.draw_text(
            Point::from_xy(0.0, 100.0),
            font_2.clone(),
            20.0,
            "这是一段测试文字。",
            false,
            TextDirection::Auto,
        );
        surface.draw_text(
            Point::from_xy(0.0, 125.0),
            font_3.clone(),
            20.0,
            "这是一段测试文字。",
            false,
            TextDirection::Auto,
        );
    }

    // See https://github.com/typst/typst/issues/6185. On the one hand, we were not using
    // the typographic ascender/descender if available, and on the other hand we were
    // calculating the font bbox wrongly.
    #[snapshot]
    fn font_wrong_metrics(page: &mut Page) {
        let mut surface = page.surface();

        let font_data: crate::Data = std::fs::read(FONT_PATH.join("NotoSerifSC_subset1.ttf"))
            .unwrap()
            .into();
        let font = Font::new(font_data.clone(), 0).unwrap();

        surface.draw_text(
            Point::from_xy(0.0, 25.0),
            font.clone(),
            25.0,
            "智",
            false,
            TextDirection::Auto,
        );
    }

    // Follow-up to https://github.com/typst/typst/issues/6185, we also forgot to convert the
    // font bbox to font units.
    #[snapshot]
    fn font_wrong_metrics_2(page: &mut Page) {
        let mut surface = page.surface();

        let font_data = DEJAVU_SANS_MONO.clone();
        let font = Font::new(font_data.clone(), 0).unwrap();

        surface.draw_text(
            Point::from_xy(0.0, 25.0),
            font.clone(),
            25.0,
            "H",
            false,
            TextDirection::Auto,
        );
    }
}

mod type3 {
    use krilla::geom::Point;
    use krilla::page::Page;
    use krilla::text::{Font, TextDirection};
    use krilla_macros::snapshot;

    use crate::TWITTER_COLOR_EMOJI;

    #[snapshot(settings_1)]
    fn font_type3_color_glyphs(page: &mut Page) {
        let font = Font::new(TWITTER_COLOR_EMOJI.clone(), 0).unwrap();
        let mut surface = page.surface();

        surface.draw_text(
            Point::from_xy(0.0, 25.0),
            font.clone(),
            25.0,
            "😀😃",
            false,
            TextDirection::Auto,
        );
    }

    #[snapshot(settings_17)]
    fn font_type3_pdf_14(page: &mut Page) {
        let font = Font::new(TWITTER_COLOR_EMOJI.clone(), 0).unwrap();
        let mut surface = page.surface();

        surface.draw_text(
            Point::from_xy(0.0, 25.0),
            font.clone(),
            25.0,
            "😀",
            false,
            TextDirection::Auto,
        );
    }

    #[snapshot]
    fn font_type3_with_not_def(page: &mut Page) {
        let mut surface = page.surface();

        surface.draw_text(
            Point::from_xy(0.0, 100.0),
            Font::new(TWITTER_COLOR_EMOJI.clone(), 0).unwrap(),
            32.0,
            "H😄",
            false,
            TextDirection::Auto,
        );
    }
}
