//! Fonts and images that outlive one document.
//!
//! A [`Document`](crate::Document) writes every font it uses, subset to the
//! glyphs it drew, and every image it drew. An application that writes many
//! documents into one [`ObjectSink`], for example one drawing per page of a
//! file it edits, would embed one font program per document.
//!
//! [`Resources`] keeps fonts and images across those documents:
//!
//! - **Registration.** A font gets the number of its PDF font object from the
//!   sink when a document that uses it is written, and keeps it; an image
//!   gets the number of its XObject when the first document that draws it is
//!   written, and later documents refer to that object.
//! - **Accumulation.** The glyphs of a font accumulate across documents.
//! - **Finalization.** [`Resources::finalize_into`] writes each font, subset
//!   to every glyph drawn so far, under the number that the documents refer
//!   to. It may run more than once; each run writes the fonts again.
//! - **Glyph IDs.** A glyph keeps the code it got in the document that first
//!   drew it. The subset places each glyph at its code, so content that
//!   earlier documents wrote stays valid after any finalization.

use std::collections::HashMap;

use indexmap::IndexMap;
use pdf_writer::Ref;

use crate::chunk_container::ChunkContainer;
use crate::error::KrillaResult;
use crate::serialize::{SerializeContext, SerializeSettings};
use crate::sink::ObjectSink;
use crate::text::{Font, FontContainer, FontIdentifier};

/// Fonts and images shared by documents that write into one sink.
///
/// A document draws with them from
/// [`Document::new_with_resources`](crate::Document::new_with_resources)
/// until [`Document::finish_with_resources`](crate::Document::finish_with_resources)
/// returns them.
pub struct Resources {
    pub(crate) settings: SerializeSettings,
    /// The fonts drawn so far, with their glyphs.
    pub(crate) fonts: IndexMap<Font, FontContainer>,
    /// The numbers of the PDF fonts that written documents refer to.
    pub(crate) font_numbers: HashMap<FontIdentifier, i32>,
    /// The numbers of the images that written documents hold, by hash.
    pub(crate) images: HashMap<u128, i32>,
}

impl Resources {
    /// Creates empty resources for documents written with `settings`.
    pub fn new(settings: SerializeSettings) -> Self {
        Self {
            settings,
            fonts: IndexMap::new(),
            font_numbers: HashMap::new(),
            images: HashMap::new(),
        }
    }

    /// Writes every font that a written document uses into `sink`, subset
    /// to the glyphs that all documents drew so far, under the number that
    /// the documents refer to. The objects that a font depends on, such as
    /// its program, get new numbers from `sink` on every call.
    pub fn finalize_into(&self, sink: &mut dyn ObjectSink) -> KrillaResult<()> {
        let mut sc = SerializeContext::new(self.settings.clone());
        let mut chunk_container = ChunkContainer::new(&sc);
        let mut pinned = HashMap::new();

        for container in self.fonts.values() {
            for t3_font in container.type3_mapper().fonts() {
                if let Some(&number) = self.font_numbers.get(&t3_font.identifier()) {
                    let root_ref = sc.new_ref();
                    t3_font.serialize(&mut sc, &mut chunk_container, root_ref);
                    pinned.insert(root_ref, Ref::new(number));
                }
            }

            let cid_font = container.cid_font();
            if let Some(&number) = self.font_numbers.get(&cid_font.identifier()) {
                let root_ref = sc.new_ref();
                cid_font.serialize(&mut sc, &mut chunk_container, root_ref)?;
                pinned.insert(root_ref, Ref::new(number));
            }
        }

        chunk_container.write_into(&mut sc, sink, &pinned)?;
        sc.check_errors()
    }

    /// Forgets every number for which `keep` returns `false`, for example
    /// the numbers that a sink rolled back. A document that uses the font or
    /// the image again gets a new number for it.
    pub fn retain_numbers(&mut self, mut keep: impl FnMut(i32) -> bool) {
        self.font_numbers.retain(|_, number| keep(*number));
        self.images.retain(|_, number| keep(*number));
    }
}

#[cfg(test)]
mod tests {
    use super::Resources;
    use crate::geom::Point;
    use crate::page::PageSettings;
    use crate::serialize::SerializeSettings;
    use crate::sink::ObjectSink;
    use crate::text::{Font, TextDirection};
    use crate::Document;

    #[derive(Default)]
    struct Collector {
        next: i32,
        bytes: Vec<u8>,
    }

    impl ObjectSink for Collector {
        fn allocate(&mut self) -> i32 {
            self.next += 1;
            self.next
        }

        fn write(&mut self, objects: &[u8]) {
            self.bytes.extend_from_slice(objects);
        }
    }

    impl Collector {
        /// Returns the bytes written since the last call.
        fn take(&mut self) -> Vec<u8> {
            std::mem::take(&mut self.bytes)
        }
    }

    fn settings() -> SerializeSettings {
        SerializeSettings {
            compress_content_streams: false,
            ..SerializeSettings::default()
        }
    }

    fn font() -> Font {
        font_named("NotoSans-Regular.ttf")
    }

    fn font_named(name: &str) -> Font {
        let path = format!("{}/../../assets/fonts/{name}", env!("CARGO_MANIFEST_DIR"));
        Font::new(std::fs::read(path).unwrap().into(), 0).unwrap()
    }

    /// Draws `text` on one page of a document with `resources` into `sink`,
    /// and returns the resources and the bytes the document wrote.
    fn draw(
        resources: Resources,
        sink: &mut Collector,
        font: &Font,
        text: &str,
    ) -> (Resources, Vec<u8>) {
        let mut document = Document::new_with_resources(resources);
        write_text(&mut document, font, text);
        let (result, resources) = document.finish_with_resources(sink);
        result.unwrap();
        (resources, sink.take())
    }

    /// Draws `text` on a new page of `document`.
    fn write_text(document: &mut Document, font: &Font, text: &str) {
        let mut page = document.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());
        let mut surface = page.surface();
        surface.draw_text(
            Point::from_xy(10.0, 100.0),
            font.clone(),
            12.0,
            text,
            false,
            TextDirection::Auto,
        );
        surface.finish();
        page.finish();
    }

    fn count(bytes: &[u8], needle: &[u8]) -> usize {
        bytes.windows(needle.len()).filter(|w| *w == needle).count()
    }

    /// Returns the number that the `/f0` font resource names.
    fn font_number(bytes: &[u8]) -> i32 {
        let text = String::from_utf8_lossy(bytes);
        let start = text.find("/f0 ").expect("a font resource") + 4;
        text[start..].split(' ').next().unwrap().parse().unwrap()
    }

    /// Returns whether `bytes` define the object `number`.
    fn defines(bytes: &[u8], number: i32) -> bool {
        let header = format!("{number} 0 obj");
        bytes
            .split(|b| *b == b'\n')
            .any(|line| line == header.as_bytes())
    }

    #[test]
    fn documents_refer_to_one_font_that_finalization_writes_once() {
        let font = font();
        let mut sink = Collector::default();
        let mut resources = Resources::new(settings());
        let mut numbers = vec![];
        for text in ["Hello", "world", "again"] {
            let (next, bytes) = draw(resources, &mut sink, &font, text);
            resources = next;
            assert_eq!(count(&bytes, b"/FontFile2"), 0);
            numbers.push(font_number(&bytes));
            assert!(!defines(&bytes, numbers[0]));
        }
        assert!(numbers.iter().all(|number| *number == numbers[0]));

        resources.finalize_into(&mut sink).unwrap();
        let bytes = sink.take();
        assert_eq!(count(&bytes, b"/FontFile2"), 1);
        assert!(defines(&bytes, numbers[0]));

        // A second finalization writes the font again under its number.
        resources.finalize_into(&mut sink).unwrap();
        let again = sink.take();
        assert_eq!(count(&again, b"/FontFile2"), 1);
        assert!(defines(&again, numbers[0]));
    }

    #[test]
    fn glyphs_keep_their_codes_across_documents_and_finalization() {
        let font = font();
        let mut sink = Collector::default();
        let (resources, first) = draw(Resources::new(settings()), &mut sink, &font, "AB");
        let (resources, second) = draw(resources, &mut sink, &font, "BC");

        // A is 1 and B is 2 in the first document; the second keeps B and
        // gives C the next code.
        assert_eq!(count(&first, br"[(\000\001\000\002)]TJ"), 1);
        assert_eq!(count(&second, br"[(\000\002\000\003)]TJ"), 1);

        resources.finalize_into(&mut sink).unwrap();
        let text = String::from_utf8_lossy(&sink.bytes);
        for entry in ["<0001> <0041>", "<0002> <0042>", "<0003> <0043>"] {
            assert!(text.contains(entry), "{entry}");
        }
    }

    #[test]
    fn forgotten_numbers_are_allocated_again() {
        let font = font();
        let mut sink = Collector::default();
        let (mut resources, first) = draw(Resources::new(settings()), &mut sink, &font, "A");
        let first = font_number(&first);
        resources.retain_numbers(|number| number != first);
        let (resources, second) = draw(resources, &mut sink, &font, "A");
        let second = font_number(&second);
        assert_ne!(second, first);

        resources.finalize_into(&mut sink).unwrap();
        assert!(defines(&sink.bytes, second));
        assert!(!defines(&sink.bytes, first));
    }

    #[cfg(feature = "raster-images")]
    fn image() -> crate::graphics::image::Image {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/images/rgb8.png"
        ))
        .unwrap();
        crate::graphics::image::Image::from_png(data.into(), true).unwrap()
    }

    /// Draws `image` on a new page of `document`.
    #[cfg(feature = "raster-images")]
    fn draw_image(document: &mut Document, image: &crate::graphics::image::Image) {
        let mut page = document.start_page_with(PageSettings::from_wh(200.0, 200.0).unwrap());
        let mut surface = page.surface();
        surface.draw_image(
            image.clone(),
            crate::geom::Size::from_wh(50.0, 50.0).unwrap(),
        );
        surface.finish();
        page.finish();
    }

    #[cfg(feature = "raster-images")]
    #[test]
    fn an_image_is_written_by_the_first_document_that_draws_it() {
        let image = image();
        let mut sink = Collector::default();
        let mut resources = Resources::new(settings());
        let mut written = vec![];
        for _ in 0..3 {
            let mut document = Document::new_with_resources(resources);
            draw_image(&mut document, &image);
            let (result, next) = document.finish_with_resources(&mut sink);
            result.unwrap();
            resources = next;
            written.push(count(&sink.take(), b"/Subtype/Image"));
        }
        assert_eq!(written, [1, 0, 0]);
    }

    #[cfg(feature = "raster-images")]
    #[test]
    fn a_document_that_finishes_alone_writes_what_it_uses() {
        let image = image();
        let mut sink = Collector::default();
        let mut document = Document::new_with_resources(Resources::new(settings()));
        write_text(&mut document, &font(), "AB");
        write_text(&mut document, &font_named("DejaVuSansMono.ttf"), "AB");
        draw_image(&mut document, &image);
        let (result, resources) = document.finish_with_resources(&mut sink);
        result.unwrap();

        // The second document writes the image and its one font itself, and
        // not the font that only the first document used.
        let mut document = Document::new_with_resources(resources);
        write_text(&mut document, &font(), "C");
        draw_image(&mut document, &image);
        let pdf = document.finish().unwrap();
        assert_eq!(count(&pdf, b"/FontFile2"), 1);
        assert_eq!(count(&pdf, b"/Subtype/Image"), 1);
    }

    #[test]
    fn a_document_without_resources_starts_new_ones() {
        let font = font();
        let mut sink = Collector::default();
        let mut document = Document::new_with(settings());
        write_text(&mut document, &font, "A");
        let (result, resources) = document.finish_with_resources(&mut sink);
        let first = font_number(&sink.take());
        result.unwrap();

        let (resources, second) = draw(resources, &mut sink, &font, "B");
        assert_eq!(font_number(&second), first);
        resources.finalize_into(&mut sink).unwrap();
        assert!(defines(&sink.bytes, first));
    }

    #[test]
    fn resources_move_between_threads() {
        fn send<T: Send>() {}
        send::<Resources>();
    }
}
