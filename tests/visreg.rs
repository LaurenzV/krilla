use krilla::document::Document;
use krilla::rgb::Rgb;
use krilla::serialize::SerializeSettings;
use krilla::{rgb, Fill, LinearGradient, Paint, SpreadMethod, Stop};
use sitro::{
    render_mupdf, render_pdfbox, render_pdfium, render_pdfjs, render_quartz, render_xpdf,
    RenderOptions, RenderedDocument, Renderer,
};
use std::path::PathBuf;
use tiny_skia_path::{PathBuilder, Rect, Size, Transform};
use usvg::NormalizedF32;

pub fn render_doc(doc: &[u8], renderer: &Renderer) -> RenderedDocument {
    let options = RenderOptions { scale: 1.0 };

    match renderer {
        Renderer::Pdfium => render_pdfium(doc, &options).unwrap(),
        Renderer::Mupdf => render_mupdf(doc, &options).unwrap(),
        Renderer::Xpdf => render_xpdf(doc, &options).unwrap(),
        Renderer::QuartzRenderer => render_quartz(doc, &options).unwrap(),
        Renderer::PdfjsRenderer => render_pdfjs(doc, &options).unwrap(),
        Renderer::PdfboxRenderer => render_pdfbox(doc, &options).unwrap(),
    }
}

pub fn save_refs(name: &str, renderer: &Renderer, document: RenderedDocument) {
    let refs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/refs")
        .join(name);

    std::fs::create_dir_all(&refs_path).unwrap();

    if document.is_empty() {
        panic!("empty document");
    } else if document.len() == 1 {
        let ref_path = refs_path.join(format!("{}.png", renderer.name()));
        std::fs::write(&ref_path, &document[0]).unwrap();
    } else {
        for (index, page) in document.iter().enumerate() {
            let ref_path = refs_path.join(format!("{}_{}.png", index + 1, renderer.name()));
            std::fs::write(&ref_path, page).unwrap();
        }
    }
}

macro_rules! generate_renderer_tests {
    ($test_name:ident, $test_body:expr) => {
        paste::item! {
            #[test]
            fn [<$test_name _pdfium>]() {
                let renderer = Renderer::Pdfium;
                $test_body(renderer);
            }

            #[test]
            fn [<$test_name _mupdf>]() {
                let renderer = Renderer::Mupdf;
                $test_body(renderer);
            }

            #[test]
            fn [<$test_name _xpdf>]() {
                let renderer = Renderer::Xpdf;
                $test_body(renderer);
            }

            #[cfg(target_os = "macos")]
            #[test]
            fn [<$test_name _quartz>]() {
                let renderer = Renderer::QuartzRenderer;
                $test_body(renderer);
            }

            #[test]
            fn [<$test_name _pdfjs>]() {
                let renderer = Renderer::PdfjsRenderer;
                $test_body(renderer);
            }

            #[test]
            fn [<$test_name _pdfbox>]() {
                let renderer = Renderer::PdfboxRenderer;
                $test_body(renderer);
            }
        }
    };
}

generate_renderer_tests!(linear_gradient, |renderer| {
    let mut doc_builder = Document::new(SerializeSettings::default());
    let mut page = doc_builder.start_page(Size::from_wh(200.0, 200.0).unwrap());
    let mut surface = page.surface();

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
    surface.finish();
    page.finish();

    let pdf = doc_builder.finish();
    let rendered = render_doc(&pdf, &renderer);
    save_refs(stringify!(linear_gradient), &renderer, rendered);
});
