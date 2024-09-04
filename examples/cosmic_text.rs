//! This example shows how to use cosmic-text to create advanced layouted text.
//!
//! It is unfortunately still somewhat hard if you are not familiar with text
//! shaping/layouting, but using this code as a template should hopefully help
//! you get started.
//!
//! Another important point to mention is that you need to ensure that the
//! version of `cosmic-text` you use uses the same `fontdb` version as krilla.
//! For this example, we are using [my fork](https://github.com/LaurenzV/cosmic-text)
//! that I try to keep in sync.

use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};
use krilla::color::rgb::Rgb;
use krilla::font::{GlyphUnits, KrillaGlyph};
use krilla::path::Fill;
use krilla::{Document, PageSettings};
use skrifa::GlyphId;
use tiny_skia_path::Point;

fn main() {
    // Set up `cosmic-text`. See their documentation for more information
    // on how you can configure it further.
    let metrics = Metrics::new(20.0, 22.0);
    let mut font_system = FontSystem::new();
    let mut buffer = Buffer::new(&mut font_system, metrics);
    buffer.set_size(&mut font_system, Some(200.0), None);
    let attrs = Attrs::new();
    let text = "This is a long text. We want it to not be wider than 200pt, \
    so that it fits on the page. Let's intersperse some emojis 💩👻💀emojis🦩🌚😁😆\
    as well as complex scripts: हैलो वर्ल्ड and مرحبا بالعالم";
    buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);
    buffer.shape_until_scroll(&mut font_system, false);

    // The usual page setup.
    let mut document = Document::new();
    let mut page = document.start_page_with(PageSettings::new(200.0, 200.0));
    let mut surface = page.surface();

    // Use the `convert_fontdb` method to get a hashmap that maps cosmic-text font IDs to
    // krilla fonts.
    let font_map = surface.convert_fontdb(font_system.db_mut(), None).unwrap();

    for run in buffer.layout_runs() {
        let y_offset = run.line_y;

        // A layout run in cosmic text can consist of glyphs from different fonts, but
        // in krilla, a glyph run must belong to the same font. Because of this, we need to group
        // the glyphs by font. In this example, we use a slice extension trait to achieve that
        // effect, but you are free to implement your own logic.
        let segmented = run
            .glyphs
            .group_by_key(|g| (font_map.get(&g.font_id).unwrap().clone(), g.font_size));

        let mut x = 0.0;
        // Go over the segmented glyph runs, and convert them into krilla glyphs.
        for ((font, size), glyphs) in segmented {
            let start_x = x;
            let glyphs = glyphs
                .iter()
                .map(|glyph| {
                    x += glyph.w;
                    KrillaGlyph::new(
                        GlyphId::new(glyph.glyph_id as u32),
                        glyph.w,
                        glyph.x_offset,
                        glyph.y_offset,
                        glyph.start..glyph.end,
                    )
                })
                .collect::<Vec<_>>();

            // Draw the glyphs using the `fill_glyphs` method!
            surface.fill_glyphs(
                Point::from_xy(start_x, y_offset),
                Fill::<Rgb>::default(),
                &glyphs,
                font,
                run.text,
                size,
                GlyphUnits::UserSpace
            );
        }
    }

    surface.finish();
    page.finish();
    let pdf = document.finish().unwrap();

    std::fs::write("target/cosmic_text.pdf", &pdf).unwrap();
}

pub trait SliceExt<T> {
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F>
    where
        F: FnMut(&T) -> K,
        K: PartialEq;
}

impl<T> SliceExt<T> for [T] {
    fn group_by_key<K, F>(&self, f: F) -> GroupByKey<'_, T, F> {
        GroupByKey { slice: self, f }
    }
}

pub struct GroupByKey<'a, T, F> {
    slice: &'a [T],
    f: F,
}

impl<'a, T, K, F> Iterator for GroupByKey<'a, T, F>
where
    F: FnMut(&T) -> K,
    K: PartialEq,
{
    type Item = (K, &'a [T]);

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.slice.iter();
        let key = (self.f)(iter.next()?);
        let count = 1 + iter.take_while(|t| (self.f)(t) == key).count();
        let (head, tail) = self.slice.split_at(count);
        self.slice = tail;
        Some((key, head))
    }
}
