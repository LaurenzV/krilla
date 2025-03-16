use krilla::graphics::paint::{Fill, LinearGradient, Paint, SpreadMethod, Stroke};
use krilla::page::Page;
use krilla::surface::Surface;
use krilla::text::TextDirection;
use krilla::{Data, Font, Point};
use krilla_macros::{snapshot, visreg};

use crate::{
    blue_fill, blue_stroke, red_fill, red_stroke, stops_with_3_solid_1, LATIN_MODERN_ROMAN,
    NOTO_COLOR_EMOJI_COLR, NOTO_SANS, NOTO_SANS_CJK, NOTO_SANS_DEVANAGARI,
};

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
    surface.set_fill(red_fill(0.5));
    surface.fill_text(
        Point::from_xy(0.0, 80.0),
        font.clone(),
        20.0,
        "red outlined text",
        outlined,
        TextDirection::Auto,
    );

    surface.set_fill(blue_fill(0.8));
    surface.fill_text(
        Point::from_xy(0.0, 100.0),
        font.clone(),
        20.0,
        "blue outlined text",
        outlined,
        TextDirection::Auto,
    );

    let grad_fill = Fill {
        paint: Paint::from(text_gradient(SpreadMethod::Pad)),
        ..Default::default()
    };

    surface.set_fill(grad_fill);
    surface.fill_text(
        Point::from_xy(0.0, 120.0),
        font.clone(),
        20.0,
        "gradient text",
        outlined,
        TextDirection::Auto,
    );

    let noto_font = Font::new(NOTO_COLOR_EMOJI_COLR.clone(), 0, true).unwrap();

    surface.set_fill(blue_fill(0.8));
    surface.fill_text(
        Point::from_xy(0.0, 140.0),
        noto_font.clone(),
        20.0,
        "😄😁😆",
        outlined,
        TextDirection::Auto,
    );

    let grad_fill = Fill {
        paint: Paint::from(text_gradient(SpreadMethod::Reflect)),
        ..Default::default()
    };

    surface.set_fill(grad_fill);
    surface.fill_text(
        Point::from_xy(0.0, 160.0),
        font,
        20.0,
        "longer gradient text with repeat",
        outlined,
        TextDirection::Auto,
    );
}

#[visreg]
fn text_outlined_with_fill(surface: &mut Surface) {
    text_with_fill_impl(surface, true)
}

fn text_with_stroke_impl(surface: &mut Surface, outlined: bool) {
    let font = Font::new(NOTO_SANS.clone(), 0, true).unwrap();
    surface.set_stroke(red_stroke(0.5, 1.0));
    surface.stroke_text(
        Point::from_xy(0.0, 80.0),
        font.clone(),
        20.0,
        "red outlined text",
        outlined,
        TextDirection::Auto,
    );

    surface.set_stroke(blue_stroke(0.8));
    surface.stroke_text(
        Point::from_xy(0.0, 100.0),
        font.clone(),
        20.0,
        "blue outlined text",
        outlined,
        TextDirection::Auto,
    );

    let grad_stroke = Stroke {
        paint: Paint::from(text_gradient(SpreadMethod::Pad)),
        ..Default::default()
    };

    surface.set_stroke(grad_stroke);
    surface.stroke_text(
        Point::from_xy(0.0, 120.0),
        font,
        20.0,
        "gradient text",
        outlined,
        TextDirection::Auto,
    );

    let font = Font::new(NOTO_COLOR_EMOJI_COLR.clone(), 0, true).unwrap();

    surface.set_stroke(blue_stroke(0.8));
    surface.stroke_text(
        Point::from_xy(0.0, 140.0),
        font,
        20.0,
        "😄😁😆",
        outlined,
        TextDirection::Auto,
    );
}

#[visreg]
fn text_outlined_with_stroke(surface: &mut Surface) {
    text_with_stroke_impl(surface, true);
}

#[visreg]
fn text_zalgo(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(0.0, 100.0),
        font,
        32.0,
        "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔̦͆̏̓͢",
        false,
        TextDirection::Auto,
    );
}

#[visreg]
fn text_direction_ltr(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS_CJK.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "你好这是一段则是文字",
        false,
        TextDirection::LeftToRight,
    );
}

#[visreg]
fn text_direction_rtl(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS_CJK.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(0.0, 100.0),
        font,
        20.0,
        "你好这是一段则是文字",
        false,
        TextDirection::RightToLeft,
    );
}

#[visreg]
fn text_direction_ttb(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS_CJK.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(100.0, 0.0),
        font,
        20.0,
        "你好这是一段则是文字",
        false,
        TextDirection::TopToBottom,
    );
}

#[visreg]
fn text_direction_btt(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS_CJK.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(100.0, 0.0),
        font,
        20.0,
        "你好这是一段则是文字",
        false,
        TextDirection::BottomToTop,
    );
}

pub(crate) fn simple_text_impl(page: &mut Page, font_data: Data) {
    let mut surface = page.surface();
    surface.fill_text(
        Point::from_xy(0.0, 50.0),
        Font::new(font_data, 0, true).unwrap(),
        16.0,
        "A line of text.",
        false,
        TextDirection::Auto,
    );
}

#[snapshot]
fn text_simple_cff(page: &mut Page) {
    simple_text_impl(page, LATIN_MODERN_ROMAN.clone());
}

#[snapshot]
fn text_simple_ttf(page: &mut Page) {
    simple_text_impl(page, NOTO_SANS.clone());
}

#[snapshot]
fn text_complex(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_text(
        Point::from_xy(0.0, 50.0),
        Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, true).unwrap(),
        16.0,
        "यह कुछ जटिल पाठ है.",
        false,
        TextDirection::Auto,
    );
}

#[snapshot]
fn text_complex_2(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_text(
        Point::from_xy(0.0, 50.0),
        Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, true).unwrap(),
        16.0,
        "यु॒धा नर॑ ऋ॒ष्वा",
        false,
        TextDirection::Auto,
    );
}

#[snapshot]
fn text_complex_3(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_text(
        Point::from_xy(0.0, 50.0),
        Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, true).unwrap(),
        12.0,
        "आ रु॒क्मैरा यु॒धा नर॑ ऋ॒ष्वा ऋ॒ष्टीर॑सृक्षत ।",
        false,
        TextDirection::Auto,
    );
}

#[snapshot]
fn text_complex_4(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_text(
        Point::from_xy(0.0, 50.0),
        Font::new(NOTO_SANS_DEVANAGARI.clone(), 0, true).unwrap(),
        10.0,
        "अन्वे॑नाँ॒ अह॑ वि॒द्युतो॑ म॒रुतो॒ जज्झ॑तीरव भनर॑र्त॒ त्मना॑ दि॒वः ॥",
        false,
        TextDirection::Auto,
    );
}

#[visreg]
fn text_zalgo_outlined(surface: &mut Surface) {
    let font = Font::new(NOTO_SANS.clone(), 0, true).unwrap();
    surface.fill_text(
        Point::from_xy(0.0, 100.0),
        font,
        32.0,
        "z͈̤̭͖̉͑́a̳ͫ́̇͑̽͒ͯlͨ͗̍̀̍̔̀ģ͔̫̫̄o̗̠͔̦͆̏̓͢",
        true,
        TextDirection::Auto,
    );
}

#[snapshot]
fn text_fill(page: &mut Page) {
    let mut surface = page.surface();
    surface.fill_text(
        Point::from_xy(0.0, 50.0),
        Font::new(NOTO_SANS.clone(), 0, true).unwrap(),
        16.0,
        "hi there",
        false,
        TextDirection::Auto,
    );
}

#[snapshot]
fn text_stroke(page: &mut Page) {
    let mut surface = page.surface();
    surface.stroke_text(
        Point::from_xy(0.0, 50.0),
        Font::new(NOTO_SANS.clone(), 0, true).unwrap(),
        16.0,
        "hi there",
        false,
        TextDirection::Auto,
    );
}
