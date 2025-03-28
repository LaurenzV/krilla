use std::ops::DerefMut;
use std::sync::Arc;

use pdf_writer::{Chunk, Finish, Name, Ref};

use crate::chunk_container::ChunkContainerFn;
use crate::configure::ValidationError;
use crate::geom::Rect;
use crate::graphics::color::{rgb, DEVICE_RGB};
use crate::resource;
use crate::resource::{Resource, Resourceable};
use crate::serialize::{Cacheable, MaybeDeviceColorSpace, SerializeContext};
use crate::stream::{FilterStreamBuilder, Stream};
use crate::util::{NameExt, Prehashed};

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    stream: Stream,
    isolated: bool,
    transparency_group_color_space: bool,
    custom_bbox: Option<Rect>,
}

#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub(crate) struct XObject(Arc<Prehashed<Repr>>);

impl XObject {
    pub(crate) fn new(
        stream: Stream,
        isolated: bool,
        transparency_group_color_space: bool,
        custom_bbox: Option<Rect>,
    ) -> Self {
        Self(Arc::new(Prehashed::new(Repr {
            stream,
            isolated,
            transparency_group_color_space,
            custom_bbox,
        })))
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.stream.is_empty()
    }

    pub(crate) fn bbox(&self) -> Rect {
        self.0.custom_bbox.unwrap_or(self.0.stream.bbox)
    }
}

impl Cacheable for XObject {
    fn chunk_container(&self) -> ChunkContainerFn {
        |cc| &mut cc.x_objects
    }

    fn serialize(self, sc: &mut SerializeContext, root_ref: Ref) -> Chunk {
        let mut chunk = Chunk::new();

        for validation_error in &self.0.stream.validation_errors {
            sc.register_validation_error(validation_error.clone());
        }

        let x_object_stream = FilterStreamBuilder::new_from_content_stream(
            &self.0.stream.content,
            &sc.serialize_settings(),
        )
        .finish(&sc.serialize_settings());
        let mut x_object = chunk.form_xobject(root_ref, x_object_stream.encoded_data());
        x_object_stream.write_filters(x_object.deref_mut().deref_mut());

        self.0
            .stream
            .resource_dictionary
            .to_pdf_resources(&mut x_object, sc.serialize_settings().pdf_version());
        x_object.bbox(
            self.0
                .custom_bbox
                .unwrap_or(self.0.stream.bbox)
                .to_pdf_rect(),
        );

        if self.0.isolated || self.0.transparency_group_color_space {
            sc.register_validation_error(ValidationError::Transparency(sc.location));

            let mut group = x_object.group();
            let transparency = group.transparency();

            if self.0.isolated {
                transparency.isolated(self.0.isolated);
            }

            if self.0.transparency_group_color_space {
                let cs = rgb::color_space(sc.serialize_settings().no_device_cs);
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
