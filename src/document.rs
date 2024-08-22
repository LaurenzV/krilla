use crate::object::outline::Outline;
use crate::object::page::PageLabel;
use crate::serialize::{SerializeSettings, SerializerContext};
use crate::surface::PageBuilder;
use tiny_skia_path::Size;

pub struct Document {
    serializer_context: SerializerContext,
    page_count: usize
}

impl Document {
    pub fn new(serialize_settings: SerializeSettings) -> Self {
        Self {
            serializer_context: SerializerContext::new(serialize_settings),
            page_count: 0
        }
    }

    pub fn new_with_sc(serializer_context: SerializerContext) -> Self {
        Self { serializer_context, page_count: 0 }
    }

    pub fn start_page(&mut self, size: Size) -> PageBuilder {
        let page_index = self.page_count;
        self.page_count += 1;
        PageBuilder::new(&mut self.serializer_context, size, page_index)
    }

    pub fn start_page_with(&mut self, size: Size, page_label: PageLabel) -> PageBuilder {
        let page_index = self.page_count;
        self.page_count += 1;
        PageBuilder::new_with(&mut self.serializer_context, size, page_label, page_index)
    }

    pub fn set_outline(&mut self, outline: Outline) {
        self.serializer_context.set_outline(outline);
    }

    pub fn finish(self) -> Vec<u8> {
        self.serializer_context.finish().finish()
    }
}
