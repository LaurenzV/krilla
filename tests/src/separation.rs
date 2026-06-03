use krilla::color::{rgb, separation};
use krilla::num::NormalizedF32;
use krilla::page::Page;
use krilla::paint::Fill;
use krilla::surface::Surface;
use krilla_macros::{snapshot, visreg};

use crate::rect_to_path;

fn spot_fill(
    tint: u8,
    colorant: separation::SeparationColorant,
    fallback: rgb::Color,
    opacity: f32,
) -> Fill {
    let space = separation::SeparationSpace::new(colorant, fallback.into());
    let color: krilla::color::Color = separation::Color::new(tint, space).into();
    Fill {
        paint: color.into(),
        opacity: NormalizedF32::new(opacity).unwrap(),
        rule: Default::default(),
    }
}

#[snapshot]
fn separation_full_tint(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    // Full tint (255) of a custom spot color with red fallback
    let fill = spot_fill(
        255,
        separation::SeparationColorant::Custom("PANTONE 185 C".to_string()),
        rgb::Color::new(255, 0, 0),
        1.0,
    );

    surface.set_fill(Some(fill));
    surface.draw_path(&path);
}

#[snapshot]
fn separation_half_tint(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    // Half tint (127) of a custom spot color with blue fallback
    let fill = spot_fill(
        127,
        separation::SeparationColorant::Custom("PANTONE 286 C".to_string()),
        rgb::Color::new(0, 0, 255),
        1.0,
    );

    surface.set_fill(Some(fill));
    surface.draw_path(&path);
}

#[snapshot]
fn separation_no_tint(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    // No tint (0) should produce white/transparent
    let fill = spot_fill(
        0,
        separation::SeparationColorant::Custom("PANTONE Green".to_string()),
        rgb::Color::new(0, 255, 0),
        1.0,
    );

    surface.set_fill(Some(fill));
    surface.draw_path(&path);
}

#[snapshot]
fn separation_with_opacity(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    // Full tint with 50% opacity
    let fill = spot_fill(
        255,
        separation::SeparationColorant::Custom("PANTONE Orange".to_string()),
        rgb::Color::new(255, 127, 0),
        0.5,
    );

    surface.set_fill(Some(fill));
    surface.draw_path(&path);
}

#[snapshot]
fn separation_all_colorants(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    // Use the special "All" colorant
    let fill = spot_fill(
        255,
        separation::SeparationColorant::AllColorants,
        rgb::Color::new(0, 0, 0),
        1.0,
    );

    surface.set_fill(Some(fill));
    surface.draw_path(&path);
}

#[snapshot]
fn separation_no_colorant(page: &mut Page) {
    let mut surface = page.surface();
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);

    // Use the special "None" colorant (used for cuts, varnish, etc.)
    let fill = spot_fill(
        255,
        separation::SeparationColorant::NoColorant,
        rgb::Color::new(200, 200, 200),
        1.0,
    );

    surface.set_fill(Some(fill));
    surface.draw_path(&path);
}
