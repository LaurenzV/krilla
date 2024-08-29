use crate::serialize::SerializeSettings;
use pdf_writer::{Chunk, Finish, Name, Pdf, Ref};
use std::collections::HashMap;

pub struct ChunkContainer {
    pub(crate) page_label_tree: Option<(Ref, Chunk)>,
    pub(crate) page_tree: Option<(Ref, Chunk)>,
    pub(crate) outline: Option<(Ref, Chunk)>,

    pub(crate) pages: Vec<Chunk>,
    pub(crate) page_labels: Vec<Chunk>,
    pub(crate) annotations: Vec<Chunk>,
    pub(crate) fonts: Vec<Chunk>,
    pub(crate) color_spaces: Vec<Chunk>,
    pub(crate) destinations: Vec<Chunk>,
    pub(crate) ext_g_states: Vec<Chunk>,
    pub(crate) images: Vec<Chunk>,
    pub(crate) masks: Vec<Chunk>,
    pub(crate) x_objects: Vec<Chunk>,
    pub(crate) shading_functions: Vec<Chunk>,
    pub(crate) patterns: Vec<Chunk>,
}

impl ChunkContainer {
    pub fn new() -> Self {
        Self {
            page_tree: None,
            outline: None,
            page_label_tree: None,

            pages: vec![],
            page_labels: vec![],
            annotations: vec![],
            fonts: vec![],
            color_spaces: vec![],
            destinations: vec![],
            ext_g_states: vec![],
            images: vec![],
            masks: vec![],
            x_objects: vec![],
            shading_functions: vec![],
            patterns: vec![],
        }
    }

    pub fn finish(mut self, serialize_settings: SerializeSettings) -> Pdf {
        let mut remapped_ref = Ref::new(1);
        let mut remapper = HashMap::new();

        // Two utility macros, that basically traverses the fields in the order that we
        // will write them and assigns new references as we go. This gives us the advantage
        // that the PDF will be numbered with monotonically increasing numbers, which,
        // while it is not a strict requirement for a valid PDF, makes it a lot
        // cleaner and might make implementing features like object streams easier
        // down the road.
        macro_rules! remap_field {
            ($self:expr, $remapper:expr, $remapped_ref:expr; $($field:ident),+) => {
                $(
                    if let Some((original_ref, chunk)) = &mut $self.$field {
                        for object_ref in chunk.object_refs() {
                            debug_assert!(!remapper.contains_key(&object_ref));

                            $remapper.insert(object_ref, $remapped_ref.bump());
                            *original_ref = *remapper.get(&object_ref).unwrap();
                        }
                    }
                )+
            };
        }

        macro_rules! remap_fields {
            ($self:expr, $remapper:expr, $remapped_ref:expr; $($field:ident),+) => {
                $(
                    for chunk in &$self.$field {
                        for ref_ in chunk.object_refs() {
                            debug_assert!(!remapper.contains_key(&ref_));

                            $remapper.insert(ref_, $remapped_ref.bump());
                        }
                    }
                )+
            };
        }

        // Chunk length is not an exact number because the length might change as we renumber,
        // so we add a bit of a buffer, which should hopefully always be enough
        // let mut pdf = Pdf::with_capacity((self.chunks_len as f32 * 1.1) as usize);
        let mut pdf = Pdf::new();

        if serialize_settings.ascii_compatible {
            pdf.set_binary_marker(&[b'A', b'A', b'A', b'A'])
        }

        // We only write a catalog if a page tree exists. Every valid PDF must have one
        // and krilla ensures that there always is one, but for snapshot tests, it can be
        // useful to not write a document catalog if we don't actually need it for the test.
        if self.page_tree.is_some() || self.outline.is_some() || self.page_label_tree.is_some() {
            let catalog_ref = remapped_ref.bump();

            let mut catalog = pdf.catalog(catalog_ref);
            remap_field!(self, remapper, remapped_ref; page_tree, outline, page_label_tree);

            if let Some(pt) = &self.page_tree {
                catalog.pages(pt.0);
            }

            if let Some(pl) = &self.page_label_tree {
                catalog.pair(Name(b"PageLabels"), pl.0);
            }

            if let Some(ol) = &self.outline {
                catalog.outlines(ol.0);
            }

            catalog.finish();
        }

        remap_fields!(self, remapper, remapped_ref; pages, page_labels, annotations, fonts, color_spaces, destinations, ext_g_states, images, masks, x_objects, shading_functions, patterns);

        macro_rules! write_field {
            ($self:expr, $remapper:expr, $pdf:expr; $($field:ident),+) => {
                $(
                    if let Some((_, chunk)) = $self.$field {
                        chunk.renumber_into($pdf, |old| *$remapper.get(&old).unwrap());
                    }
                )+
            };
        }

        macro_rules! write_fields {
            ($self:expr, $remapper:expr, $pdf:expr; $($field:ident),+) => {
                $(
                    for chunk in $self.$field {
                        chunk.renumber_into($pdf, |old| *$remapper.get(&old).unwrap());
                    }
                )+
            };
        }

        write_field!(self, remapper, &mut pdf; page_tree, outline, page_label_tree);
        write_fields!(self, remapper, &mut pdf; pages, page_labels, annotations, fonts, color_spaces, destinations, ext_g_states, images, masks, x_objects, shading_functions, patterns);

        pdf
    }
}
