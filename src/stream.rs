//! Drawing to a non-page context.
//!
//! In 90% of the cases, you will not have to use streams. In most cases,
//! all you need to do when using this crate is to first construct a document,
//! and then add new pages to the document and use the [`Page::surface`] method to get
//! access to the drawing context. However, there are cases when you don't want to
//! draw on the main page surface, but instead you want to create a "sub-surface"
//! where you can draw independently of the main page contents. This is what streams
//! are there for. Currently, there are only two situations where you need to do that:
//!
//! - When using masks and defining the contents of the mask.
//! - When using a pattern fill or stroke and defining the contents of the pattern.
//!
//! If you want to do any of the two above, you need to call the [`Surface::stream_builder`] method
//! of the current surface. The stream builder represents a kind of sub-context that is
//! independent of the main surface you are working with. Once you have a stream builder, you
//! can once again invoke the [`StreamBuilder::surface`] method, and use this new surface to define the contents
//! of your mask/pattern. In the end, you can call [`StreamBuilder::finish`] which will return a [`Stream`] object.
//! This [`Stream`] object contains the encoded instructions of the mask/pattern, which you can
//! then use to create new [`Pattern`]/[`Mask`] objects.
//!
//! [`Page::surface`]: crate::page::Page::surface
//! [`Surface::stream_builder`]: crate::surface::Surface::stream_builder
//! [`Pattern`]: crate::paint::Pattern
//! [`Mask`]: crate::mask::Mask

use crate::content::ContentBuilder;
use crate::resource::{ResourceDictionary, ResourceDictionaryBuilder};
use crate::serialize::SerializerContext;
use crate::surface::Surface;
use crate::util::RectWrapper;
use tiny_skia_path::Rect;

/// A stream.
///
/// See the module description for an explanation of its purpose.
// The only reason we implement clone for this type is that in some cases,
// we might need to clone a pattern (including its stream)
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Stream {
    pub(crate) content: Vec<u8>,
    pub(crate) bbox: RectWrapper,
    pub(crate) resource_dictionary: ResourceDictionary,
}

impl Stream {
    pub(crate) fn new(
        content: Vec<u8>,
        bbox: Rect,
        resource_dictionary: ResourceDictionary,
    ) -> Self {
        Self {
            content,
            bbox: RectWrapper(bbox),
            resource_dictionary,

        }
    }

    pub(crate) fn empty() -> Self {
        Self {
            content: vec![],
            bbox: RectWrapper(Rect::from_xywh(0.0, 0.0, 0.0, 0.0).unwrap()),
            resource_dictionary: ResourceDictionaryBuilder::new().finish(),
        }
    }
}

/// A builder to create streams.
pub struct StreamBuilder<'a> {
    sc: &'a mut SerializerContext,
    stream: Stream,
}

impl<'a> StreamBuilder<'a> {
    pub(crate) fn new(sc: &'a mut SerializerContext) -> Self {
        Self {
            sc,
            stream: Stream::empty(),
        }
    }

    /// Get the surface of the stream builder.
    pub fn surface(&mut self) -> Surface {
        let finish_fn = Box::new(|stream| {
            self.stream = stream;
        });

        Surface::new(self.sc, ContentBuilder::new(), finish_fn)
    }

    /// Turn the stream builder into a stream.
    pub fn finish(self) -> Stream {
        self.stream
    }
}
