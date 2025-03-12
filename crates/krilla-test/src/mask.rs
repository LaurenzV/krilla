use crate::{basic_mask, rect_to_path};
use krilla::color::rgb;
use krilla::mask::MaskType;
use krilla::path::Fill;
use krilla::surface::Surface;
use krilla::NormalizedF32;
use krilla_macros::visreg2;

fn mask_visreg_impl(mask_type: MaskType, surface: &mut Surface, color: rgb::Color) {
    let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
    let mask = basic_mask(surface, mask_type);
    surface.push_mask(mask);
    surface.fill_path(
        &path,
        Fill {
            paint: color.into(),
            opacity: NormalizedF32::ONE,
            rule: Default::default(),
        },
    );
    surface.pop();
}

#[visreg2(all)]
pub fn mask_luminosity(surface: &mut Surface) {
    mask_visreg_impl(MaskType::Luminosity, surface, rgb::Color::new(0, 255, 0));
}

#[visreg2(all)]
pub fn mask_alpha(surface: &mut Surface) {
    mask_visreg_impl(MaskType::Luminosity, surface, rgb::Color::new(0, 0, 128));
}
