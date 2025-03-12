use krilla_macros::{snapshot2, visreg2};
use tiny_skia_path::{Point, Size, Transform};

use crate::SvgSettings;
use crate::{
    blue_fill, blue_stroke, red_fill, red_stroke, stops_with_3_solid_1, FONTDB,
    NOTO_COLOR_EMOJI_COLR, NOTO_SANS, NOTO_SANS_CJK, NOTO_SANS_DEVANAGARI, SVGS_PATH,
};
use krilla::font::Font;
use krilla::page::Page;
use krilla::paint::{LinearGradient, Paint, SpreadMethod};
use krilla::path::{Fill, Stroke};
use krilla::surface::Surface;
use krilla::surface::TextDirection;

#[visreg2]
fn text_direction_ltr(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS_CJK.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(0.0, 100.0),
        Fill::default(),
        font,
        20.0,
        &[],
        "你好这是一段则是文字",
        false,
        TextDirection::LeftToRight,
    );
}

#[visreg2]
fn text_direction_rtl(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS_CJK.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(0.0, 100.0),
        Fill::default(),
        font,
        20.0,
        &[],
        "你好这是一段则是文字",
        false,
        TextDirection::RightToLeft,
    );
}

#[visreg2]
fn text_direction_ttb(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS_CJK.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(100.0, 0.0),
        Fill::default(),
        font,
        20.0,
        &[],
        "你好这是一段则是文字",
        false,
        TextDirection::TopToBottom,
    );
}

#[visreg2]
fn text_direction_btt(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS_CJK.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(100.0, 0.0),
        Fill::default(),
        font,
        20.0,
        &[],
        "你好这是一段则是文字",
        false,
        TextDirection::BottomToTop,
    );
}

fn simple_text_impl(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_text(
        Point::from_xy(0.0, 50.0),
        Fill::default(),
        Font::new(NOTO_SANS.clone(), 0, true).unwrap(),
        16.0,
        &[],
        "A line of text.",
        false,
        TextDirection::Auto,
    );
}

#[snapshot2(single_page)]
fn simple_text(page: &mut Page) {
    simple_text_impl(page);
}

#[snapshot2(single_page, settings_25)]
fn simple_text_pdf20(page: &mut Page) {
    // The main purpose of this test is to ensure that the fonts without CIDSet are
    // still written properly for PDF 2.0.
    simple_text_impl(page);
}

#[snapshot2(single_page)]
fn complex_text(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_text(
        Point::from_xy(0.0, 50.0),
        Fill::default(),
        Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, true).unwrap(),
        16.0,
        &[],
        "यह कुछ जटिल पाठ है.",
        false,
        TextDirection::Auto,
    );
}

#[snapshot2(single_page)]
fn complex_text_2(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_text(
        Point::from_xy(0.0, 50.0),
        Fill::default(),
        Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, true).unwrap(),
        16.0,
        &[],
        "यु॒धा नर॑ ऋ॒ष्वा",
        false,
        TextDirection::Auto,
    );
}

#[snapshot2(single_page)]
fn complex_text_3(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_text(
        Point::from_xy(0.0, 50.0),
        Fill::default(),
        Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, true).unwrap(),
        12.0,
        &[],
        "आ रु॒क्मैरा यु॒धा नर॑ ऋ॒ष्वा ऋ॒ष्टीर॑सृक्षत ।",
        false,
        TextDirection::Auto,
    );
}

#[snapshot2(single_page)]
fn complex_text_4(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_text(
        Point::from_xy(0.0, 50.0),
        Fill::default(),
        Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, true).unwrap(),
        10.0,
        &[],
        "अन्वे॑नाँ॒ अह॑ वि॒द्युतो॑ म॒रुतो॒ जज्झ॑तीरव भनर॑र्त॒ त्मना॑ दि॒वः ॥",
        false,
        TextDirection::Auto,
    );
}

pub(crate) fn sample_svg() -> usvg::Tree {
    let data = std::fs::read(SVGS_PATH.join("resvg_masking_mask_with_opacity_1.svg")).unwrap();
    usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap()
}

#[visreg2]
fn svg_simple(surface: &mut Surface) {
    let tree = sample_svg();
    surface.draw_svg(&tree, tree.size(), SvgSettings::default());
}

#[visreg2]
fn svg_outlined_text(surface: &mut Surface) {
    let data = std::fs::read(SVGS_PATH.join("resvg_text_text_simple_case.svg")).unwrap();
    let tree = usvg::Tree::from_data(
        &data,
        &usvg::Options {
            fontdb: FONTDB.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let settings = SvgSettings {
        embed_text: false,
        ..Default::default()
    };
    surface.draw_svg(&tree, tree.size(), settings);
}

#[visreg2]
fn svg_resized(surface: &mut Surface) {
    surface.draw_svg(
        &sample_svg(),
        Size::from_wh(120.0, 80.0).unwrap(),
        SvgSettings::default(),
    );
}

#[visreg2]
fn svg_should_be_clipped(surface: &mut Surface) {
    let data =
        std::fs::read(SVGS_PATH.join("custom_paint_servers_pattern_patterns_2.svg")).unwrap();
    let tree = usvg::Tree::from_data(&data, &usvg::Options::default()).unwrap();

    surface.push_transform(&Transform::from_translate(100.0, 0.0));
    surface.draw_svg(&tree, tree.size(), SvgSettings::default());
    surface.pop();
}

#[visreg2]
fn svg_with_filter(surface: &mut Surface) {
    let data = std::fs::read(SVGS_PATH.join("small_text_with_filter.svg")).unwrap();
    let tree = usvg::Tree::from_data(
        &data,
        &usvg::Options {
            fontdb: FONTDB.clone(),
            ..usvg::Options::default()
        },
    )
    .unwrap();

    surface.draw_svg(&tree, tree.size(), SvgSettings::default());
}

fn text_gradient(spread_method: SpreadMethod) -> LinearGradient {
    LinearGradient {
        x1: 50.0,
        y1: 0.0,
        x2: 150.0,
        y2: 0.0,
        transform: Default::default(),
        spread_method,
        stops: stops_with_3_solid_1(),
        anti_alias: false,
    }
}

fn text_with_fill_impl(surface: &mut Surface, outlined: bool) {
    let font = Font::new(NOTO_SANS.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(0.0, 80.0),
        red_fill(0.5),
        font.clone(),
        20.0,
        &[],
        "red outlined text",
        outlined,
        TextDirection::Auto,
    );

    surface.fill_text(
        Point::from_xy(0.0, 100.0),
        blue_fill(0.8),
        font.clone(),
        20.0,
        &[],
        "blue outlined text",
        outlined,
        TextDirection::Auto,
    );

    let grad_fill = Fill {
        paint: Paint::from(text_gradient(SpreadMethod::Pad)),
        ..Default::default()
    };

    surface.fill_text(
        Point::from_xy(0.0, 120.0),
        grad_fill,
        font.clone(),
        20.0,
        &[],
        "gradient text",
        outlined,
        TextDirection::Auto,
    );

    let noto_font = Font::new(NOTO_COLOR_EMOJI_COLR.clone(), 0, true).unwrap();

    surface.fill_text(
        Point::from_xy(0.0, 140.0),
        blue_fill(0.8),
        noto_font.clone(),
        20.0,
        &[],
        "😄😁😆",
        outlined,
        TextDirection::Auto,
    );

    let grad_fill = Fill {
        paint: Paint::from(text_gradient(SpreadMethod::Reflect)),
        ..Default::default()
    };

    surface.fill_text(
        Point::from_xy(0.0, 160.0),
        grad_fill,
        font,
        20.0,
        &[],
        "longer gradient text with repeat",
        outlined,
        TextDirection::Auto,
    );
}

#[visreg2]
fn text_outlined_with_fill(surface: &mut Surface) {
    text_with_fill_impl(surface, true)
}

fn text_with_stroke_impl(surface: &mut Surface, outlined: bool) {
    let font = Font::new(NOTO_SANS.clone(), 0, true).unwrap();
    surface.stroke_text(
        Point::from_xy(0.0, 80.0),
        red_stroke(0.5, 1.0),
        font.clone(),
        20.0,
        &[],
        "red outlined text",
        outlined,
        TextDirection::Auto,
    );

    surface.stroke_text(
        Point::from_xy(0.0, 100.0),
        blue_stroke(0.8),
        font.clone(),
        20.0,
        &[],
        "blue outlined text",
        outlined,
        TextDirection::Auto,
    );

    let grad_stroke = Stroke {
        paint: Paint::from(text_gradient(SpreadMethod::Pad)),
        ..Default::default()
    };

    surface.stroke_text(
        Point::from_xy(0.0, 120.0),
        grad_stroke,
        font,
        20.0,
        &[],
        "gradient text",
        outlined,
        TextDirection::Auto,
    );

    let font = Font::new(NOTO_COLOR_EMOJI_COLR.clone(), 0, true).unwrap();

    surface.stroke_text(
        Point::from_xy(0.0, 140.0),
        blue_stroke(0.8),
        font,
        20.0,
        &[],
        "😄😁😆",
        outlined,
        TextDirection::Auto,
    );
}

#[visreg2]
fn text_outlined_with_stroke(surface: &mut Surface) {
    text_with_stroke_impl(surface, true);
}

#[visreg2]
fn text_zalgo(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(0.0, 100.0),
        Fill::default(),
        font,
        32.0,
        &[],
        "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔̦͆̏̓͢",
        false,
        TextDirection::Auto,
    );
}

#[visreg2]
fn text_zalgo_outlined(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(0.0, 100.0),
        Fill::default(),
        font,
        32.0,
        &[],
        "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔̦͆̏̓͢",
        true,
        TextDirection::Auto,
    );
}
