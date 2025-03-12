mod shading {
    use crate::{rect_to_path, stops_with_1_solid, stops_with_2_solid_1, stops_with_3_solid_1};
    use krilla::paint::{LinearGradient, RadialGradient, SpreadMethod, SweepGradient};
    use krilla::path::Fill;
    use krilla::surface::Surface;
    use krilla::Page;
    use krilla_macros::{snapshot2, visreg2};
    use tiny_skia_path::NormalizedF32;

    #[visreg2(all)]
    fn linear_gradient_pad(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let gradient = LinearGradient {
            x1: 50.0,
            y1: 0.0,
            x2: 150.0,
            y2: 0.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: stops_with_2_solid_1(),
            anti_alias: false,
        };

        surface.fill_path(
            &path,
            Fill {
                paint: gradient.into(),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            },
        );
    }

    #[visreg2(all)]
    fn linear_gradient_repeat(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let gradient = LinearGradient {
            x1: 50.0,
            y1: 0.0,
            x2: 150.0,
            y2: 0.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Repeat,
            stops: stops_with_2_solid_1(),
            anti_alias: false,
        };

        surface.fill_path(
            &path,
            Fill {
                paint: gradient.into(),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            },
        );
    }

    #[visreg2(all)]
    fn sweep_gradient_pad(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let gradient = SweepGradient {
            cx: 100.0,
            cy: 100.0,
            start_angle: 0.0,
            end_angle: 90.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: stops_with_2_solid_1(),
            anti_alias: false,
        };

        surface.fill_path(
            &path,
            Fill {
                paint: gradient.into(),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            },
        );
    }

    #[visreg2(all)]
    fn sweep_gradient_repeat(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let gradient = SweepGradient {
            cx: 100.0,
            cy: 100.0,
            start_angle: 0.0,
            end_angle: 90.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Repeat,
            stops: stops_with_2_solid_1(),
            anti_alias: false,
        };

        surface.fill_path(
            &path,
            Fill {
                paint: gradient.into(),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            },
        );
    }

    // Should be turned into a solid color.
    #[snapshot2(single_page)]
    fn gradient_single_stop(page: &mut Page) {
        let mut surface = page.surface();

        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let gradient = RadialGradient {
            cx: 100.0,
            cy: 100.0,
            cr: 30.0,
            fx: 120.0,
            fy: 120.0,
            fr: 60.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: stops_with_1_solid(),
            anti_alias: false,
        };

        surface.fill_path(
            &path,
            Fill {
                paint: gradient.into(),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            },
        );
    }

    #[visreg2(all)]
    fn radial_gradient_pad(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let gradient = RadialGradient {
            cx: 100.0,
            cy: 100.0,
            cr: 30.0,
            fx: 120.0,
            fy: 120.0,
            fr: 60.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: stops_with_3_solid_1(),
            anti_alias: false,
        };

        surface.fill_path(
            &path,
            Fill {
                paint: gradient.into(),
                opacity: NormalizedF32::ONE,
                rule: Default::default(),
            },
        );
    }
}
