use std::ops::DerefMut;
use std::sync::Arc;

use pdf_writer::{Finish, Name, Ref};

use crate::chunk_container::ChunkContainer;
use crate::configure::ValidationError;
use crate::geom::Rect;
use crate::graphics::color::{luma, rgb};
use crate::resource;
use crate::resource::Resourceable;
use crate::serialize::{Cacheable, SerializeContext};
use crate::stream::{FilterStreamBuilder, Stream};
use crate::util::Prehashed;

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub(crate) enum GroupColorSpace {
    Rgb,
    Luma,
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct Repr {
    stream: Stream,
    isolated: bool,
    transparency_group_color_space: Option<GroupColorSpace>,
    custom_bbox: Option<Rect>,
}

#[derive(Debug, Hash, Clone, Eq, PartialEq)]
pub(crate) struct XObject(Arc<Prehashed<Repr>>);

impl XObject {
    pub(crate) fn new(
        stream: Stream,
        isolated: bool,
        mut transparency_group_color_space: Option<GroupColorSpace>,
        custom_bbox: Option<Rect>,
    ) -> Self {
        // In case a mask was invoked in the content stream, we _always_ create
        // a new transparency group. Please see <https://github.com/typst/typst/issues/5509>.
        // Just to provide a brief explanation: I have not found any mention
        // of a transparency group being required when using a soft mask in the
        // PDF spec. However, Apple Preview (and only them!) have a weird bug
        // where if you transform an XObject with a soft mask but not transparency
        // group, the transform gets applied twice to the mask, resulting in
        // rendering issues. Using a transparency group in this case seems to
        // fix the issue.
        // TODO: Apply group transparency to page as well.
        if stream.uses_mask {
            transparency_group_color_space.get_or_insert(GroupColorSpace::Rgb);
        }

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
    fn serialize(
        self,
        sc: &mut SerializeContext,
        chunk_container: &mut ChunkContainer,
        root_ref: Ref,
    ) {
        let mut chunk = sc.new_chunk();

        for validation_error in &self.0.stream.validation_errors {
            sc.register_validation_error(validation_error.clone());
        }

        let use_transparency_group =
            self.0.isolated || self.0.transparency_group_color_space.is_some();

        if use_transparency_group {
            sc.register_validation_error(ValidationError::Transparency(sc.location));
        }

        let struct_parents = self
            .0
            .stream
            .xobject_identifier
            .map(|xi| sc.register_xobject_struct_parent(xi, root_ref));

        let serialize_settings = sc.serialize_settings();

        // "Ordinarily, the CS entry may be present only for isolated transparency groups (those
        // for which I is true), and even then it is optional. However, this entry shall be present in
        // the group attributes dictionary for any transparency group XObject that has no parent
        // group or page from which to inherit."
        //
        // So while we don't technically always need to write it, let's just play it save
        // and always add a group color space in case we have a transparency group.
        let transparency_group_cs = if use_transparency_group {
            let no_device_cs = serialize_settings.no_device_cs;
            let group_cs = self
                .0
                .transparency_group_color_space
                .unwrap_or(GroupColorSpace::Rgb);
            let cs = match group_cs {
                GroupColorSpace::Rgb => rgb::color_space(no_device_cs),
                GroupColorSpace::Luma => luma::color_space(no_device_cs),
            };

            Some(sc.register_colorspace(chunk_container, cs.into()))
        } else {
            None
        };

        let x_object_stream = FilterStreamBuilder::new_from_content_stream(
            &self.0.stream.content,
            &serialize_settings,
        )
        .finish(&serialize_settings);
        let mut x_object = chunk.form_xobject(root_ref, x_object_stream.encoded_data());
        x_object_stream.write_filters(x_object.deref_mut().deref_mut());

        self.0.stream.resource_dictionary.to_pdf_resources(
            &mut x_object,
            sc,
            &mut chunk_container.non_stream.resource_dictionaries,
        );
        x_object.bbox(
            self.0
                .custom_bbox
                .unwrap_or(self.0.stream.bbox)
                .to_pdf_rect(),
        );

        if let Some(struct_parents) = struct_parents {
            x_object.struct_parents(struct_parents);
        }

        if use_transparency_group {
            let mut group = x_object.group();
            let transparency = group.transparency();

            if self.0.isolated {
                transparency.isolated(self.0.isolated);
            }

            if let Some(transparency_group_cs) = transparency_group_cs {
                transparency_group_cs.write(transparency.insert(Name(b"CS")));
            }

            transparency.finish();
            group.finish();
        }

        x_object.finish();
        chunk_container.streams.x_objects.push(chunk);
    }
}

impl Resourceable for XObject {
    type Resource = resource::XObject;
}
