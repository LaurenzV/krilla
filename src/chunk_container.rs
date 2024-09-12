use crate::metadata::Metadata;
use crate::serialize::SerializeSettings;
use crate::util::hash_base64;
use pdf_writer::{Chunk, Finish, Name, Pdf, Ref};
use std::collections::HashMap;
use xmp_writer::{RenditionClass, XmpWriter};

/// Collects all of the chunks that we create while building
/// the PDF and then writes them out in an orderly manner.
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

    pub(crate) metadata: Option<Metadata>,
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

            metadata: None,
        }
    }

    fn get_other_metadata(&self, root_ref: Ref, pdf: &mut Pdf) -> Chunk {
        const PDF_VERSION: &str = "PDF-1.7";

        let mut xmp = XmpWriter::new();
        if let Some(metadata) = &self.metadata {
            metadata.serialize_xmp_metadata(&mut xmp);
        }

        let instance_id = hash_base64(pdf.as_bytes());

        let document_id = if let Some(metadata) = &self.metadata {
            if let Some(document_id) = &metadata.document_id {
                hash_base64(&(PDF_VERSION, document_id))
            } else if metadata.title.is_some() && metadata.authors.is_some() {
                hash_base64(&(PDF_VERSION, &metadata.title, &metadata.authors))
            } else {
                instance_id.clone()
            }
        } else {
            instance_id.clone()
        };

        xmp.num_pages(self.pages.len() as u32);
        xmp.instance_id(&instance_id);
        xmp.document_id(&document_id);
        pdf.set_file_id((
            document_id.as_bytes().to_vec(),
            instance_id.as_bytes().to_vec(),
        ));

        xmp.rendition_class(RenditionClass::Proof);
        xmp.pdf_version("1.7");

        Chunk::new()
    }

    pub fn finish(mut self, serialize_settings: SerializeSettings) -> Pdf {
        let mut remapped_ref = Ref::new(1);
        let mut remapper = HashMap::new();

        // Two utility macros, that basically traverses the fields in the order that we
        // will write them to the PDF and assigns new references as we go.
        // This gives us the advantage that the PDF will be numbered with
        // monotonically increasing numbers, which, while it is not a strict requirement
        // for a valid PDF, makes it a lot cleaner and might make implementing features
        // like object streams easier down the road.
        //
        // It also allows us to estimate the capacity we will need for the new PDF.
        let mut chunks_len = 0;
        macro_rules! remap_field {
            ($self:expr, $remapper:expr, $remapped_ref:expr; $($field:ident),+) => {
                $(
                    if let Some((original_ref, chunk)) = &mut $self.$field {
                        chunks_len += chunk.len();
                        for object_ref in chunk.refs() {
                            debug_assert!(!remapper.contains_key(&object_ref));

                            $remapper.insert(object_ref, $remapped_ref.bump());
                        }

                        *original_ref = *remapper.get(&original_ref).unwrap();
                    }
                )+
            };
        }

        macro_rules! remap_fields {
            ($self:expr, $remapper:expr, $remapped_ref:expr; $($field:ident),+) => {
                $(
                    for chunk in &$self.$field {
                        chunks_len += chunk.len();
                        for ref_ in chunk.refs() {
                            debug_assert!(!remapper.contains_key(&ref_));

                            $remapper.insert(ref_, $remapped_ref.bump());
                        }
                    }
                )+
            };
        }

        // Chunk length is not an exact number because the length might change as we renumber,
        // so we add a bit of a padding by multiplying with 1.1. The 200 is additional padding
        // for the document catalog. This hopefully allows us to avoid re-alloactions in the general
        // case, and thus give us better performance.
        let mut pdf = Pdf::with_capacity((chunks_len as f32 * 1.1 + 200.0) as usize);

        if serialize_settings.ascii_compatible {
            pdf.set_binary_marker(&[b'A', b'A', b'A', b'A'])
        }

        remap_field!(self, remapper, remapped_ref; page_tree, outline, page_label_tree);
        remap_fields!(self, remapper, remapped_ref; pages, page_labels,
            annotations, fonts, color_spaces, destinations,
            ext_g_states, images, masks, x_objects, shading_functions,
            patterns
        );

        macro_rules! write_field {
            ($self:expr, $remapper:expr, $pdf:expr; $($field:ident),+) => {
                $(
                    if let Some((_, chunk)) = &$self.$field {
                        chunk.renumber_into($pdf, |old| *$remapper.get(&old).unwrap());
                    }
                )+
            };
        }

        macro_rules! write_fields {
            ($self:expr, $remapper:expr, $pdf:expr; $($field:ident),+) => {
                $(
                    for chunk in &$self.$field {
                        chunk.renumber_into($pdf, |old| *$remapper.get(&old).unwrap());
                    }
                )+
            };
        }

        write_field!(self, remapper, &mut pdf; page_tree, outline, page_label_tree);
        write_fields!(self, remapper, &mut pdf; pages, page_labels,
            annotations, fonts, color_spaces, destinations,
            ext_g_states, images, masks, x_objects,
            shading_functions, patterns
        );

        if let Some(metadata) = self.metadata {
            metadata.serialize_document_info(&mut remapped_ref, &mut pdf);
        }

        // We only write a catalog if a page tree exists. Every valid PDF must have one
        // and krilla ensures that there always is one, but for snapshot tests, it can be
        // useful to not write a document catalog if we don't actually need it for the test.
        if self.page_tree.is_some() || self.outline.is_some() || self.page_label_tree.is_some() {
            let catalog_ref = remapped_ref.bump();

            let mut catalog = pdf.catalog(catalog_ref);

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

        pdf
    }
}
