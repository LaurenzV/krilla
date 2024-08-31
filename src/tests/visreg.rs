use crate::color::rgb;
use crate::color::rgb::Rgb;
use crate::paint::{LinearGradient, Paint, SpreadMethod, Stop};
use crate::path::Fill;
use crate::surface::Surface;
use krilla_macros::visreg;
use tiny_skia_path::{NormalizedF32, PathBuilder, Rect, Transform};

#[visreg(all)]
fn linear_gradient(surface: &mut Surface) {
    let mut builder = PathBuilder::new();
    builder.push_rect(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap());
    let path = builder.finish().unwrap();

    let gradient = LinearGradient {
        x1: 20.0,
        y1: 0.0,
        x2: 180.0,
        y2: 0.0,
        transform: Transform::identity(),
        spread_method: SpreadMethod::Pad,
        stops: vec![
            Stop::<Rgb> {
                offset: NormalizedF32::new(0.0).unwrap(),
                color: rgb::Color::new(255, 0, 0),
                opacity: NormalizedF32::new(1.0).unwrap(),
            },
            Stop {
                offset: NormalizedF32::new(0.5).unwrap(),
                color: rgb::Color::new(0, 255, 0),
                opacity: NormalizedF32::new(0.5).unwrap(),
            },
            Stop {
                offset: NormalizedF32::new(1.0).unwrap(),
                color: rgb::Color::new(0, 0, 255),
                opacity: NormalizedF32::new(1.0).unwrap(),
            },
        ],
    };

    surface.fill_path(
        &path,
        Fill {
            paint: Paint::LinearGradient(gradient),
            opacity: NormalizedF32::new(0.5).unwrap(),
            rule: Default::default(),
        },
    );
}
