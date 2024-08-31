//! Drawing to a non-page context.
//!
//! In 90% of the cases, you will not have to use streams. In most cases,
//! all you need to do when using this crate is to first construct a document,
//! and then add new pages to the document and use the `surface` method to get
//! access to the drawing context. However, there are cases when you don't want to
//! draw on the main page surface, but instead you want to create a "sub-surface"
//! where you can draw independently of the main page contents. This is what streams
//! are there for. Currently, there are only two situations where you need to do that:
//!
//! - When using masks and defining the contents of the mask.
//! - When using a pattern fill or stroke and defining the contents of the pattern.
//!
//! If you want to do any of the two above, you need to call the `stream_builder` method
//! of the current surface. The stream builder represents a kind of sub-context that is
//! independent of the main surface you are working with. Once you have a stream builder, you
//! can once again invoke the `surface` method, and use this new surface to define the contents
//! of your mask/pattern. In the end, you can call `finish`, which will return a `Stream` object.
//! This `Stream` object contains the encoded instructions of the mask/pattern, which you can
//! then use to create new `Pattern`/`Mask` objects.
use crate::resource::{ResourceDictionary, ResourceDictionaryBuilder};
use crate::util::RectWrapper;
use skrifa::GlyphId;
use std::ops::Range;
use std::sync::Arc;
use tiny_skia_path::Rect;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct Repr {
    content: Vec<u8>,
    bbox: RectWrapper,
    resource_dictionary: ResourceDictionary,
}

/// A stream.
///
/// See the module description for an explanation of its purpose.
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Stream(Arc<Repr>);

impl Stream {
    pub(crate) fn new(
        content: Vec<u8>,
        bbox: Rect,
        resource_dictionary: ResourceDictionary,
    ) -> Self {
        Self(Arc::new(Repr {
            content,
            bbox: RectWrapper(bbox),
            resource_dictionary,
        }))
    }

    pub(crate) fn content(&self) -> &[u8] {
        &self.0.content
    }

    pub(crate) fn bbox(&self) -> Rect {
        self.0.bbox.0
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.content.is_empty()
    }

    pub(crate) fn resource_dictionary(&self) -> &ResourceDictionary {
        &self.0.resource_dictionary
    }

    pub(crate) fn empty() -> Self {
        Self(Arc::new(Repr {
            content: vec![],
            bbox: RectWrapper(Rect::from_xywh(0.0, 0.0, 0.0, 0.0).unwrap()),
            resource_dictionary: ResourceDictionaryBuilder::new().finish(),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct Glyph {
    pub glyph_id: GlyphId,
    pub range: Range<usize>,
    pub x_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub size: f32,
}

impl Glyph {
    pub fn new(
        glyph_id: GlyphId,
        x_advance: f32,
        x_offset: f32,
        y_offset: f32,
        range: Range<usize>,
        size: f32,
    ) -> Self {
        Self {
            glyph_id,
            x_advance,
            x_offset,
            y_offset,
            range,
            size,
        }
    }
}
