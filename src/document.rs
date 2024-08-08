use crate::serialize::{SerializeSettings, SerializerContext};
use crate::surface::CanvasBuilder;
use fontdb::Database;
use pdf_writer::Pdf;
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

    pub fn start_page(&mut self, size: Size) -> CanvasBuilder {
        CanvasBuilder::new(&mut self.serializer_context, size)
    }

    pub fn finish(self, fontdb: &Database) -> Vec<u8> {
        self.serializer_context.finish(fontdb).finish()
    }
}
