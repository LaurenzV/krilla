use std::path::PathBuf;
use sitro::{render_mupdf, render_pdfbox, render_pdfium, render_pdfjs, render_quartz, render_xpdf, RenderedDocument, Renderer, RenderOptions};
use tiny_skia_path::{PathBuilder, Rect, Size};
use krilla::document::Document;
use krilla::Fill;
use krilla::rgb::Rgb;
use krilla::serialize::SerializeSettings;

pub fn render_doc(doc: &[u8], renderer: Renderer) -> RenderedDocument {
    let options = RenderOptions {
        scale: 1.0,
    };

    match renderer {
        Renderer::Pdfium => render_pdfium(doc, &options).unwrap(),
        Renderer::Mupdf => render_mupdf(doc, &options).unwrap(),
        Renderer::Xpdf => render_xpdf(doc, &options).unwrap(),
        Renderer::QuartzRenderer => render_quartz(doc, &options).unwrap(),
        Renderer::PdfjsRenderer => render_pdfjs(doc, &options).unwrap(),
        Renderer::PdfboxRenderer => render_pdfbox(doc, &options).unwrap(),
    }
}

pub fn save_refs(name: &str, renderer: Renderer, document: RenderedDocument) {
    let refs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/refs")
        .join(name);

    std::fs::create_dir_all(&refs_path).unwrap();

    if document.is_empty() {
        panic!("empty document");
    }   else if document.len() == 1 {
        let ref_path = refs_path.join(format!("{}.png", renderer.name()));
        std::fs::write(&ref_path, &document[0]).unwrap();
    }   else {
        for (index, page) in document.iter().enumerate() {
            let ref_path = refs_path.join(format!("{}_{}.png", index + 1, renderer.name()));
            std::fs::write(&ref_path, &document[0]).unwrap();
        }
    }
}

#[test]
fn basic_page() {
    let mut doc_builder = Document::new(SerializeSettings::default());
    let mut page = doc_builder.start_page(Size::from_wh(200.0, 200.0).unwrap());
    let mut surface = page.surface();

    let mut builder = PathBuilder::new();
    builder.push_rect(Rect::from_xywh(20.0, 20.0, 160.0, 160.0).unwrap());
    let path = builder.finish().unwrap();

    surface.fill_path(&path, Fill::<Rgb>::default());
    surface.finish();
    page.finish();

    let pdf = doc_builder.finish();
    let rendered = render_doc(&pdf, Renderer::Pdfium);
    save_refs("basic_page", Renderer::Pdfium, rendered);
}