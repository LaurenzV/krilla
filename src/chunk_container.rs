use crate::metadata::Metadata;
use crate::serialize::SerializeSettings;
use crate::util::{hash_base64, Deferred};
use pdf_writer::{Chunk, Finish, Name, Pdf, Ref};
use std::collections::HashMap;
use xmp_writer::{RenditionClass, XmpWriter};

trait ChunkExt {
    fn wait(&self) -> &Chunk;
}

impl ChunkExt for Chunk {
    fn wait(&self) -> &Chunk {
        self
    }
}

/// Collects all chunks that we create while building
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
    pub(crate) images: Vec<Deferred<Chunk>>,
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
            ($remapper:expr, $remapped_ref:expr; $($field:expr),+) => {
                $(
                    if let Some((original_ref, chunk)) = $field {
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
            ($remapper:expr, $remapped_ref:expr; $($field:expr),+) => {
                $(
                    for chunk in $field {
                        let chunk = chunk.wait();
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

        remap_field!(remapper, remapped_ref; &mut self.page_tree, &mut self.outline, &mut self.page_label_tree);
        remap_fields!(remapper, remapped_ref; &self.pages, &self.page_labels,
            &self.annotations, &self.fonts, &self.color_spaces, &self.destinations,
            &self.ext_g_states, &self.images, &self.masks, &self.x_objects, &self.shading_functions,
            &self.patterns
        );

        macro_rules! write_field {
            ($remapper:expr, $pdf:expr; $($field:expr),+) => {
                $(
                    if let Some((_, chunk)) = $field {
                        chunk.renumber_into($pdf, |old| *$remapper.get(&old).unwrap());
                    }
                )+
            };
        }

        macro_rules! write_fields {
            ($remapper:expr, $pdf:expr; $($field:expr),+) => {
                $(
                    for chunk in $field {
                        let chunk = chunk.wait();
                        chunk.renumber_into($pdf, |old| *$remapper.get(&old).unwrap());
                    }
                )+
            };
        }

        write_field!(remapper, &mut pdf; &self.page_tree, &self.outline, &self.page_label_tree);
        write_fields!(remapper, &mut pdf; &self.pages, &self.page_labels,
            &self.annotations, &self.fonts, &self.color_spaces, &self.destinations,
            &self.ext_g_states, &self.images, &self.masks, &self.x_objects,
            &self.shading_functions, &self.patterns
        );

        // Write the PDF document info metadata.
        if let Some(metadata) = &self.metadata {
            metadata.serialize_document_info(&mut remapped_ref, &mut pdf);
        }

        // Write the XMP data, if applicable
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
        xmp.format("application/pdf");
        // TODO: Add XMP languages
        xmp.instance_id(&instance_id);
        xmp.document_id(&document_id);
        pdf.set_file_id((
            document_id.as_bytes().to_vec(),
            instance_id.as_bytes().to_vec(),
        ));

        xmp.rendition_class(RenditionClass::Proof);
        xmp.pdf_version("1.7");

        // We only write a catalog if a page tree exists. Every valid PDF must have one
        // and krilla ensures that there always is one, but for snapshot tests, it can be
        // useful to not write a document catalog if we don't actually need it for the test.
        if self.page_tree.is_some() || self.outline.is_some() || self.page_label_tree.is_some() {
            let meta_ref = if serialize_settings.xmp_metadata {
                let meta_ref = remapped_ref.bump();
                let xmp_buf = xmp.finish(None);
                pdf.stream(meta_ref, xmp_buf.as_bytes())
                    .pair(Name(b"Type"), Name(b"Metadata"))
                    .pair(Name(b"Subtype"), Name(b"XML"));
                Some(meta_ref)
            } else {
                None
            };

            let catalog_ref = remapped_ref.bump();

            let mut catalog = pdf.catalog(catalog_ref);

            if let Some(pt) = &self.page_tree {
                catalog.pages(pt.0);
            }

            if let Some(meta_ref) = meta_ref {
                catalog.metadata(meta_ref);
            }

            if let Some(pl) = &self.page_label_tree {
                catalog.pair(Name(b"PageLabels"), pl.0);
            }

            // TODO: Add viewer preferences
            // TODO: Add lang

            if let Some(ol) = &self.outline {
                catalog.outlines(ol.0);
            }

            catalog.finish();
        }

        pdf
    }
}
