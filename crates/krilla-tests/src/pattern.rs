mod shading {
    use krilla::graphics::paint::{
        Fill, LinearGradient, RadialGradient, SpreadMethod, SweepGradient,
    };
    use krilla::page::Page;
    use krilla::surface::Surface;
    use krilla::NormalizedF32;
    use krilla_macros::{snapshot, visreg};

    use crate::{
        rect_to_path, stops_with_1_solid, stops_with_2_solid_1, stops_with_3_luma,
        stops_with_3_solid_1,
    };

    #[visreg(all)]
    fn pattern_linear_gradient_pad(surface: &mut Surface) {
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

        surface.set_fill(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        });
        surface.fill_path(&path);
    }

    #[visreg(all)]
    fn pattern_linear_gradient_repeat(surface: &mut Surface) {
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

        surface.set_fill(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        });
        surface.fill_path(&path);
    }

    #[visreg(all)]
    fn pattern_sweep_gradient_pad(surface: &mut Surface) {
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

        surface.set_fill(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        });
        surface.fill_path(&path);
    }

    #[visreg(all)]
    fn pattern_sweep_gradient_repeat(surface: &mut Surface) {
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

        surface.set_fill(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        });
        surface.fill_path(&path);
    }

    #[visreg(all)]
    fn pattern_radial_gradient_pad(surface: &mut Surface) {
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

        surface.set_fill(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        });
        surface.fill_path(&path);
    }

    // Should be turned into a solid color.
    #[snapshot]
    fn pattern_gradient_single_stop(page: &mut Page) {
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

        surface.set_fill(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        });
        surface.fill_path(&path);
    }

    #[snapshot]
    fn pattern_luma_stops(page: &mut Page) {
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
            stops: stops_with_3_luma(),
            anti_alias: false,
        };

        surface.set_fill(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        });
        surface.fill_path(&path);
    }
}

mod tiling {
    use krilla::graphics::paint::{Fill, Pattern};
    use krilla::surface::Surface;
    use krilla::NormalizedF32;
    use krilla_macros::visreg;

    use crate::{basic_pattern_stream, rect_to_path};

    #[visreg(all)]
    fn pattern_tiling_basic(surface: &mut Surface) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let stream_builder = surface.stream_builder();
        let pattern_stream = basic_pattern_stream(stream_builder);

        let pattern = Pattern {
            stream: pattern_stream,
            transform: Default::default(),
            width: 20.0,
            height: 20.0,
        };

        surface.set_fill(Fill {
            paint: pattern.into(),
            opacity: NormalizedF32::new(0.5).unwrap(),
            rule: Default::default(),
        });
        surface.fill_path(&path)
    }
}
