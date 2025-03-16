use krilla::color::rgb;
use krilla::mask::MaskType;
use krilla::path::Fill;
use krilla::surface::Surface;
use krilla::{NormalizedF32, Page};
use krilla_macros::{snapshot, visreg};

use crate::{basic_mask, green_fill, rect_to_path};

fn mask_visreg_impl(mask_type: MaskType, surface: &mut Surface, color: rgb::Color) {
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
    let mask = basic_mask(surface, mask_type);
    surface.push_mask(mask);
    surface.set_fill(Fill {
        paint: color.into(),
        opacity: NormalizedF32::ONE,
        rule: Default::default(),
    });
    surface.fill_path(&path);
    surface.pop();
}

#[visreg(all)]
pub fn mask_luminosity(surface: &mut Surface) {
    mask_visreg_impl(MaskType::Luminosity, surface, rgb::Color::new(0, 255, 0));
}

#[visreg(all)]
pub fn mask_alpha(surface: &mut Surface) {
    mask_visreg_impl(MaskType::Luminosity, surface, rgb::Color::new(0, 0, 128));
}

#[snapshot]
fn mask(page: &mut Page) {
    let mut surface = page.surface();
    let mask = basic_mask(&mut surface, MaskType::Alpha);
    surface.push_mask(mask);
    let path = rect_to_path(0.0, 0.0, 100.0, 100.0);
    surface.set_fill(green_fill(0.5));
    surface.fill_path(&path);
    surface.pop();
}
