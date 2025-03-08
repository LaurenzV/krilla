//! Collecting chunks during PDF creation.

use std::collections::HashMap;

use pdf_writer::{Chunk, Finish, Name, Pdf, Ref, Str, TextStr};
use xmp_writer::{RenditionClass, XmpWriter};

use crate::error::KrillaResult;
use crate::metadata::Metadata;
use crate::serialize::SerializeContext;
use crate::util::{hash_base64, Deferred};
use crate::validation::ValidationError;
use crate::version::PdfVersion;

/// Collects all chunks that we create while building
/// the PDF and then writes them out in an orderly manner.
#[derive(Default)]
pub(crate) struct ChunkContainer {
    pub(crate) page_tree: Option<(Ref, Chunk)>,
    pub(crate) outline: Option<(Ref, Chunk)>,
    pub(crate) page_label_tree: Option<(Ref, Chunk)>,
    pub(crate) destination_profiles: Option<(Ref, Chunk)>,
    pub(crate) struct_tree_root: Option<(Ref, Chunk)>,

    pub(crate) struct_elements: Vec<Chunk>,
    pub(crate) page_labels: Vec<Chunk>,
    pub(crate) annotations: Vec<Chunk>,
    pub(crate) fonts: Vec<Chunk>,
    pub(crate) color_spaces: Vec<Chunk>,
    pub(crate) icc_profiles: Vec<Chunk>,
    pub(crate) destinations: Vec<Chunk>,
    pub(crate) ext_g_states: Vec<Chunk>,
    pub(crate) masks: Vec<Chunk>,
    pub(crate) x_objects: Vec<Chunk>,
    pub(crate) shading_functions: Vec<Chunk>,
    pub(crate) patterns: Vec<Chunk>,
    pub(crate) pages: Vec<Deferred<Chunk>>,
    pub(crate) images: Vec<Deferred<KrillaResult<Chunk>>>,
    pub(crate) embedded_files: Vec<Chunk>,

    pub(crate) metadata: Option<Metadata>,
}

impl ChunkContainer {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    // TODO: Split up into multiple methods?
    pub(crate) fn finish(self, sc: &mut SerializeContext) -> KrillaResult<Pdf> {
        let mut remapped_ref = Ref::new(1);
        let mut remapper = HashMap::new();

        // Allows us to estimate the capacity we will need for the new PDF.
        let mut chunks_byte_len = 0;

        // This traverses the chunks in the order that we will write them to the PDF and assigns new
        // references as we go. This gives us the advantage that the PDF will be numbered with
        // monotonically increasing numbers, which, while it is not a strict requirement for a valid
        // PDF, makes it a lot cleaner and might make implementing features like object streams
        // easier down the road.
        //
        // It also allows us to estimate the capacity we will need for the new PDF.
        self.visit(&mut |chunk| {
            for object_ref in chunk.refs() {
                let existing = remapper.insert(object_ref, remapped_ref.bump());
                debug_assert!(existing.is_none());
            }
            chunks_byte_len += chunk.len();
        })?;

        // Chunk length is not an exact number because the length might change as we renumber,
        // so we add a bit of a padding by multiplying with 1.1. The 200 is additional padding
        // for the document catalog. This hopefully allows us to avoid re-alloactions in the general
        // case, and thus give us better performance.
        let capacity = (chunks_byte_len as f32 * 1.1 + 200.0) as usize;
        let mut pdf = Pdf::with_capacity(capacity);
        sc.serialize_settings().pdf_version.set_version(&mut pdf);

        if sc.serialize_settings().ascii_compatible
            && !sc.serialize_settings().validator.requires_binary_header()
        {
            pdf.set_binary_marker(b"AAAA")
        }

        // Write the chunks in all the fields.
        self.visit(&mut |chunk| {
            chunk.renumber_into(&mut pdf, |old| remapper[&old]);
        })?;

        // TODO: Replace with `is_none_or` once MSRV allows to.
        let missing_title = match self.metadata.as_ref() {
            None => true,
            Some(m) => m.title.is_none(),
        };

        if missing_title {
            sc.register_validation_error(ValidationError::NoDocumentTitle);
        }

        // Write the PDF document info metadata.
        if let Some(metadata) = &self.metadata {
            metadata.serialize_document_info(
                &mut remapped_ref,
                &mut pdf,
                sc.serialize_settings().pdf_version,
            );
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

        let named_destinations = sc.global_objects.named_destinations.take();
        let embedded_files = sc.global_objects.embedded_files.take();

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
                catalog.pages(remapper[&pt.0]);
            }

            if let Some(meta_ref) = meta_ref {
                catalog.metadata(meta_ref);
            }

            if let Some(pl) = &self.page_label_tree {
                catalog.pair(Name(b"PageLabels"), remapper[&pl.0]);
            }

            if let Some(oi) = &self.destination_profiles {
                catalog.pair(Name(b"OutputIntents"), remapper[&oi.0]);
            }

            if let Some(lang) = self.metadata.and_then(|m| m.language).as_ref() {
                catalog.lang(TextStr(lang));
            } else {
                sc.register_validation_error(ValidationError::NoDocumentLanguage);
            }

            if let Some(st) = &self.struct_tree_root {
                catalog.pair(Name(b"StructTreeRoot"), remapper[&st.0]);
                let mut mark_info = catalog.mark_info();
                mark_info.marked(true);
                if sc.serialize_settings().pdf_version >= PdfVersion::Pdf16
                    && sc.serialize_settings().pdf_version < PdfVersion::Pdf20
                {
                    // We always set suspects to false because it's required by PDF/UA.
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
                catalog.outlines(remapper[&ol.0]);
            }

            if !named_destinations.is_empty() || !embedded_files.is_empty() {
                // Cannot use pdf-writer API here because it requires Ref's, while
                // we write our destinations directly into the array.
                let mut names = catalog.names();

                if !named_destinations.is_empty() {
                    let mut dest_name_tree = names.destinations();
                    let mut dest_name_entries = dest_name_tree.names();

                    // Sort to prevent inconsistent order.
                    let mut sorted = named_destinations.into_iter().collect::<Vec<_>>();
                    sorted.sort_by(|a, b| a.1.cmp(&b.1));

                    for (name, dest_ref) in sorted {
                        dest_name_entries.insert(Str(name.name.as_bytes()), remapper[&dest_ref]);
                    }

                    dest_name_entries.finish();
                    dest_name_tree.finish();
                }

                if !embedded_files.is_empty() {
                    let mut embedded_files_name_tree = names.embedded_files();
                    let mut embedded_name_entries = embedded_files_name_tree.names();

                    for (name, _ref) in &embedded_files {
                        embedded_name_entries.insert(Str(name.as_bytes()), remapper[_ref]);
                    }
                }
            }

            if !embedded_files.is_empty()
                && sc.serialize_settings().validator.allows_associated_files()
            {
                let mut associated_files = catalog.insert(Name(b"AF")).array().typed();
                for _ref in embedded_files.values() {
                    associated_files.item(remapper[_ref]).finish();
                }
            }

            catalog.finish();
        }

        Ok(pdf)
    }
}

/// Visits all chunks in a type.
trait Visit {
    fn visit(&self, f: &mut impl FnMut(&Chunk)) -> KrillaResult<()>;
}

impl Visit for ChunkContainer {
    fn visit(&self, f: &mut impl FnMut(&Chunk)) -> KrillaResult<()> {
        self.page_tree.visit(f)?;
        self.outline.visit(f)?;
        self.page_label_tree.visit(f)?;
        self.destination_profiles.visit(f)?;
        self.struct_tree_root.visit(f)?;
        self.struct_elements.visit(f)?;
        self.page_labels.visit(f)?;
        self.annotations.visit(f)?;
        self.fonts.visit(f)?;
        self.color_spaces.visit(f)?;
        self.icc_profiles.visit(f)?;
        self.destinations.visit(f)?;
        self.ext_g_states.visit(f)?;
        self.masks.visit(f)?;
        self.x_objects.visit(f)?;
        self.shading_functions.visit(f)?;
        self.patterns.visit(f)?;
        self.pages.visit(f)?;
        self.images.visit(f)?;
        self.embedded_files.visit(f)?;
        Ok(())
    }
}

impl Visit for Chunk {
    fn visit(&self, f: &mut impl FnMut(&Chunk)) -> KrillaResult<()> {
        f(self);
        Ok(())
    }
}

impl Visit for Option<(Ref, Chunk)> {
    fn visit(&self, f: &mut impl FnMut(&Chunk)) -> KrillaResult<()> {
        if let Some((_, chunk)) = self {
            chunk.visit(f)?;
        }
        Ok(())
    }
}

impl<T: Visit + Send + Sync + 'static> Visit for Deferred<T> {
    fn visit(&self, f: &mut impl FnMut(&Chunk)) -> KrillaResult<()> {
        self.wait().visit(f)
    }
}

impl<T: Visit> Visit for KrillaResult<T> {
    fn visit(&self, f: &mut impl FnMut(&Chunk)) -> KrillaResult<()> {
        self.as_ref().map_err(|e| e.clone())?.visit(f)
    }
}

impl<T: Visit> Visit for Vec<T> {
    fn visit(&self, f: &mut impl FnMut(&Chunk)) -> KrillaResult<()> {
        for field in self {
            field.visit(f)?;
        }
        Ok(())
    }
}
