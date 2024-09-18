use crate::chunk_container::ChunkContainer;
use crate::color::rgb;
use crate::error::KrillaResult;
use crate::object::Object;
use crate::resource::RegisterableResource;
use crate::serialize::{FilterStream, SerializerContext};
use crate::stream::Stream;
use crate::util::{RectExt, RectWrapper};
use pdf_writer::{Chunk, Finish, Name, Ref};
use std::ops::DerefMut;
use tiny_skia_path::Rect;

#[derive(Debug, Hash, Eq, PartialEq)]
pub(crate) struct XObject {
    stream: Stream,
    isolated: bool,
    transparency_group_color_space: bool,
    custom_bbox: Option<RectWrapper>,
}

impl XObject {
    pub fn new(
        stream: Stream,
        isolated: bool,
        transparency_group_color_space: bool,
        custom_bbox: Option<Rect>,
    ) -> Self {
        XObject {
            stream,
            isolated,
            transparency_group_color_space,
            custom_bbox: custom_bbox.map(RectWrapper),
        }
    }

    pub fn bbox(&self) -> Rect {
        self.custom_bbox.map(|c| c.0).unwrap_or(self.stream.bbox())
    }
}

impl RegisterableResource<crate::resource::XObject> for XObject {}

impl Object for XObject {
    fn chunk_container<'a>(&self, cc: &'a mut ChunkContainer) -> &'a mut Vec<Chunk> {
        &mut cc.x_objects
    }

    fn serialize(&self, sc: &mut SerializerContext, root_ref: Ref) -> KrillaResult<Chunk> {
        let mut chunk = Chunk::new();

        let x_object_stream =
            FilterStream::new_from_content_stream(self.stream.content(), &sc.serialize_settings);
        let mut x_object = chunk.form_xobject(root_ref, x_object_stream.encoded_data());
        x_object_stream.write_filters(x_object.deref_mut().deref_mut());

        self.stream
            .resource_dictionary()
            .to_pdf_resources(&mut x_object)?;
        x_object.bbox(
            self.custom_bbox
                .map(|c| c.0)
                .unwrap_or(self.stream.bbox())
                .to_pdf_rect(),
        );

        if self.isolated || self.transparency_group_color_space {
            let mut group = x_object.group();
            let transparency = group.transparency();

            if self.isolated {
                transparency.isolated(self.isolated);
            }

            if self.transparency_group_color_space {
                let cs = rgb::Color::rgb_based_color_space(sc.serialize_settings.no_device_cs);
                transparency.pair(Name(b"CS"), sc.add_cs(cs));
            }

            transparency.finish();
            group.finish();
        }

        x_object.finish();

        Ok(chunk)
    }
}

#[cfg(test)]
mod tests {

    use crate::object::xobject::XObject;
    use crate::path::Fill;
    use crate::serialize::SerializerContext;
    use crate::stream::StreamBuilder;
    use crate::tests::rect_to_path;
    use krilla_macros::snapshot;

    #[snapshot]
    fn x_object_with_transparency(sc: &mut SerializerContext) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let mut sb = StreamBuilder::new(sc);
        let mut surface = sb.surface();
        surface.fill_path(&path, Fill::default());
        surface.finish();
        let stream = sb.finish();
        let x_object = XObject::new(stream, true, true, None);
        sc.add_object(x_object).unwrap();
    }
}
