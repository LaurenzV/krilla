use crate::bytecode::{ByteCode, Instruction};
use crate::color::PdfColorExt;
use crate::paint::Paint;
use crate::resource::ResourceDictionary;
use crate::serialize::{ObjectSerialize, PageSerialize, RefAllocator, SerializeSettings};
use crate::util::{LineCapExt, LineJoinExt, NameExt, RectExt, TransformExt};
use crate::{Fill, FillRule, LineCap, LineJoin, Stroke};
use pdf_writer::{Chunk, Content, Finish, Pdf, Ref};
use tiny_skia_path::{Path, PathSegment, Rect, Size, Transform};

pub struct Canvas {
    byte_code: ByteCode,
    size: Size,
}

impl Canvas {
    pub fn new(size: Size) -> Self {
        Self {
            byte_code: ByteCode::new(),
            size,
        }
    }

    pub fn stroke_path(
        &mut self,
        path: Path,
        transform: tiny_skia_path::Transform,
        stroke: Stroke,
    ) {
        self.byte_code
            .push(Instruction::StrokePath(Box::new((path, transform, stroke))));
    }

    pub fn fill_path(&mut self, path: Path, transform: tiny_skia_path::Transform, fill: Fill) {
        self.byte_code
            .push(Instruction::FillPath(Box::new((path, transform, fill))));
    }
}

pub struct CanvasPdfSerializer {
    resource_dictionary: ResourceDictionary,
    content: Content,
    bbox: Rect,
}

impl CanvasPdfSerializer {
    pub fn new() -> Self {
        Self {
            resource_dictionary: ResourceDictionary::new(),
            content: Content::new(),
            bbox: Rect::from_xywh(0.0, 0.0, 0.0, 0.0).unwrap(),
        }
    }

    pub fn transform(&mut self, transform: &tiny_skia_path::Transform) {
        if !transform.is_identity() {
            self.content.transform(transform.to_pdf_transform());
        }
    }

    pub fn finish(self) -> (Vec<u8>, ResourceDictionary, Rect) {
        (self.content.finish(), self.resource_dictionary, self.bbox)
    }

    pub fn save_state(&mut self) {
        self.content.save_state();
    }

    pub fn restore_state(&mut self) {
        self.content.restore_state();
    }

    pub fn fill_path(&mut self, path: &Path, transform: &tiny_skia_path::Transform, fill: &Fill) {
        let path_bbox = path.bounds().transform(*transform).unwrap();
        self.bbox.expand(&path_bbox);

        self.content.save_state();
        self.transform(transform);

        match &fill.paint {
            Paint::Color(c) => {
                let color_space = self
                    .resource_dictionary
                    .register_color_space(c.get_pdf_color_space());
                self.content.set_fill_color_space(color_space.to_pdf_name());
                self.content.set_fill_color(c.to_pdf_components());
            }
            Paint::LinearGradient(_) => unimplemented!(),
            Paint::RadialGradient(_) => unimplemented!(),
        }

        draw_path(path.segments(), &mut self.content);
        match fill.rule {
            FillRule::NonZero => self.content.fill_nonzero(),
            FillRule::EvenOdd => self.content.fill_even_odd(),
        };
        self.content.restore_state();
    }

    pub fn stroke_path(
        &mut self,
        path: &Path,
        transform: &tiny_skia_path::Transform,
        stroke: &Stroke,
    ) {
        let path_bbox = path.bounds().transform(*transform).unwrap();
        self.bbox.expand(&path_bbox);

        self.content.save_state();
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

        // Only write if they don't correspond to the default values as defined in the
        // PDF specification.
        if stroke.width.get() != 1.0 {
            self.content.set_line_width(stroke.width.get());
        }

        if stroke.miter_limit.get() != 10.0 {
            self.content.set_miter_limit(stroke.miter_limit.get());
        }

        if stroke.line_cap != LineCap::Butt {
            self.content.set_line_cap(stroke.line_cap.to_pdf_line_cap());
        }

        if stroke.line_join != LineJoin::Miter {
            self.content
                .set_line_join(stroke.line_join.to_pdf_line_join());
        }

        if let Some(stroke_dash) = &stroke.dash {
            self.content
                .set_dash_pattern(stroke_dash.array.iter().cloned(), stroke_dash.offset);
        }

        draw_path(path.segments(), &mut self.content);
        self.content.stroke();

        self.content.restore_state();
    }
}

impl ObjectSerialize for Canvas {
    fn serialize_into(
        self,
        chunk: &mut Chunk,
        ref_allocator: &mut RefAllocator,
        serialize_settings: &SerializeSettings,
    ) -> Ref {
        let root_ref = ref_allocator.new_ref();

        let (content_stream, mut resource_dictionary, bbox) = {
            let mut serializer = CanvasPdfSerializer::new();

            for op in self.byte_code.instructions() {
                match op {
                    Instruction::SaveState => {
                        serializer.save_state();
                    }
                    Instruction::RestoreState => {
                        serializer.restore_state();
                    }
                    Instruction::StrokePath(stroke_data) => {
                        serializer.stroke_path(&stroke_data.0, &stroke_data.1, &stroke_data.2);
                    }
                    Instruction::FillPath(fill_data) => {
                        serializer.fill_path(&fill_data.0, &fill_data.1, &fill_data.2);
                    }
                    Instruction::DrawCanvas(_) => todo!(),
                }
            }

            serializer.finish()
        };

        let mut x_object = chunk.form_xobject(root_ref, &content_stream);
        resource_dictionary.to_pdf_resources(ref_allocator, &mut x_object.resources());
        x_object.bbox(bbox.to_pdf_rect());
        x_object.finish();

        if serialize_settings.serialize_dependencies {
            for color_space in resource_dictionary.color_spaces.get_entries() {
                color_space
                    .1
                    .serialize_into(chunk, ref_allocator, serialize_settings);
            }
        }

        root_ref
    }
}

impl PageSerialize for Canvas {
    fn serialize(self, serialize_settings: &SerializeSettings) -> Pdf {
        let mut ref_allocator = RefAllocator::new();

        let catalog_ref = ref_allocator.new_ref();
        let page_tree_ref = ref_allocator.new_ref();
        let page_ref = ref_allocator.new_ref();
        let content_ref = ref_allocator.new_ref();

        let mut chunk = Chunk::new();

        chunk.pages(page_tree_ref).count(1).kids([page_ref]);

        let (content_stream, mut resource_dictionary, _) = {
            let mut serializer = CanvasPdfSerializer::new();
            serializer.transform(&Transform::from_row(
                1.0,
                0.0,
                0.0,
                -1.0,
                0.0,
                self.size.height(),
            ));

            for op in self.byte_code.instructions() {
                match op {
                    Instruction::SaveState => {
                        serializer.save_state();
                    }
                    Instruction::RestoreState => {
                        serializer.restore_state();
                    }
                    Instruction::StrokePath(stroke_data) => {
                        serializer.stroke_path(&stroke_data.0, &stroke_data.1, &stroke_data.2)
                    }
                    Instruction::FillPath(fill_data) => {
                        serializer.fill_path(&fill_data.0, &fill_data.1, &fill_data.2);
                    }
                    Instruction::DrawCanvas(_) => todo!(),
                }
            }

            serializer.finish()
        };

        if serialize_settings.serialize_dependencies {
            for color_space in resource_dictionary.color_spaces.get_entries() {
                color_space
                    .1
                    .serialize_into(&mut chunk, &mut ref_allocator, serialize_settings);
            }
        }

        chunk.stream(content_ref, &content_stream);

        let mut page = chunk.page(page_ref);
        resource_dictionary.to_pdf_resources(&mut ref_allocator, &mut page.resources());

        page.media_box(self.size.to_rect(0.0, 0.0).unwrap().to_pdf_rect());
        page.parent(page_tree_ref);
        page.contents(content_ref);
        page.finish();

        let mut pdf = Pdf::new();
        pdf.catalog(catalog_ref).pages(page_tree_ref);
        pdf.extend(&chunk);

        pdf
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

#[cfg(test)]
mod tests {
    use crate::canvas::Canvas;
    use crate::color::Color;
    use crate::paint::Paint;
    use crate::serialize::{ObjectSerialize, SerializeSettings};
    use crate::{Fill, Stroke};
    use tiny_skia_path::{Path, PathBuilder, Size, Transform};

    fn dummy_path() -> Path {
        let mut builder = PathBuilder::new();
        builder.move_to(0.0, 0.0);
        builder.line_to(100.0, 0.0);
        builder.line_to(100.0, 100.0);
        builder.line_to(0.0, 100.0);
        builder.close();

        builder.finish().unwrap()
    }

    #[test]
    fn serialize_canvas_1() {
        let mut canvas = Canvas::new(Size::from_wh(100.0, 100.0).unwrap());
        canvas.stroke_path(
            dummy_path(),
            Transform::from_scale(2.0, 2.0),
            Stroke::default(),
        );

        let chunk = canvas.serialize(&SerializeSettings::default()).0;
        std::fs::write("out/serialize_canvas_1.txt", chunk.as_bytes());
    }

    #[test]
    fn serialize_canvas_stroke() {
        let mut canvas = Canvas::new(Size::from_wh(100.0, 100.0).unwrap());
        canvas.stroke_path(
            dummy_path(),
            Transform::from_scale(2.0, 2.0),
            Stroke::default(),
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = canvas.serialize(&serialize_settings).0;
        std::fs::write("out/serialize_canvas_stroke.txt", chunk.as_bytes());
    }

    #[test]
    fn serialize_canvas_page() {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(100.0, 100.0).unwrap());
        canvas.stroke_path(
            dummy_path(),
            Transform::from_scale(0.5, 0.5),
            Stroke::default(),
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = PageSerialize::serialize(canvas, &serialize_settings);
        let finished = chunk.finish();
        std::fs::write("out/serialize_canvas_page.txt", &finished);
        std::fs::write("out/serialize_canvas_page.pdf", &finished);
    }

    #[test]
    fn serialize_canvas_fill() {
        use crate::serialize::PageSerialize;
        let mut canvas = Canvas::new(Size::from_wh(100.0, 100.0).unwrap());
        canvas.fill_path(
            dummy_path(),
            Transform::from_scale(2.0, 2.0),
            Fill {
                paint: Paint::Color(Color::new_rgb(200, 0, 0)),
                ..Fill::default()
            },
        );

        let serialize_settings = SerializeSettings {
            serialize_dependencies: true,
        };

        let chunk = PageSerialize::serialize(canvas, &serialize_settings);
        let finished = chunk.finish();

        std::fs::write("out/serialize_canvas_fill.txt", &finished);
        std::fs::write("out/serialize_canvas_fill.pdf", &finished);
    }
}
