pub struct Canvas {
    content: pdf_writer::Content,
    q_nesting: u8,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            content: pdf_writer::Content::new(),
            q_nesting: 0,
        }
    }

    fn transform(&mut self, transform: tiny_skia_path::Transform) {
        if !transform.is_identity() {
            self.content.transform(transform.to_pdf_transform());
        }
    }

    fn save_state(&mut self) {
        self.content.save_state();
    }

    fn restore_state(&mut self) {
        self.content.save_state();
    }
}

trait TransformExt {
    fn to_pdf_transform(&self) -> [f32; 6];
}

impl TransformExt for tiny_skia_path::Transform {
    fn to_pdf_transform(&self) -> [f32; 6] {
        [self.sx, self.ky, self.kx, self.sy, self.tx, self.ty]
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::canvas::Canvas;
//     use tiny_skia_path::Transform;
//
//     macro_rules! equals_str {
//         ($expected:expr, $actual:expr) => {
//             assert_eq!($expected, std::str::from_utf8(&$actual).unwrap());
//         };
//     }
//
//     #[test]
//     fn dont_write_identity_transform() {
//         let mut content = Canvas::new();
//         content.transform(Transform::identity());
//         assert!(content.finish().is_empty());
//     }
//
//     #[test]
//     fn basic_transform() {
//         let mut content = Canvas::new();
//         content.transform(Transform::from_scale(2.0, 2.0));
//         equals_str!("2 0 0 2 0 0 cm", content.finish());
//     }
//
//     #[test]
//     fn complex_transform() {
//         let mut content = Canvas::new();
//         content.transform(Transform::from_row(1.5, 2.1, 2.0, 1.3, 0.5, 0.4));
//         equals_str!("1.5 2.1 2 1.3 0.5 0.4 cm", content.finish());
//     }
// }
