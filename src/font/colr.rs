use crate::blend_mode::BlendMode;
use crate::canvas::{Canvas, Surface};
use crate::color::Color;
use crate::font::OutlineBuilder;
use crate::paint::{LinearGradient, Paint, RadialGradient, SpreadMethod, Stop, SweepGradient};
use crate::transform::TransformWrapper;
use crate::{Fill, FillRule};
use skrifa::color::{Brush, ColorPainter, ColorStop, CompositeMode};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::prelude::LocationRef;
use skrifa::raw::types::BoundingBox;
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use tiny_skia_path::{FiniteF32, NormalizedF32, Path, PathBuilder, Size, Transform};

struct ColrCanvas<'a> {
    font: &'a FontRef<'a>,
    clips: Vec<Vec<Path>>,
    transforms: Vec<Transform>,
    canvases: Vec<Canvas>,
    blend_modes: Vec<BlendMode>,
    size: u16,
}

impl<'a> ColrCanvas<'a> {
    pub fn new(font_ref: &'a FontRef<'a>) -> Self {
        let size = font_ref
            .metrics(skrifa::instance::Size::unscaled(), LocationRef::default())
            .units_per_em;
        let canvas = Canvas::new(Size::from_wh(size as f32, size as f32).unwrap());

        Self {
            font: font_ref,
            transforms: vec![Transform::identity()],
            clips: vec![vec![]],
            canvases: vec![canvas],
            blend_modes: vec![],
            size,
        }
    }
}

impl ColrCanvas<'_> {
    fn palette_index_to_color(&self, palette_index: u16, alpha: f32) -> (Color, NormalizedF32) {
        if palette_index != u16::MAX {
            let color = self
                .font
                .cpal()
                .unwrap()
                .color_records_array()
                .unwrap()
                .unwrap()[palette_index as usize];

            (
                Color::new_rgb(color.red, color.green, color.blue),
                NormalizedF32::new(alpha * color.alpha as f32 / 255.0).unwrap(),
            )
        } else {
            (Color::new_rgb(0, 0, 0), NormalizedF32::new(alpha).unwrap())
        }
    }

    fn stops(&self, stops: &[ColorStop]) -> Vec<Stop> {
        stops
            .iter()
            .map(|s| {
                let (color, alpha) = self.palette_index_to_color(s.palette_index, s.alpha);

                Stop {
                    offset: NormalizedF32::new(s.offset).unwrap(),
                    color,
                    opacity: alpha,
                }
            })
            .collect::<Vec<_>>()
    }
}

trait ExtendExt {
    fn to_spread_method(&self) -> SpreadMethod;
}

impl ExtendExt for skrifa::color::Extend {
    fn to_spread_method(&self) -> SpreadMethod {
        match self {
            skrifa::color::Extend::Pad => SpreadMethod::Pad,
            skrifa::color::Extend::Repeat => SpreadMethod::Repeat,
            skrifa::color::Extend::Reflect => SpreadMethod::Reflect,
            skrifa::color::Extend::Unknown => SpreadMethod::Pad,
        }
    }
}

impl ColorPainter for ColrCanvas<'_> {
    fn push_transform(&mut self, transform: skrifa::color::Transform) {
        let new_transform = self
            .transforms
            .last()
            .unwrap()
            .pre_concat(Transform::from_row(
                transform.xx,
                transform.yx,
                transform.xy,
                transform.yy,
                transform.dx,
                transform.dy,
            ));
        self.transforms.push(new_transform);
    }

    fn pop_transform(&mut self) {
        self.transforms.pop();
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        let mut old = self.clips.last().unwrap().clone();

        let mut glyph_builder = OutlineBuilder(PathBuilder::new());
        let outline_glyphs = self.font.outline_glyphs();
        let outline_glyph = outline_glyphs.get(glyph_id).unwrap();
        outline_glyph
            .draw(
                DrawSettings::unhinted(skrifa::instance::Size::unscaled(), LocationRef::default()),
                &mut glyph_builder,
            )
            .unwrap();
        let path = glyph_builder
            .finish()
            .unwrap()
            .transform(*self.transforms.last().unwrap())
            .unwrap();

        old.push(path);

        self.clips.push(old);
    }

    fn push_clip_box(&mut self, clip_box: BoundingBox<f32>) {
        let mut old = self.clips.last().unwrap().clone();

        let mut path_builder = PathBuilder::new();
        path_builder.move_to(clip_box.x_min, clip_box.y_min);
        path_builder.line_to(clip_box.x_min, clip_box.y_max);
        path_builder.line_to(clip_box.x_max, clip_box.y_max);
        path_builder.line_to(clip_box.x_max, clip_box.y_min);
        path_builder.close();

        let path = path_builder
            .finish()
            .unwrap()
            .transform(*self.transforms.last().unwrap())
            .unwrap();
        old.push(path);

        self.clips.push(old);
    }

    fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn fill(&mut self, brush: Brush<'_>) {
        if let Some(fill) = match brush {
            Brush::Solid {
                palette_index,
                alpha,
            } => {
                let (color, alpha) = self.palette_index_to_color(palette_index, alpha);
                Some(Fill {
                    paint: Paint::Color(color),
                    opacity: alpha,
                    rule: Default::default(),
                })
            }
            Brush::LinearGradient {
                p0,
                p1,
                color_stops,
                extend,
            } => {
                let linear = LinearGradient {
                    x1: FiniteF32::new(p0.x).unwrap(),
                    y1: FiniteF32::new(p0.y).unwrap(),
                    x2: FiniteF32::new(p1.x).unwrap(),
                    y2: FiniteF32::new(p1.y).unwrap(),
                    stops: self.stops(color_stops),
                    spread_method: extend.to_spread_method(),
                    transform: TransformWrapper(*self.transforms.last().unwrap()),
                };

                Some(Fill {
                    paint: Paint::LinearGradient(linear),
                    opacity: NormalizedF32::ONE,
                    rule: Default::default(),
                })
            }
            Brush::RadialGradient {
                c0,
                r0,
                c1,
                r1,
                color_stops,
                extend,
            } => {
                let radial = RadialGradient {
                    fx: FiniteF32::new(c0.x).unwrap(),
                    fy: FiniteF32::new(c0.y).unwrap(),
                    fr: FiniteF32::new(r0).unwrap(),
                    cx: FiniteF32::new(c1.x).unwrap(),
                    cy: FiniteF32::new(c1.y).unwrap(),
                    cr: FiniteF32::new(r1).unwrap(),
                    stops: self.stops(color_stops),
                    spread_method: extend.to_spread_method(),
                    transform: TransformWrapper(*self.transforms.last().unwrap()),
                };

                Some(Fill {
                    paint: Paint::RadialGradient(radial),
                    opacity: NormalizedF32::ONE,
                    rule: Default::default(),
                })
            }
            Brush::SweepGradient {
                c0,
                start_angle,
                end_angle,
                color_stops,
                extend,
            } => {
                if start_angle == end_angle
                    && (matches!(
                        extend,
                        skrifa::color::Extend::Reflect | skrifa::color::Extend::Repeat
                    ))
                {
                    None
                } else {
                    let sweep = SweepGradient {
                        cx: FiniteF32::new(c0.x).unwrap(),
                        cy: FiniteF32::new(c0.y).unwrap(),
                        start_angle: FiniteF32::new(start_angle).unwrap(),
                        end_angle: FiniteF32::new(end_angle).unwrap(),
                        stops: self.stops(color_stops),
                        spread_method: extend.to_spread_method(),
                        // COLR gradients run in the different direction
                        transform: TransformWrapper(*self.transforms.last().unwrap()),
                    };

                    Some(Fill {
                        paint: Paint::SweepGradient(sweep),
                        opacity: NormalizedF32::ONE,
                        rule: Default::default(),
                    })
                }
            }
        } {
            let canvas = self.canvases.last_mut().unwrap();

            let mut clipped = canvas.clipped_many(
                self.clips
                    .last()
                    .unwrap()
                    .iter()
                    .map(|p| (p.clone(), FillRule::NonZero))
                    .collect::<Vec<_>>(),
            );

            let mut path_builder = PathBuilder::new();
            path_builder.move_to(0.0, 0.0);
            path_builder.line_to(self.size as f32, 0.0);
            path_builder.line_to(self.size as f32, self.size as f32);
            path_builder.line_to(0.0, self.size as f32);
            path_builder.close();

            clipped.fill_path(path_builder.finish().unwrap(), Transform::identity(), fill);

            clipped.finish();
        }
    }

    fn push_layer(&mut self, composite_mode: CompositeMode) {
        let mode = match composite_mode {
            CompositeMode::SrcOver => BlendMode::SourceOver,
            CompositeMode::Screen => BlendMode::Screen,
            CompositeMode::Overlay => BlendMode::Overlay,
            CompositeMode::Darken => BlendMode::Darken,
            CompositeMode::Lighten => BlendMode::Lighten,
            CompositeMode::ColorDodge => BlendMode::ColorDodge,
            CompositeMode::ColorBurn => BlendMode::ColorBurn,
            CompositeMode::HardLight => BlendMode::HardLight,
            CompositeMode::SoftLight => BlendMode::SoftLight,
            CompositeMode::Difference => BlendMode::Difference,
            CompositeMode::Exclusion => BlendMode::Exclusion,
            CompositeMode::Multiply => BlendMode::Multiply,
            CompositeMode::HslHue => BlendMode::Hue,
            CompositeMode::SrcAtop => BlendMode::SourceAtop,
            CompositeMode::DestAtop => BlendMode::DestinationAtop,
            CompositeMode::HslColor => BlendMode::Color,
            CompositeMode::HslLuminosity => BlendMode::Luminosity,
            CompositeMode::HslSaturation => BlendMode::Saturation,
            CompositeMode::Clear => BlendMode::Clear,
            CompositeMode::Src => BlendMode::Source,
            CompositeMode::Dest => BlendMode::Destination,
            CompositeMode::DestOver => BlendMode::DestinationOver,
            CompositeMode::DestIn => BlendMode::DestinationIn,
            CompositeMode::SrcIn => BlendMode::SourceIn,
            CompositeMode::SrcOut => BlendMode::SourceOut,
            CompositeMode::DestOut => BlendMode::DestinationOut,
            CompositeMode::Xor => BlendMode::Xor,
            CompositeMode::Plus => BlendMode::Plus,
            CompositeMode::Unknown => BlendMode::SourceOver,
        };
        let canvas = Canvas::new(Size::from_wh(self.size as f32, self.size as f32).unwrap());
        self.blend_modes.push(mode);
        self.canvases.push(canvas);
    }

    fn pop_layer(&mut self) {
        let draw_canvas = self.canvases.pop().unwrap();

        let canvas = self.canvases.last_mut().unwrap();
        let mut blended = canvas.blended(self.blend_modes.pop().unwrap());
        let mut isolated = blended.isolated();
        isolated.draw_canvas(draw_canvas);
        isolated.finish();
        blended.finish();
    }
}

#[cfg(test)]
mod tests {
    use crate::canvas::{Canvas, Surface};
    use crate::font::colr::ColrCanvas;
    use crate::serialize::{PageSerialize, SerializeSettings};
    use skrifa::prelude::LocationRef;
    use skrifa::{FontRef, GlyphId, MetadataProvider};
    use tiny_skia_path::Size;

    fn single_glyph(font_ref: &FontRef, glyph: GlyphId) -> Canvas {
        let mut colr_canvas = ColrCanvas::new(&font_ref);

        let colr_glyphs = font_ref.color_glyphs();
        if let Some(colr_glyph) = colr_glyphs.get(glyph) {
            let _ = colr_glyph.paint(LocationRef::default(), &mut colr_canvas);
        }
        let canvas = colr_canvas.canvases.last().unwrap().clone();
        canvas
    }

    #[test]
    fn colr() {
        let font_data =
            std::fs::read("/Users/lstampfl/Programming/GitHub/krilla/test_glyphs-glyf_colr_1.ttf")
                .unwrap();
        let font_data = std::fs::read("/Library/Fonts/NotoColorEmoji-Regular.ttf").unwrap();
        let font_ref = FontRef::from_index(&font_data, 0).unwrap();
        let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), LocationRef::default());

        let glyphs = (0..=220).collect::<Vec<_>>();
        let glyphs = vec![
            2397, 2400, 2401, 2398, 2403, 2402, 3616, 2399, 2600, 2463, 2464, 2406, 2407, 2404,
            2591, 2410, 2571, 2421, 2420, 2083, 2423, 2422, 2593, 2408, 2424, 2425, 2572, 2426,
            2556, 2562, 2690, 2575, 2573, 2559, 2930, 2931, 2689, 2555, 2570, 2413, 2414, 2451,
            2412, 2415, 2465, 2441, 2567, 2409, 2417, 2439, 962, 2566, 2449, 2452, 2557, 2560,
            2565, 2576, 2569, 3789, 2596, 2597, 2929, 2693, 2595, 2695, 963, 2450, 2577, 2563,
            2594, 2599, 2411, 2558, 2631, 2692, 2418, 2428, 2462, 2757, 2443, 2444, 2447, 2448,
            2691, 2601, 2435, 2436, 2437, 2445, 2434, 2431, 2442, 2446, 2419, 2432, 2427, 2416,
            2438, 2440, 2592, 2433, 2430, 2429, 2574, 2405, 2239, 3472, 3473, 2261, 2564, 2234,
            2235, 2236, 3471, 2238, 2561, 2455, 2453, 2454, 2456, 2457, 2458, 2461, 2460, 2459,
            3579, 3580, 3581, 2874, 2985, 2916, 3162, 2928, 2880, 2956, 2962, 3168, 3030, 3102,
            3036, 2968, 2974, 2855, 2856, 2854, 2922, 2862, 3150, 3126, 2891, 2892, 3156, 2868,
            2997, 2998, 2210, 2937, 2898, 3132, 3060, 3114, 3120, 2986, 3108, 3024, 3144, 3143,
            2943, 3174, 2904, 3066, 2910, 2624, 2625, 3072, 3078, 2209, 2621, 2838, 3714, 3776,
            3777, 3701, 3700, 3450, 2208, 3452, 3451, 3790, 3477, 2231, 2633, 2215, 2216, 2632,
            2217, 2218, 2635, 1689, 1690, 1649, 386, 705, 1650, 387, 706, 1652, 389, 708, 1651,
            388, 707, 2226, 787, 786, 2634, 2229, 2230, 2470, 1048, 1047, 2471, 1065, 1064, 2466,
            980, 979, 2467, 997, 996, 2240, 868, 867, 2469, 1031, 1030, 2630, 1420, 1419, 2468,
            1014, 1013, 2568, 1173, 1172, 2582, 1222, 1221, 1661, 394, 713, 1635, 358, 682, 1638,
            361, 685, 1662, 395, 714, 1631, 355, 679, 1632, 356, 680, 1642, 380, 699, 1639, 362,
            686, 1641, 379, 698, 1643, 381, 700, 1640, 378, 697, 1636, 359, 683, 1637, 360, 684,
            1663, 396, 715, 1644, 382, 701, 1645, 383, 702, 2223, 751, 750, 2381, 956, 955, 2241,
            885, 884, 2598, 2232, 841, 840, 2684, 2579, 2233, 2228, 809, 808, 2227, 2636, 2580,
            1200, 1199, 2225, 770, 769, 3054, 3053, 3052, 2578, 681, 357, 1633, 2237, 2137, 2581,
            1634, 2619, 1329, 1328, 2620, 1346, 1345, 2639, 1746, 1745, 2640, 1763, 1762, 2641,
            1780, 1779, 2642, 1797, 1796, 2643, 1814, 1813, 2644, 1816, 1815, 2645, 1818, 1817,
            2627, 2243, 907, 906, 2244, 924, 923, 2512, 1149, 1147, 2628, 1368, 1367, 2629, 1401,
            1399, 1647, 384, 703, 1653, 390, 709, 1655, 392, 711, 2161, 45, 43, 1151, 1148, 1150,
            1403, 1400, 1402, 1648, 385, 704, 1654, 391, 710, 1656, 393, 712, 47, 44, 46, 2242,
            2382, 2380, 2224, 753, 752, 2637, 1712, 1711, 3096, 3095, 3094, 2585, 2163, 2826, 2825,
            2166, 120, 119, 2162, 64, 63, 2497, 1082, 1081, 2164, 86, 85, 2782, 1841, 1840, 2165,
            103, 102, 2510, 1099, 1098, 2511, 1116, 1115, 2583, 1239, 1238, 2586, 1273, 1272, 2587,
            1290, 1289, 2588, 1307, 1306, 2584, 1256, 1255, 2638, 1729, 1728, 2521, 2949, 1646,
            2222, 2220, 2221, 2248, 718, 398, 719, 2249, 716, 397, 717, 2219, 373, 375, 376, 374,
            377, 368, 370, 371, 369, 372, 692, 694, 695, 693, 696, 363, 364, 365, 366, 367, 687,
            688, 689, 690, 691, 3571, 3469, 3470, 3778, 3468, 3439, 3416, 3673, 2615, 3440, 3418,
            3695, 122, 2204, 3444, 3671, 3685, 2206, 3406, 121, 2604, 3434, 3404, 2195, 3437, 3412,
            3666, 3780, 3677, 3672, 3779, 3694, 2205, 3402, 2194, 3403, 3441, 2197, 3419, 3447,
            3413, 3415, 3414, 3430, 3431, 3681, 3676, 2198, 2613, 3674, 3683, 3433, 3401, 3400,
            3443, 3435, 3405, 3449, 2616, 3678, 2606, 3445, 123, 3429, 3446, 2614, 3690, 3691,
            3680, 3688, 3448, 2605, 2196, 3417, 3427, 2202, 2203, 2832, 3428, 2831, 3555, 3667,
            3668, 3775, 3689, 3670, 3566, 3692, 3682, 3684, 2830, 3774, 3768, 3771, 3772, 2590,
            3442, 3408, 3426, 2608, 3411, 3436, 3407, 2610, 2611, 2207, 3409, 3432, 2617, 3423,
            3424, 3425, 3669, 3773, 2199, 2609, 2603, 2612, 3675, 3770, 3693, 3420, 3410, 2607,
            2200, 3421, 3422, 2681, 2201, 3679, 3766, 3438, 3559, 3665, 3686, 3765, 2680, 3687,
            3277, 2818, 3482, 3497, 3395, 3269, 3268, 3266, 3618, 3267, 3264, 3265, 3769, 2683,
            3257, 2682, 3390, 3258, 3259, 3256, 3260, 3262, 3341, 3343, 3271, 3272, 3801, 3273,
            3274, 3275, 3276, 3233, 3234, 3235, 2121, 2122, 2123, 2124, 2125, 2126, 2127, 2128,
            3237, 2129, 2130, 2131, 3244, 2132, 2133, 3748, 3495, 3239, 3238, 3828, 2134, 2748,
            2120, 3794, 3245, 3249, 3241, 3242, 3243, 3246, 3247, 3248, 3250, 3251, 3252, 3227,
            3230, 3228, 3795, 3796, 3821, 3812, 3830, 3797, 3798, 3388, 3389, 3575, 3232, 3544,
            3493, 3231, 3491, 3492, 3494, 3598, 3726, 3791, 3792, 2726, 2735, 2736, 2737, 2379,
            3240, 3574, 3724, 3338, 3339, 3720, 3340, 2140, 2141, 2142, 2677, 3344, 3345, 3335,
            3336, 3349, 2146, 2152, 2338, 2663, 3735, 2154, 3560, 3360, 3362, 3802, 3721, 2652,
            3746, 2664, 2388, 3355, 3730, 3758, 2651, 3759, 3455, 3456, 3662, 2602, 3702, 3457,
            3458, 2211, 3716, 3717, 2647, 3718, 3459, 2212, 3661, 3737, 3738, 3739, 2213, 3460,
            2214, 3461, 3462, 3597, 3346, 3740, 3463, 3464, 3663, 3664, 3465, 3466, 3736, 3467,
            3453, 3454, 3356, 3347, 3715, 3752, 3817, 2311, 3474, 2246, 2247, 3522, 2293, 2299,
            3350, 3351, 2144, 2149, 3354, 2308, 2155, 2668, 2156, 2157, 2158, 2159, 2667, 3747,
            2665, 2589, 3753, 2675, 3530, 3531, 3799, 3521, 2290, 2291, 3535, 3536, 2318, 2271,
            2386, 2387, 3793, 3564, 3565, 3502, 2273, 3503, 3504, 3727, 2150, 2145, 2310, 2153,
            2307, 2304, 2305, 2306, 2309, 3537, 3538, 3557, 3488, 2336, 3394, 3751, 2283, 2284,
            2285, 2286, 2287, 2288, 3515, 2282, 2281, 3507, 2289, 3508, 2300, 3570, 2280, 3541,
            3396, 3499, 2669, 2265, 2266, 2267, 2268, 2269, 2264, 3764, 2655, 3481, 3478, 3479,
            3719, 3529, 3480, 3526, 3527, 3528, 2295, 2294, 2296, 2297, 2298, 2395, 3520, 2786,
            2384, 2383, 3562, 3563, 3519, 2272, 3505, 3506, 3567, 2274, 2275, 2391, 2392, 3509,
            2276, 2277, 2278, 3510, 3511, 3512, 3513, 3561, 2279, 3514, 2784, 2389, 2390, 3568,
            2321, 2322, 2319, 2320, 3539, 3569, 3546, 3750, 3816, 3805, 3547, 2394, 3807, 2260,
            3550, 3745, 3398, 2533, 3754, 3545, 3755, 3548, 3810, 2393, 2772, 2618, 3542, 1824,
            3818, 3756, 2649, 2650, 2670, 3602, 2532, 2687, 3809, 2648, 3722, 3723, 2337, 3551,
            2292, 2660, 3476, 3741, 2245, 3742, 2659, 2661, 2501, 2530, 3757, 2671, 2950, 2527,
            3749, 2673, 2519, 2672, 3596, 2522, 2653, 2694, 2666, 2678, 3729, 3731, 3732, 3733,
            3734, 3760, 2674, 2654, 3725, 3603, 2529, 3595, 3814, 3761, 3815, 3399, 3762, 3558,
        ];

        let num_glyphs = glyphs.len();

        let width = 2000;

        let size = 150u32;
        let num_cols = width / size;
        let height = (num_glyphs as f32 / num_cols as f32).ceil() as u32 * size;
        let units_per_em = metrics.units_per_em as f32;
        let mut cur_point = 0;

        let mut parent_canvas = Canvas::new(Size::from_wh(width as f32, height as f32).unwrap());

        for i in glyphs.iter().copied() {
            let canvas = single_glyph(&font_ref, GlyphId::new(i));

            fn get_transform(
                cur_point: u32,
                size: u32,
                num_cols: u32,
                units_per_em: f32,
            ) -> crate::Transform {
                let el = cur_point / size;
                let col = el % num_cols;
                let row = el / num_cols;

                crate::Transform::from_row(
                    (1.0 / units_per_em) * size as f32,
                    0.0,
                    0.0,
                    (1.0 / units_per_em) * size as f32,
                    col as f32 * size as f32,
                    row as f32 * size as f32,
                )
            }

            let mut transformed = parent_canvas.transformed(
                get_transform(cur_point, size, num_cols, units_per_em).pre_concat(
                    tiny_skia_path::Transform::from_row(
                        1.0,
                        0.0,
                        0.0,
                        -1.0,
                        0.0,
                        units_per_em as f32,
                    ),
                ),
            );
            transformed.draw_canvas(canvas);
            transformed.finish();

            cur_point += size;
        }

        let pdf = parent_canvas.serialize(SerializeSettings::default());
        let finished = pdf.finish();
        let _ = std::fs::write("out/colr.pdf", &finished);
        let _ = std::fs::write("out/colr.txt", &finished);
    }
}
