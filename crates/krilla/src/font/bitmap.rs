//! Drawing bitmap-based glyphs to a surface.

use crate::font::bitmap::utils::{BitmapData, BitmapFormat, BitmapStrikes, Origin};
use crate::font::Font;
use crate::object::image::Image;
use crate::surface::Surface;
use skrifa::{GlyphId, MetadataProvider};
use std::sync::Arc;
use tiny_skia_path::{Size, Transform};

/// Draw a bitmap-based glyph on a surface.
pub fn draw_glyph(font: Font, glyph: GlyphId, surface: &mut Surface) -> Option<()> {
    let metrics = font
        .font_ref()
        .metrics(skrifa::instance::Size::unscaled(), font.location_ref());

    let bitmap_strikes = BitmapStrikes::new(font.font_ref());

    let bitmap_glyph = bitmap_strikes.iter().filter_map(|s| s.get(glyph)).last()?;
    let upem = metrics.units_per_em as f32;

    match bitmap_glyph.data {
        BitmapData::Png(data) => {
            let image = Image::from_png(Arc::new(data.to_vec()))?;
            let size = Size::from_wh(image.size().0 as f32, image.size().1 as f32).unwrap();

            // Adapted from vello.
            let scale_factor = upem / (bitmap_glyph.ppem_y);
            let mut transform =
                Transform::from_translate(-bitmap_glyph.bearing_x, bitmap_glyph.bearing_y)
                    .pre_scale(scale_factor, scale_factor)
                    .pre_translate(-bitmap_glyph.inner_bearing_x, -bitmap_glyph.inner_bearing_y);

            transform = match bitmap_glyph.placement_origin {
                Origin::TopLeft => transform,
                Origin::BottomLeft => transform.pre_translate(0.0, -(image.size().1 as f32)),
            };

            transform = if let Some(format) = bitmap_strikes.format() {
                if format == BitmapFormat::Sbix {
                    // For unknown reasons, using Apple Color Emoji will lead to a vertical shift on MacOS, but this shift
                    // doesn't seem to be coming from the font and most likely is somehow hardcoded. On Windows,
                    // this shift will not be applied. However, if this shift is not applied the emojis are a bit
                    // too high up when being together with other text, so we try to imitate this.
                    // See also https://github.com/harfbuzz/harfbuzz/issues/2679#issuecomment-1345595425
                    // We approximate this vertical shift that seems to be produced by it.
                    // This value seems to be pretty close to what is happening on MacOS.
                    transform
                        .pre_concat(Transform::from_translate(0.0, 0.128 * upem / scale_factor))
                } else {
                    transform
                }
            } else {
                transform
            };

            surface.push_transform(&transform);
            surface.draw_image(image, size);
            surface.pop();

            Some(())
        }
        BitmapData::Bgra(_) => None,
        BitmapData::Mask(_) => None,
    }
}

mod utils {
    // Copyright 2024 the Vello Authors
    // SPDX-License-Identifier: Apache-2.0 OR MIT

    // Based on https://github.com/googlefonts/fontations/blob/cbdf8b485e955e3acee40df1344e33908805ed31/skrifa/src/bitmap.rs
    #![allow(warnings)]

    //! Bitmap strikes and glyphs.
    use skrifa::{
        instance::{LocationRef, Size},
        metrics::GlyphMetrics,
        raw::{
            tables::{bitmap, cbdt, cblc, ebdt, eblc, sbix},
            types::{GlyphId, Tag},
            FontData, TableProvider,
        },
        MetadataProvider,
    };

    /// Set of strikes, each containing embedded bitmaps of a single size.
    #[derive(Clone)]
    pub struct BitmapStrikes<'a>(StrikesKind<'a>);

    impl<'a> BitmapStrikes<'a> {
        /// Creates a new `BitmapStrikes` for the given font.
        ///
        /// This will prefer `sbix`, `CBDT`, and `CBLC` formats in that order.
        ///
        /// To select a specific format, use [`with_format`](Self::with_format).
        pub fn new(font: &(impl TableProvider<'a> + MetadataProvider<'a>)) -> Self {
            for format in [BitmapFormat::Sbix, BitmapFormat::Cbdt, BitmapFormat::Ebdt] {
                if let Some(strikes) = Self::with_format(font, format) {
                    return strikes;
                }
            }
            Self(StrikesKind::None)
        }

        /// Creates a new `BitmapStrikes` for the given font and format.
        ///
        /// Returns `None` if the requested format is not available.
        pub fn with_format(
            font: &(impl TableProvider<'a> + MetadataProvider<'a>),
            format: BitmapFormat,
        ) -> Option<Self> {
            let kind = match format {
                BitmapFormat::Sbix => StrikesKind::Sbix(
                    font.sbix().ok()?,
                    font.glyph_metrics(Size::unscaled(), LocationRef::default()),
                ),
                BitmapFormat::Cbdt => {
                    StrikesKind::Cbdt(CbdtTables::new(font.cblc().ok()?, font.cbdt().ok()?))
                }
                BitmapFormat::Ebdt => {
                    StrikesKind::Ebdt(EbdtTables::new(font.eblc().ok()?, font.ebdt().ok()?))
                }
            };
            Some(Self(kind))
        }

        /// Returns the format representing the underlying table for this set of
        /// strikes.
        pub fn format(&self) -> Option<BitmapFormat> {
            match &self.0 {
                StrikesKind::None => None,
                StrikesKind::Sbix(..) => Some(BitmapFormat::Sbix),
                StrikesKind::Cbdt(..) => Some(BitmapFormat::Cbdt),
                StrikesKind::Ebdt(..) => Some(BitmapFormat::Ebdt),
            }
        }

        /// Returns the number of available strikes.
        pub fn len(&self) -> usize {
            match &self.0 {
                StrikesKind::None => 0,
                StrikesKind::Sbix(sbix, _) => sbix.strikes().len(),
                StrikesKind::Cbdt(cbdt) => cbdt.location.bitmap_sizes().len(),
                StrikesKind::Ebdt(ebdt) => ebdt.location.bitmap_sizes().len(),
            }
        }

        /// Returns true if there are no available strikes.
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        /// Returns the strike at the given index.
        pub fn get(&self, index: usize) -> Option<BitmapStrike<'a>> {
            let kind = match &self.0 {
                StrikesKind::None => return None,
                StrikesKind::Sbix(sbix, metrics) => {
                    StrikeKind::Sbix(sbix.strikes().get(index).ok()?, metrics.clone())
                }
                StrikesKind::Cbdt(tables) => StrikeKind::Cbdt(
                    tables.location.bitmap_sizes().get(index).copied()?,
                    tables.clone(),
                ),
                StrikesKind::Ebdt(tables) => StrikeKind::Ebdt(
                    tables.location.bitmap_sizes().get(index).copied()?,
                    tables.clone(),
                ),
            };
            Some(BitmapStrike(kind))
        }

        /// Returns the best matching glyph for the given size and glyph
        /// identifier.
        ///
        /// In this case, "best" means a glyph of the exact size, nearest larger
        /// size, or nearest smaller size, in that order.
        pub fn glyph_for_size(&self, size: Size, glyph_id: GlyphId) -> Option<BitmapGlyph<'a>> {
            // Return the largest size for an unscaled request
            let size = size.ppem().unwrap_or(f32::MAX);
            self.iter()
                .fold(None, |best: Option<BitmapGlyph<'a>>, entry| {
                    let entry_size = entry.ppem();
                    if let Some(best) = best {
                        let best_size = best.ppem_y;
                        if (entry_size >= size && entry_size < best_size)
                            || (best_size < size && entry_size > best_size)
                        {
                            entry.get(glyph_id).or(Some(best))
                        } else {
                            Some(best)
                        }
                    } else {
                        entry.get(glyph_id)
                    }
                })
        }

        /// Returns an iterator over all available strikes.
        pub fn iter(&self) -> impl Iterator<Item = BitmapStrike<'a>> + 'a + Clone {
            let this = self.clone();
            (0..this.len()).filter_map(move |ix| this.get(ix))
        }
    }

    #[derive(Clone)]
    enum StrikesKind<'a> {
        None,
        Sbix(sbix::Sbix<'a>, GlyphMetrics<'a>),
        Cbdt(CbdtTables<'a>),
        Ebdt(EbdtTables<'a>),
    }

    /// Set of embedded bitmap glyphs of a specific size.
    #[derive(Clone)]
    pub struct BitmapStrike<'a>(StrikeKind<'a>);

    impl<'a> BitmapStrike<'a> {
        /// Returns the pixels-per-em (size) of this strike.
        pub fn ppem(&self) -> f32 {
            match &self.0 {
                StrikeKind::Sbix(sbix, _) => sbix.ppem() as f32,
                StrikeKind::Cbdt(size, _) => size.ppem_y() as f32,
                StrikeKind::Ebdt(size, _) => size.ppem_y() as f32,
            }
        }

        /// Returns a bitmap glyph for the given identifier, if available.
        pub fn get(&self, glyph_id: GlyphId) -> Option<BitmapGlyph<'a>> {
            match &self.0 {
                StrikeKind::Sbix(sbix, metrics) => {
                    let glyph = sbix.glyph_data(glyph_id).ok()??;
                    if glyph.graphic_type() != Tag::new(b"png ") {
                        return None;
                    }
                    let glyf_bb = metrics.bounds(glyph_id).unwrap_or_default();
                    let lsb = metrics.left_side_bearing(glyph_id).unwrap_or_default();
                    let ppem = sbix.ppem() as f32;
                    let png_data = glyph.data();
                    // PNG format:
                    // 8 byte header, IHDR chunk (4 byte length, 4 byte chunk type), width, height
                    let reader = FontData::new(png_data);
                    let width = reader.read_at::<u32>(16).ok()?;
                    let height = reader.read_at::<u32>(20).ok()?;
                    Some(BitmapGlyph {
                        data: BitmapData::Png(glyph.data()),
                        bearing_x: lsb,
                        bearing_y: glyf_bb.y_min as f32,
                        inner_bearing_x: glyph.origin_offset_x() as f32,
                        inner_bearing_y: glyph.origin_offset_y() as f32,
                        ppem_x: ppem,
                        ppem_y: ppem,
                        width,
                        height,
                        advance: metrics.advance_width(glyph_id).unwrap_or_default(),
                        placement_origin: Origin::BottomLeft,
                    })
                }
                StrikeKind::Cbdt(size, tables) => {
                    let location = size
                        .location(tables.location.offset_data(), glyph_id)
                        .ok()?;
                    let data = tables.data.data(&location).ok()?;
                    BitmapGlyph::from_bdt(&size, &data)
                }
                StrikeKind::Ebdt(size, tables) => {
                    let location = size
                        .location(tables.location.offset_data(), glyph_id)
                        .ok()?;
                    let data = tables.data.data(&location).ok()?;
                    BitmapGlyph::from_bdt(&size, &data)
                }
            }
        }
    }

    #[derive(Clone)]
    enum StrikeKind<'a> {
        Sbix(sbix::Strike<'a>, GlyphMetrics<'a>),
        Cbdt(bitmap::BitmapSize, CbdtTables<'a>),
        Ebdt(bitmap::BitmapSize, EbdtTables<'a>),
    }

    #[derive(Clone)]
    struct BdtTables<L, D> {
        location: L,
        data: D,
    }

    impl<L, D> BdtTables<L, D> {
        fn new(location: L, data: D) -> Self {
            Self { location, data }
        }
    }

    type CbdtTables<'a> = BdtTables<cblc::Cblc<'a>, cbdt::Cbdt<'a>>;
    type EbdtTables<'a> = BdtTables<eblc::Eblc<'a>, ebdt::Ebdt<'a>>;

    /// An embedded bitmap glyph.
    #[derive(Clone)]
    pub struct BitmapGlyph<'a> {
        pub data: BitmapData<'a>,
        pub bearing_x: f32,
        pub bearing_y: f32,
        pub inner_bearing_x: f32,
        pub inner_bearing_y: f32,
        pub ppem_x: f32,
        pub ppem_y: f32,
        pub advance: f32,
        pub width: u32,
        pub height: u32,
        pub placement_origin: Origin,
    }

    impl<'a> BitmapGlyph<'a> {
        fn from_bdt(
            bitmap_size: &bitmap::BitmapSize,
            bitmap_data: &bitmap::BitmapData<'a>,
        ) -> Option<Self> {
            let metrics = BdtMetrics::new(&bitmap_data);
            let (ppem_x, ppem_y) = (bitmap_size.ppem_x() as f32, bitmap_size.ppem_y() as f32);
            let bpp = bitmap_size.bit_depth();
            let data = match bpp {
                32 => {
                    match &bitmap_data.content {
                        bitmap::BitmapContent::Data(bitmap::BitmapDataFormat::Png, bytes) => {
                            BitmapData::Png(bytes)
                        }
                        // 32-bit formats are always byte aligned
                        bitmap::BitmapContent::Data(
                            bitmap::BitmapDataFormat::ByteAligned,
                            bytes,
                        ) => BitmapData::Bgra(bytes),
                        _ => return None,
                    }
                }
                1 | 2 | 4 | 8 => {
                    let (data, is_packed) = match &bitmap_data.content {
                        bitmap::BitmapContent::Data(
                            bitmap::BitmapDataFormat::ByteAligned,
                            bytes,
                        ) => (bytes, false),
                        bitmap::BitmapContent::Data(
                            bitmap::BitmapDataFormat::BitAligned,
                            bytes,
                        ) => (bytes, true),
                        _ => return None,
                    };
                    BitmapData::Mask(MaskData {
                        bpp,
                        is_packed,
                        data,
                    })
                }
                // All other bit depth values are invalid
                _ => return None,
            };
            Some(Self {
                data,
                bearing_x: 0.0,
                bearing_y: 0.0,
                inner_bearing_x: metrics.inner_bearing_x,
                inner_bearing_y: metrics.inner_bearing_y,
                ppem_x,
                ppem_y,
                width: metrics.width,
                height: metrics.height,
                advance: metrics.advance,
                placement_origin: Origin::TopLeft,
            })
        }
    }

    struct BdtMetrics {
        inner_bearing_x: f32,
        inner_bearing_y: f32,
        advance: f32,
        width: u32,
        height: u32,
    }

    impl BdtMetrics {
        fn new(data: &bitmap::BitmapData) -> Self {
            match data.metrics {
                bitmap::BitmapMetrics::Small(metrics) => Self {
                    inner_bearing_x: metrics.bearing_x() as f32,
                    inner_bearing_y: metrics.bearing_y() as f32,
                    advance: metrics.advance() as f32,
                    width: metrics.width() as u32,
                    height: metrics.height() as u32,
                },
                bitmap::BitmapMetrics::Big(metrics) => Self {
                    inner_bearing_x: metrics.hori_bearing_x() as f32,
                    inner_bearing_y: metrics.hori_bearing_y() as f32,
                    advance: metrics.hori_advance() as f32,
                    width: metrics.width() as u32,
                    height: metrics.height() as u32,
                },
            }
        }
    }

    /// Determines the origin point for drawing a bitmap glyph.
    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum Origin {
        TopLeft,
        BottomLeft,
    }

    /// Data content of a bitmap.
    #[derive(Clone)]
    pub enum BitmapData<'a> {
        /// Uncompressed 32-bit color bitmap data, pre-multiplied in BGRA order
        /// and encoded in the sRGB color space.
        Bgra(&'a [u8]),
        /// Compressed PNG bitmap data.
        Png(&'a [u8]),
        /// Data representing a single channel alpha mask.
        Mask(MaskData<'a>),
    }

    /// A single channel alpha mask.
    #[derive(Clone)]
    pub struct MaskData<'a> {
        /// Number of bits-per-pixel. Always 1, 2, 4 or 8.
        pub bpp: u8,
        /// True if each row of the data is bit-aligned. Otherwise, each row
        /// is padded to the next byte.
        pub is_packed: bool,
        /// Raw bitmap data.
        pub data: &'a [u8],
    }

    /// The format (or table) containing the data backing a set of bitmap strikes.
    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub enum BitmapFormat {
        Sbix,
        Cbdt,
        Ebdt,
    }
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::tests::{all_glyphs_to_pdf, NOTO_COLOR_EMOJI_CBDT};
    use krilla_macros::visreg;

    // We don't run on pdf.js because it leads to a high pixel difference in CI
    // for some reason.
    #[visreg(document, pdfium, mupdf, pdfbox, ghostscript, poppler, quartz)]
    fn noto_color_emoji_cbdt(document: &mut Document) {
        let font_data = NOTO_COLOR_EMOJI_CBDT.clone();
        all_glyphs_to_pdf(font_data, None, false, true, document);
    }

    #[cfg(target_os = "macos")]
    #[visreg(document, all)]
    fn apple_color_emoji(document: &mut Document) {
        use std::sync::Arc;

        let font_data =
            Arc::new(std::fs::read("/System/Library/Fonts/Apple Color Emoji.ttc").unwrap());
        all_glyphs_to_pdf(font_data, None, false, true, document);
    }
}
