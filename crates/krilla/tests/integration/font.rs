use crate::{all_glyphs_to_pdf, NOTO_COLOR_EMOJI_CBDT};
use krilla::Document;
use krilla_macros::{visreg, visreg2};

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
