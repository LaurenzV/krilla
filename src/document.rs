use crate::error::KrillaResult;
use crate::object::outline::Outline;
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

    pub fn set_outline(&mut self, outline: Outline) {
        self.serializer_context.set_outline(outline);
    }

    pub fn finish(self) -> KrillaResult<Vec<u8>> {
        Ok(self.serializer_context.finish()?.finish())
    }
}
