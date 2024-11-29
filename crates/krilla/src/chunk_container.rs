use crate::error::{KrillaError, KrillaResult};
use crate::metadata::Metadata;
use crate::serialize::SerializerContext;
use crate::util::{hash_base64, Deferred};
use crate::validation::ValidationError;
use crate::version::PdfVersion;
use pdf_writer::{Chunk, Finish, Name, Pdf, Ref, Str, TextStr};
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
#[derive(Default)]
pub struct ChunkContainer {
    pub(crate) page_label_tree: Option<(Ref, Chunk)>,
    pub(crate) page_tree: Option<(Ref, Chunk)>,
    pub(crate) outline: Option<(Ref, Chunk)>,
    pub(crate) destination_profiles: Option<(Ref, Chunk)>,
    pub(crate) struct_tree_root: Option<(Ref, Chunk)>,

    pub(crate) pages: Vec<Chunk>,
    pub(crate) struct_elements: Vec<Chunk>,
    pub(crate) page_labels: Vec<Chunk>,
    pub(crate) annotations: Vec<Chunk>,
    pub(crate) fonts: Vec<Chunk>,
    pub(crate) color_spaces: Vec<Chunk>,
    pub(crate) icc_profiles: Vec<Chunk>,
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
        Self::default()
    }

    pub fn finish(mut self, sc: &mut SerializerContext) -> KrillaResult<Pdf> {
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
        sc.serialize_settings().pdf_version.set_version(&mut pdf);

        if sc.serialize_settings().ascii_compatible
            && !sc.serialize_settings().validator.requires_binary_header()
        {
            pdf.set_binary_marker(b"AAAA")
        }

        remap_field!(remapper, remapped_ref; &mut self.page_tree, &mut self.outline,
            &mut self.page_label_tree, &mut self.destination_profiles,
        &mut self.struct_tree_root);
        remap_fields!(remapper, remapped_ref; &self.pages, &self.struct_elements, &self.page_labels,
            &self.annotations, &self.fonts, &self.color_spaces, &self.icc_profiles, &self.destinations,
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

        write_field!(remapper, &mut pdf; &self.page_tree, &self.outline,
            &self.page_label_tree, &self.destination_profiles,
        &mut self.struct_tree_root);
        write_fields!(remapper, &mut pdf; &self.pages, &self.struct_elements, &self.page_labels,
            &self.annotations, &self.fonts, &self.color_spaces, &self.icc_profiles, &self.destinations,
            &self.ext_g_states, &self.images, &self.masks, &self.x_objects,
            &self.shading_functions, &self.patterns
        );

        if self.metadata.as_ref().is_none_or(|m| m.title.is_none()) {
            sc.register_validation_error(ValidationError::NoDocumentTitle);
        }

        // Write the PDF document info metadata.
        if let Some(metadata) = &self.metadata {
            metadata.serialize_document_info(&mut remapped_ref, &mut pdf);
        }

        let mut xmp = XmpWriter::new();
        if let Some(metadata) = &self.metadata {
            metadata.serialize_xmp_metadata(&mut xmp);
        }

        sc.serialize_settings().validator.write_xmp(&mut xmp);

        let instance_id = hash_base64(pdf.as_bytes());

        let document_id = if let Some(metadata) = &self.metadata {
            if let Some(document_id) = &metadata.document_id {
                hash_base64(&(sc.serialize_settings().pdf_version.as_str(), document_id))
            } else if metadata.title.is_some() && metadata.authors.is_some() {
                hash_base64(&(
                    sc.serialize_settings().pdf_version.as_str(),
                    &metadata.title,
                    &metadata.authors,
                ))
            } else {
                instance_id.clone()
            }
        } else {
            instance_id.clone()
        };

        xmp.num_pages(self.pages.len() as u32);
        xmp.format("application/pdf");
        xmp.instance_id(&instance_id);
        xmp.document_id(&document_id);
        pdf.set_file_id((
            document_id.as_bytes().to_vec(),
            instance_id.as_bytes().to_vec(),
        ));

        xmp.rendition_class(RenditionClass::Proof);
        sc.serialize_settings().pdf_version.write_xmp(&mut xmp);

        // We only write a catalog if a page tree exists. Every valid PDF must have one
        // and krilla ensures that there always is one, but for snapshot tests, it can be
        // useful to not write a document catalog if we don't actually need it for the test.
        if self.page_tree.is_some()
            || self.outline.is_some()
            || self.page_label_tree.is_some()
            || self.destination_profiles.is_some()
            || self.struct_tree_root.is_some()
        {
            let meta_ref = if sc.serialize_settings().xmp_metadata {
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

            if let Some(oi) = &self.destination_profiles {
                catalog.pair(Name(b"OutputIntents"), oi.0);
            }

            if let Some(lang) = self.metadata.and_then(|m| m.language).as_ref() {
                catalog.lang(TextStr(lang));
            } else {
                sc.register_validation_error(ValidationError::NoDocumentLanguage);
            }

            if let Some(st) = &self.struct_tree_root {
                catalog.pair(Name(b"StructTreeRoot"), st.0);
                let mut mark_info = catalog.mark_info();
                mark_info.marked(true);
                if sc.serialize_settings().pdf_version >= PdfVersion::Pdf16 {
                    // We always set suspects to false because it's required by PDF/UA
                    mark_info.suspects(false);
                }
                mark_info.finish();
            }

            if sc
                .serialize_settings()
                .validator
                .requires_display_doc_title()
            {
                catalog.viewer_preferences().display_doc_title(true);
            }

            if let Some(ol) = &self.outline {
                catalog.outlines(ol.0);
            }

            if !sc.used_named_destinations.is_empty() {
                // Cannot use pdf-writer API here because it requires Ref's, while
                // we write our destinations directly into the array.
                let mut names = catalog.names();
                let mut name_tree = names.destinations();
                let mut name_entries = name_tree.names();

                let used_destinations = std::mem::take(&mut sc.used_named_destinations);
                let named_destinations = std::mem::take(&mut sc.named_destinations);
                for name in &used_destinations {
                    let dest =
                        named_destinations
                            .get(name)
                            .ok_or(KrillaError::UserError(format!(
                                "named destination {} was used, but not registered in document",
                                name.inner()
                            )))?;
                    name_entries.insert(Str(name.inner().as_bytes()), *remapper.get(dest).unwrap());
                }
            }

            catalog.finish();
        }

        Ok(pdf)
    }
}
