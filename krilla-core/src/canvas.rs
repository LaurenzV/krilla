use crate::color::PdfColorExt;
use crate::paint::Paint;
use crate::resource::ResourceDictionary;
use crate::util::{LineCapExt, LineJoinExt, NameExt, TransformExt};
use crate::Stroke;
use pdf_writer::Content;
use tiny_skia_path::{Path, PathSegment};

pub struct Canvas {
    content: Content,
    resource_dictionary: ResourceDictionary,
    q_nesting: u8,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            content: Content::new(),
            resource_dictionary: ResourceDictionary::new(),
            q_nesting: 0,
        }
    }

    fn transform(&mut self, transform: &tiny_skia_path::Transform) {
        if !transform.is_identity() {
            self.content.transform(transform.to_pdf_transform());
        }
    }

    fn save_state(&mut self) {
        self.content.save_state();
        self.q_nesting.checked_add(1).unwrap();
    }

    fn restore_state(&mut self) {
        self.content.save_state();
        self.q_nesting.checked_sub(1).unwrap();
    }

    pub fn stroke_path(
        &mut self,
        path: &Path,
        transform: &tiny_skia_path::Transform,
        stroke: &Stroke,
    ) {
        self.save_state();
        self.transform(transform);

        match &stroke.paint {
            Paint::Color(c) => {
                let color_space = self
                    .resource_dictionary
                    .register_color_space(c.get_pdf_color_space());
                self.content
                    .set_stroke_color_space(color_space.to_pdf_name());
                self.content.set_stroke_color(c.to_pdf_components());
            }
            Paint::LinearGradient(_) => unimplemented!(),
            Paint::RadialGradient(_) => unimplemented!(),
        }
        self.content.set_line_width(stroke.width.get());
        self.content.set_miter_limit(stroke.miter_limit.get());
        self.content.set_line_cap(stroke.line_cap.to_pdf_line_cap());
        self.content
            .set_line_join(stroke.line_join.to_pdf_line_join());

        if let Some(stroke_dash) = &stroke.dash {
            self.content
                .set_dash_pattern(stroke_dash.array.iter().cloned(), stroke_dash.offset);
        } else {
            self.content.set_dash_pattern(vec![], 0.0);
        }

        draw_path(path.segments(), &mut self.content);
        self.content.stroke();

        self.restore_state();
    }
}

/// Draws a path into a content stream. Note that this does not perform any stroking/filling,
/// it only creates a subpath.
fn draw_path(path_data: impl Iterator<Item = PathSegment>, content: &mut Content) {
    // Taken from resvg
    fn calc(n1: f32, n2: f32) -> f32 {
        (n1 + n2 * 2.0) / 3.0
    }

    let mut p_prev = None;

    for operation in path_data {
        match operation {
            PathSegment::MoveTo(p) => {
                content.move_to(p.x, p.y);
                p_prev = Some(p);
            }
            PathSegment::LineTo(p) => {
                content.line_to(p.x, p.y);
                p_prev = Some(p);
            }
            PathSegment::QuadTo(p1, p2) => {
                // Since PDF doesn't support quad curves, we need to convert them into
                // cubic.
                let prev = p_prev.unwrap();
                content.cubic_to(
                    calc(prev.x, p1.x),
                    calc(prev.y, p1.y),
                    calc(p2.x, p1.x),
                    calc(p2.y, p1.y),
                    p2.x,
                    p2.y,
                );
                p_prev = Some(p2);
            }
            PathSegment::CubicTo(p1, p2, p3) => {
                content.cubic_to(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y);
                p_prev = Some(p3);
            }
            PathSegment::Close => {
                content.close_path();
            }
        };
    }
}

