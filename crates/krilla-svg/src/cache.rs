//! Export-local reuse of immutable filtered rasters.

use std::sync::Arc;

use krilla::image::Image;
use usvg::{tiny_skia_path::Transform, Tree};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct RasterKey {
    tree: usize,
    group: usize,
    transform: [u32; 6],
    width: u32,
    height: u32,
    filter_scale: u32,
}

impl RasterKey {
    pub(crate) fn new(
        tree: &Arc<Tree>,
        group: &usvg::Group,
        transform: Transform,
        width: u32,
        height: u32,
        filter_scale: f32,
    ) -> Self {
        Self {
            tree: Arc::as_ptr(tree) as usize,
            group: group as *const usvg::Group as usize,
            transform: [
                transform.sx,
                transform.kx,
                transform.ky,
                transform.sy,
                transform.tx,
                transform.ty,
            ]
            .map(f32::to_bits),
            width,
            height,
            filter_scale: filter_scale.to_bits(),
        }
    }
}

struct Entry {
    key: RasterKey,
    // Prevent both tree and group allocation identities from being recycled.
    _tree: Arc<Tree>,
    image: Image,
    charge: usize,
}

/// A bounded LRU of filtered raster images for a single export.
///
/// Use with [`crate::SurfaceExt::draw_svg_cached`]. Resolved trees are retained
/// by Arc identity, and exact raster inputs are compared. The cache contains
/// no PDF resource handles. It requires exclusive access while drawing.
///
/// Limits cover charged raster storage, not process memory, retained trees,
/// transient allocations, or images retained by the PDF document.
pub struct FilterCache {
    entries: Vec<Entry>,
    max_bytes: usize,
    max_entries: usize,
    retained_bytes: usize,
}

impl Default for FilterCache {
    fn default() -> Self {
        Self::new(64 * 1024 * 1024, 64)
    }
}

impl FilterCache {
    /// Create a cache with a raster-charge budget and entry limit.
    /// Zero for either limit disables retention, preserving normal rendering.
    pub fn new(max_bytes: usize, max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_bytes,
            max_entries,
            retained_bytes: 0,
        }
    }

    pub(crate) fn get(&mut self, key: RasterKey) -> Option<Image> {
        if let Some(index) = self.entries.iter().position(|e| e.key == key) {
            let entry = self.entries.remove(index);
            let image = entry.image.clone();
            self.entries.push(entry);
            Some(image)
        } else {
            None
        }
    }

    pub(crate) fn insert(&mut self, key: RasterKey, tree: &Arc<Tree>, image: Image) {
        let charge = (key.width as usize)
            .checked_mul(key.height as usize)
            .and_then(|n| n.checked_mul(8))
            .and_then(|n| n.checked_add(1024));
        let Some(charge) = charge.filter(|&n| n <= self.max_bytes && self.max_entries > 0) else {
            return;
        };
        while self.entries.len() >= self.max_entries
            || self.retained_bytes > self.max_bytes - charge
        {
            let entry = self.entries.remove(0);
            self.retained_bytes -= entry.charge;
        }
        if self.entries.try_reserve(1).is_err() {
            return;
        }
        self.entries.push(Entry {
            key,
            _tree: tree.clone(),
            image,
            charge,
        });
        self.retained_bytes += charge;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SurfaceExt, SvgSettings};
    use krilla::{geom::Size, Document};

    fn tree(color: &str) -> Arc<Tree> {
        Arc::new(Tree::from_str(&format!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="30"><defs><filter id="same"><feGaussianBlur stdDeviation="1"/></filter></defs><g filter="url(#same)"><rect x="5" y="5" width="30" height="20" fill="{color}"/></g></svg>"##), &Default::default()).unwrap())
    }

    fn key(tree: &Arc<Tree>, scale: f32) -> RasterKey {
        RasterKey::new(
            tree,
            tree.root(),
            Transform::from_scale(scale, scale),
            40,
            30,
            4.0,
        )
    }

    fn image() -> Image {
        Image::from_rgba8(vec![0; 40 * 30 * 4], 40, 30)
    }

    fn export(tree: &Arc<Tree>, cache: Option<&mut FilterCache>, sizes: &[f32]) -> Vec<u8> {
        let mut document = Document::new();
        let mut cache = cache;
        for &width in sizes {
            let mut page = document.start_page();
            let mut surface = page.surface();
            let size = Size::from_wh(width, width * 0.75).unwrap();
            if let Some(cache) = cache.as_deref_mut() {
                surface
                    .draw_svg_cached(tree, size, SvgSettings::default(), cache)
                    .unwrap();
            } else {
                surface
                    .draw_svg(tree, size, SvgSettings::default())
                    .unwrap();
            }
            surface.finish();
        }
        document.finish().unwrap()
    }

    #[test]
    fn exact_raster_key_not_rounded_dimensions() {
        let tree = tree("red");
        let mut cache = FilterCache::default();
        let original = key(&tree, 1.0);
        cache.insert(original, &tree, image());
        for distinct in [
            RasterKey {
                width: original.width + 1,
                ..original
            },
            RasterKey {
                height: original.height + 1,
                ..original
            },
            RasterKey {
                filter_scale: original.filter_scale + 1,
                ..original
            },
        ] {
            assert!(cache.get(distinct).is_none());
        }
        for i in 0..6 {
            let mut distinct = original;
            distinct.transform[i] ^= 1;
            assert!(cache.get(distinct).is_none());
        }
        assert!(cache.get(original).is_some());
        assert!(cache
            .get(key(&tree, f32::from_bits(1.0f32.to_bits() + 1)))
            .is_none());
    }

    #[test]
    fn resolved_trees_with_same_ids_do_not_alias() {
        let red = tree("red");
        let blue = tree("blue");
        let mut cache = FilterCache::default();
        cache.insert(key(&red, 1.0), &red, image());
        assert!(cache.get(key(&blue, 1.0)).is_none());
        assert!(cache.get(key(&red, 1.0)).is_some());
        assert_eq!(Arc::strong_count(&red), 2);
    }

    #[test]
    fn lru_eviction_and_budget_bypass() {
        let tree = tree("red");
        let mut cache = FilterCache::new(2 * (40 * 30 * 8 + 1024), 2);
        cache.insert(key(&tree, 1.0), &tree, image());
        cache.insert(key(&tree, 2.0), &tree, image());
        cache.get(key(&tree, 1.0));
        cache.insert(key(&tree, 3.0), &tree, image());
        assert!(cache.get(key(&tree, 2.0)).is_none());
        assert!(cache.get(key(&tree, 1.0)).is_some());
        assert_eq!(cache.entries.len(), 2);
        assert!(cache.retained_bytes <= cache.max_bytes);
        let mut small = FilterCache::new(1, 64);
        small.insert(key(&tree, 1.0), &tree, image());
        assert!(small.entries.is_empty());
        assert_eq!(small.retained_bytes, 0);
    }

    #[test]
    fn repeated_and_alternating_sizes_produce_identical_pdf_bytes() {
        let tree = tree("red");
        let sizes = [40.0, 80.0, 40.0, 80.0];
        let expected = export(&tree, None, &sizes);
        let mut cache = FilterCache::default();
        assert_eq!(expected, export(&tree, Some(&mut cache), &sizes));
        assert_eq!(cache.entries.len(), 2);
    }

    #[test]
    fn independent_exports_and_disabled_cache_preserve_artwork() {
        let tree = tree("red");
        let expected = export(&tree, None, &[40.0, 40.0]);
        for _ in 0..2 {
            let mut cache = FilterCache::default();
            assert_eq!(expected, export(&tree, Some(&mut cache), &[40.0, 40.0]));
            assert_eq!(cache.entries.len(), 1);
        }
        let mut disabled = FilterCache::new(0, 64);
        assert_eq!(expected, export(&tree, Some(&mut disabled), &[40.0, 40.0]));
        assert!(disabled.entries.is_empty());
        assert_eq!(disabled.retained_bytes, 0);
    }

    #[test]
    fn subpixel_filtered_artwork_preserves_existing_export_behavior() {
        let tiny = tree("red");
        let normal = Arc::new(Tree::from_str(r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="30"><defs><filter id="b"><feGaussianBlur stdDeviation="0.001"/></filter></defs><rect x="2" y="2" width="30" height="20" fill="blue"/><g filter="url(#b)"><rect x="3" y="3" width="0.01" height="0.01" fill="red"/></g></svg>"##, &Default::default()).unwrap());
        for (tree, width) in [(&tiny, 0.001), (&normal, 40.0)] {
            let expected = export(tree, None, &[width, width]);
            for budget in [0, 64 * 1024 * 1024] {
                let mut cache = FilterCache::new(budget, 64);
                assert_eq!(expected, export(tree, Some(&mut cache), &[width, width]));
                assert!(cache.entries.is_empty());
            }
        }
    }

    #[test]
    fn evicted_tree_rerenders_without_changing_output() {
        let red = tree("red");
        let blue = tree("blue");
        let draw = |mut cache: Option<&mut FilterCache>| {
            let mut document = Document::new();
            for tree in [&red, &blue, &red] {
                let mut page = document.start_page();
                let mut surface = page.surface();
                let size = Size::from_wh(40.0, 30.0).unwrap();
                if let Some(cache) = cache.as_deref_mut() {
                    surface
                        .draw_svg_cached(tree, size, SvgSettings::default(), cache)
                        .unwrap();
                } else {
                    surface
                        .draw_svg(tree, size, SvgSettings::default())
                        .unwrap();
                }
                surface.finish();
            }
            document.finish().unwrap()
        };
        let expected = draw(None);
        let mut cache = FilterCache::new(64 * 1024 * 1024, 1);
        assert_eq!(expected, draw(Some(&mut cache)));
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(Arc::strong_count(&blue), 1);
    }

    #[test]
    fn identical_svg_source_with_different_resolved_images_does_not_alias() {
        const RED: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207,
            192, 240, 31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66,
            96, 130,
        ];
        const BLUE: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 96, 96,
            248, 255, 31, 0, 3, 2, 1, 255, 230, 119, 11, 174, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66,
            96, 130,
        ];
        let resolve = |png: &[u8]| {
            let data = Arc::new(png.to_vec());
            let options = usvg::Options {
                image_href_resolver: usvg::ImageHrefResolver {
                    resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
                    resolve_string: Box::new(move |_, _| Some(usvg::ImageKind::PNG(data.clone()))),
                },
                ..Default::default()
            };
            Arc::new(Tree::from_str(r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="40" height="30"><defs><filter id="same"><feGaussianBlur stdDeviation="1"/></filter></defs><g filter="url(#same)"><image x="5" y="5" width="30" height="20" xlink:href="tile.png"/></g></svg>"##, &options).unwrap())
        };
        let red = resolve(RED);
        let blue = resolve(BLUE);
        assert_ne!(export(&red, None, &[40.0]), export(&blue, None, &[40.0]));
        let trees = [red.clone(), blue.clone(), red, blue];
        let draw = |mut cache: Option<&mut FilterCache>| {
            let mut document = Document::new();
            for tree in &trees {
                let mut page = document.start_page();
                let mut surface = page.surface();
                let size = Size::from_wh(40.0, 30.0).unwrap();
                if let Some(cache) = cache.as_deref_mut() {
                    surface
                        .draw_svg_cached(tree, size, SvgSettings::default(), cache)
                        .unwrap();
                } else {
                    surface
                        .draw_svg(tree, size, SvgSettings::default())
                        .unwrap();
                }
                surface.finish();
            }
            document.finish().unwrap()
        };
        let expected = draw(None);
        let mut cache = FilterCache::default();
        assert_eq!(expected, draw(Some(&mut cache)));
        assert_eq!(cache.entries.len(), 2);
    }
}
