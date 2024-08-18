use crate::object::page::PageLabel;
use crate::serialize::{SerializeSettings, SerializerContext};
use crate::surface::PageBuilder;
use tiny_skia_path::Size;

pub struct Document {
    serializer_context: SerializerContext,
}

impl Document {
    pub fn new(serialize_settings: SerializeSettings) -> Self {
        Self {
            serializer_context: SerializerContext::new(serialize_settings),
        }
    }

    pub fn new_with_sc(serializer_context: SerializerContext) -> Self {
        Self { serializer_context }
    }

    pub fn start_page(&mut self, size: Size) -> PageBuilder {
        PageBuilder::new(&mut self.serializer_context, size)
    }

    pub fn start_page_with(&mut self, size: Size, page_label: PageLabel) -> PageBuilder {
        PageBuilder::new_with(&mut self.serializer_context, size, page_label)
    }

    pub fn finish(self) -> Vec<u8> {
        self.serializer_context.finish().finish()
    }
}
