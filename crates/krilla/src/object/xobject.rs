//! XObjects.

use std::ops::DerefMut;

use pdf_writer::{Chunk, Finish, Name, Ref};
use tiny_skia_path::Rect;

use crate::color::{rgb, DEVICE_RGB};
use crate::configure::ValidationError;
use crate::object::{Cacheable, ChunkContainerFn, Resourceable};
use crate::resource;
use crate::resource::Resource;
use crate::serialize::{MaybeDeviceColorSpace, SerializeContext};
use crate::stream::{FilterStreamBuilder, Stream};
use crate::util::{NameExt, RectExt, RectWrapper};

#[derive(Debug, Hash, Eq, PartialEq)]
pub(crate) struct XObject {
    stream: Stream,
    isolated: bool,
    transparency_group_color_space: bool,
    custom_bbox: Option<RectWrapper>,
}

impl XObject {
    pub(crate) fn new(
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

    pub(crate) fn is_empty(&self) -> bool {
        self.stream.is_empty()
    }

    pub(crate) fn bbox(&self) -> Rect {
        self.custom_bbox.map(|c| c.0).unwrap_or(self.stream.bbox.0)
    }
}

impl Cacheable for XObject {
    fn chunk_container(&self) -> ChunkContainerFn {
        |cc| &mut cc.x_objects
    }

    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        for validation_error in self.stream.validation_errors {
            sc.register_validation_error(validation_error);
        }

        let x_object_stream = FilterStreamBuilder::new_from_content_stream(
            &self.stream.content,
            &sc.serialize_settings(),
        )
        .finish(&sc.serialize_settings());
        let mut x_object = chunk.form_xobject(root_ref, x_object_stream.encoded_data());
        x_object_stream.write_filters(x_object.deref_mut().deref_mut());

        self.stream
            .resource_dictionary
            .to_pdf_resources(&mut x_object, sc.serialize_settings().pdf_version());
        x_object.bbox(
            self.custom_bbox
                .map(|c| c.0)
                .unwrap_or(*self.stream.bbox)
                .to_pdf_rect(),
        );

        if self.isolated || self.transparency_group_color_space {
            sc.register_validation_error(ValidationError::Transparency(sc.location));

            let mut group = x_object.group();
            let transparency = group.transparency();

            if self.isolated {
                transparency.isolated(self.isolated);
            }

            if self.transparency_group_color_space {
                let cs = rgb::Color::rgb_color_space(sc.serialize_settings().no_device_cs);
                let pdf_cs = transparency.insert(Name(b"CS"));

                match sc.register_colorspace(cs) {
                    MaybeDeviceColorSpace::DeviceRgb => pdf_cs.primitive(DEVICE_RGB.to_pdf_name()),
                    // Can only be SRGB
                    MaybeDeviceColorSpace::ColorSpace(cs) => pdf_cs.primitive(cs.get_ref()),
                    _ => unreachable!(),
                }
            }

            transparency.finish();
            group.finish();
        }

        x_object.finish();

        chunk
    }
}

impl Resourceable for XObject {
    type Resource = resource::XObject;
}

#[cfg(test)]
mod tests {
    use krilla_macros::snapshot;

    use crate::object::xobject::XObject;
    use crate::path::Fill;
    use crate::serialize::SerializeContext;
    use crate::stream::StreamBuilder;
    use crate::tests::rect_to_path;

    #[snapshot]
    fn x_object_with_transparency(sc: &mut SerializeContext) {
        let path = rect_to_path(20.0, 20.0, 180.0, 180.0);
        let mut sb = StreamBuilder::new(sc);
        let mut surface = sb.surface();
        surface.fill_path(&path, Fill::default());
        surface.finish();
        let stream = sb.finish();
        let x_object = XObject::new(stream, true, true, None);
        sc.register_cacheable(x_object);
    }
}
