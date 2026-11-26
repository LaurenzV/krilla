use krilla::color::rgb;
use krilla::mask::{Mask, MaskType};
use krilla::num::NormalizedF32;
use krilla::page::Page;
use krilla::paint::Fill;
use krilla::surface::Surface;
use krilla_macros::{snapshot, visreg};

use crate::{basic_mask, green_fill, rect_to_path, red_fill};

fn mask_visreg_impl(mask_type: MaskType, surface: &mut Surface, color: rgb::Color) {
    let path = rect_to_path(0.0, 0.0, 160.0, 160.0);

    let mut stream_builder = surface.stream_builder();
    let mut sub_surface = stream_builder.surface();
    sub_surface.set_fill(Some(red_fill(0.2)));
    sub_surface.draw_path(&path);
    sub_surface.finish();
    let mask = Mask::new(stream_builder.finish(), mask_type);

    surface.push_mask(mask);
    surface.set_fill(Some(Fill {
        paint: color.into(),
        opacity: NormalizedF32::ONE,
        rule: Default::default(),
    }));
    surface.draw_path(&path);
    surface.pop();
}

#[visreg(all, width = 160, height = 160)]
pub fn mask_luminosity(surface: &mut Surface) {
    mask_visreg_impl(MaskType::Luminosity, surface, rgb::Color::new(0, 255, 0));
}

#[visreg(all, width = 160, height = 160)]
pub fn mask_alpha(surface: &mut Surface) {
    mask_visreg_impl(MaskType::Luminosity, surface, rgb::Color::new(0, 0, 128));
}

#[snapshot]
fn mask(page: &mut Page) {
    let mut surface = page.surface();
    let mask = basic_mask(&mut surface, MaskType::Alpha);
    surface.push_mask(mask);
    let path = rect_to_path(0.0, 0.0, 100.0, 100.0);
    surface.set_fill(Some(green_fill(0.5)));
    surface.draw_path(&path);
    surface.pop();
}
