mod shading {
    use krilla::num::NormalizedF32;
    use krilla::page::Page;
    use krilla::paint::{Fill, LinearGradient, RadialGradient, SpreadMethod, SweepGradient};
    use krilla::surface::Surface;
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

        surface.set_fill(Some(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        }));
        surface.draw_path(&path);
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

        surface.set_fill(Some(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        }));
        surface.draw_path(&path);
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

        surface.set_fill(Some(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        }));
        surface.draw_path(&path);
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

        surface.set_fill(Some(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        }));
        surface.draw_path(&path);
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

        surface.set_fill(Some(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        }));
        surface.draw_path(&path);
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

        surface.set_fill(Some(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        }));
        surface.draw_path(&path);
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

        surface.set_fill(Some(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        }));
        surface.draw_path(&path);
    }
}

mod tiling {
    use krilla::num::NormalizedF32;
    use krilla::paint::{Fill, Pattern};
    use krilla::surface::Surface;
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

        surface.set_fill(Some(Fill {
            paint: pattern.into(),
            opacity: NormalizedF32::new(0.5).unwrap(),
            rule: Default::default(),
        }));
        surface.draw_path(&path)
    }
}

#[cfg(test)]
mod gradient_robustness {
    use krilla::geom::PathBuilder;
    use krilla::num::NormalizedF32;
    use krilla::paint::{Fill, LinearGradient, RadialGradient, SpreadMethod, SweepGradient};
    use krilla::{Document, SerializeSettings};

    fn rect_path() -> krilla::geom::Path {
        let mut b = PathBuilder::new();
        b.move_to(10.0, 10.0);
        b.line_to(90.0, 10.0);
        b.line_to(90.0, 90.0);
        b.line_to(10.0, 90.0);
        b.close();
        b.finish().unwrap()
    }

    /// An empty `stops` vector on a gradient must not panic the serializer —
    /// the gradient is simply not emitted.
    #[test]
    fn linear_gradient_with_empty_stops_does_not_panic() {
        let mut document = Document::new_with(SerializeSettings::default());
        let mut page = document.start_page();
        let mut surface = page.surface();
        let gradient = LinearGradient {
            x1: 0.0,
            y1: 0.0,
            x2: 100.0,
            y2: 0.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: Vec::new(),
            anti_alias: false,
        };
        surface.set_fill(Some(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        }));
        surface.draw_path(&rect_path());
        surface.finish();
        page.finish();
        document.finish().expect("serialisation must succeed");
    }

    #[test]
    fn radial_gradient_with_empty_stops_does_not_panic() {
        let mut document = Document::new_with(SerializeSettings::default());
        let mut page = document.start_page();
        let mut surface = page.surface();
        let gradient = RadialGradient {
            fx: 50.0,
            fy: 50.0,
            fr: 0.0,
            cx: 50.0,
            cy: 50.0,
            cr: 50.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: Vec::new(),
            anti_alias: false,
        };
        surface.set_fill(Some(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        }));
        surface.draw_path(&rect_path());
        surface.finish();
        page.finish();
        document.finish().expect("serialisation must succeed");
    }

    #[test]
    fn sweep_gradient_with_empty_stops_does_not_panic() {
        let mut document = Document::new_with(SerializeSettings::default());
        let mut page = document.start_page();
        let mut surface = page.surface();
        let gradient = SweepGradient {
            cx: 50.0,
            cy: 50.0,
            start_angle: 0.0,
            end_angle: 360.0,
            transform: Default::default(),
            spread_method: SpreadMethod::Pad,
            stops: Vec::new(),
            anti_alias: false,
        };
        surface.set_fill(Some(Fill {
            paint: gradient.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        }));
        surface.draw_path(&rect_path());
        surface.finish();
        page.finish();
        document.finish().expect("serialisation must succeed");
    }
}
