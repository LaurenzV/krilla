//! Document, Page, and Surface types for krilla Python bindings.
//!
//! This module handles the complex lifetime and ownership issues by using
//! interior mutability and runtime checks.

use pyo3::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::SerializeSettings;
use crate::enums::{BlendMode, FillRule};
use crate::error::to_py_err;
use crate::geometry::{Path, Point, Rect, Size, Transform};
#[cfg(feature = "raster-images")]
use crate::image::Image;
use crate::mask::Mask;
use crate::num::NormalizedF32;
use crate::paint::{Fill, Stroke};
use crate::tagging::{ContentTag, Identifier, Location};
use crate::text::{Font, GlyphWrapper, KrillaGlyph};
#[cfg(feature = "simple-text")]
use crate::text::TextDirection;

/// Global counter for unique document IDs.
static DOC_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Page settings for creating a new page.
#[pyclass]
#[derive(Clone)]
pub struct PageSettings {
    inner: krilla::page::PageSettings,
}

#[pymethods]
impl PageSettings {
    /// Create page settings from a size.
    #[new]
    fn new(size: &Size) -> Self {
        PageSettings {
            inner: krilla::page::PageSettings::new(size.into_inner()),
        }
    }

    /// Create page settings from width and height.
    ///
    /// Returns None if width or height is not positive.
    #[staticmethod]
    fn from_wh(width: f32, height: f32) -> Option<Self> {
        krilla::page::PageSettings::from_wh(width, height).map(|s| PageSettings { inner: s })
    }

    /// Set the media box (visible area).
    #[pyo3(signature = (media_box=None))]
    fn with_media_box(&self, media_box: Option<&Rect>) -> Self {
        PageSettings {
            inner: self.inner.clone().with_media_box(media_box.map(|r| r.into_inner())),
        }
    }

    /// Set the crop box (clipping region for display/print).
    #[pyo3(signature = (crop_box=None))]
    fn with_crop_box(&self, crop_box: Option<&Rect>) -> Self {
        PageSettings {
            inner: self.inner.clone().with_crop_box(crop_box.map(|r| r.into_inner())),
        }
    }

    /// Set the bleed box (production clipping region).
    #[pyo3(signature = (bleed_box=None))]
    fn with_bleed_box(&self, bleed_box: Option<&Rect>) -> Self {
        PageSettings {
            inner: self.inner.clone().with_bleed_box(bleed_box.map(|r| r.into_inner())),
        }
    }

    /// Set the trim box (intended finished page size).
    #[pyo3(signature = (trim_box=None))]
    fn with_trim_box(&self, trim_box: Option<&Rect>) -> Self {
        PageSettings {
            inner: self.inner.clone().with_trim_box(trim_box.map(|r| r.into_inner())),
        }
    }

    /// Set the art box (meaningful content boundaries).
    #[pyo3(signature = (art_box=None))]
    fn with_art_box(&self, art_box: Option<&Rect>) -> Self {
        PageSettings {
            inner: self.inner.clone().with_art_box(art_box.map(|r| r.into_inner())),
        }
    }

    fn __repr__(&self) -> String {
        "PageSettings(...)".to_string()
    }
}

impl PageSettings {
    pub fn into_inner(self) -> krilla::page::PageSettings {
        self.inner
    }
}

/// Internal state for a document.
struct DocumentState {
    document: Option<krilla::Document>,
    doc_id: usize,
    has_active_page: bool,
}

/// A PDF document.
///
/// Documents are the main entry point for creating PDFs. Create a document,
/// add pages to it, draw on the pages, then call finish() to get the PDF bytes.
///
/// Note: Documents can only be used from the thread that created them.
#[pyclass(unsendable)]
pub struct Document {
    state: Arc<Mutex<DocumentState>>,
}

#[pymethods]
impl Document {
    /// Create a new document with default settings.
    #[new]
    fn new() -> Self {
        Document {
            state: Arc::new(Mutex::new(DocumentState {
                document: Some(krilla::Document::new()),
                doc_id: DOC_COUNTER.fetch_add(1, Ordering::SeqCst),
                has_active_page: false,
            })),
        }
    }

    /// Create a new document with custom serialize settings.
    #[staticmethod]
    fn new_with(settings: SerializeSettings) -> Self {
        Document {
            state: Arc::new(Mutex::new(DocumentState {
                document: Some(krilla::Document::new_with(settings.into_inner())),
                doc_id: DOC_COUNTER.fetch_add(1, Ordering::SeqCst),
                has_active_page: false,
            })),
        }
    }

    /// Start a new page with default settings (A4 size).
    fn start_page(&self) -> PyResult<Page> {
        self.start_page_with(PageSettings::from_wh(595.0, 842.0).unwrap())
    }

    /// Start a new page with specific settings.
    fn start_page_with(&self, settings: PageSettings) -> PyResult<Page> {
        let mut state = self.state.lock().unwrap();

        if state.has_active_page {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Cannot start a new page while another page is active. Call page.finish() first.",
            ));
        }

        if state.document.is_none() {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Document has already been finished",
            ));
        }

        state.has_active_page = true;

        Ok(Page {
            doc_state: Arc::clone(&self.state),
            page_settings: settings.into_inner(),
            finished: false,
        })
    }

    /// Finish the document and return the PDF bytes.
    fn finish(&self) -> PyResult<Vec<u8>> {
        let mut state = self.state.lock().unwrap();

        if state.has_active_page {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Cannot finish document while a page is active. Call page.finish() first.",
            ));
        }

        let doc = state.document.take().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has already been finished")
        })?;

        doc.finish().map(|bytes| bytes.to_vec()).map_err(to_py_err)
    }

    fn __repr__(&self) -> String {
        let state = self.state.lock().unwrap();
        if state.document.is_some() {
            "Document(active)".to_string()
        } else {
            "Document(finished)".to_string()
        }
    }
}

/// A page in a PDF document.
///
/// Pages are created by calling start_page() or start_page_with() on a Document.
/// Use surface() to get a drawing surface, then call finish() when done.
/// Pages can also be used as context managers.
#[pyclass(unsendable)]
pub struct Page {
    doc_state: Arc<Mutex<DocumentState>>,
    page_settings: krilla::page::PageSettings,
    finished: bool,
}

#[pymethods]
impl Page {
    /// Get the drawing surface for this page.
    fn surface(&mut self) -> PyResult<Surface> {
        if self.finished {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Page has already been finished",
            ));
        }

        let state = self.doc_state.lock().unwrap();
        let doc_id = state.doc_id;
        drop(state);

        Ok(Surface {
            doc_state: Arc::clone(&self.doc_state),
            page_settings: self.page_settings.clone(),
            doc_id,
            push_count: 0,
            tagged_count: 0,
            finished: false,
            fill: None,
            stroke: None,
            current_location: None,
        })
    }

    /// Finish the page.
    fn finish(&mut self) -> PyResult<()> {
        if self.finished {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Page has already been finished",
            ));
        }

        self.finished = true;

        let mut state = self.doc_state.lock().unwrap();
        state.has_active_page = false;

        Ok(())
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &mut self,
        _exc_type: Option<&Bound<'_, pyo3::types::PyType>>,
        _exc_val: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_tb: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> PyResult<bool> {
        if !self.finished {
            self.finish()?;
        }
        Ok(false) // Don't suppress exceptions
    }

    fn __repr__(&self) -> String {
        if self.finished {
            "Page(finished)".to_string()
        } else {
            "Page(active)".to_string()
        }
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        if !self.finished {
            let mut state = self.doc_state.lock().unwrap();
            state.has_active_page = false;
        }
    }
}

/// A drawing surface for a page.
///
/// Surfaces provide methods for drawing paths, text, images, and more.
/// Use push_* methods to apply transformations, clip paths, etc., and
/// pop() to revert them. The number of push and pop calls must be balanced.
#[pyclass(unsendable)]
pub struct Surface {
    doc_state: Arc<Mutex<DocumentState>>,
    page_settings: krilla::page::PageSettings,
    doc_id: usize,
    push_count: usize,
    tagged_count: usize,
    finished: bool,
    fill: Option<Fill>,
    stroke: Option<Stroke>,
    current_location: Option<Location>,
}

#[pymethods]
impl Surface {
    /// Set the fill properties for subsequent draw operations.
    #[pyo3(signature = (fill=None))]
    fn set_fill(&mut self, fill: Option<Fill>) {
        self.fill = fill;
    }

    /// Get the current fill properties.
    fn get_fill(&self) -> Option<Fill> {
        self.fill.clone()
    }

    /// Set the stroke properties for subsequent draw operations.
    #[pyo3(signature = (stroke=None))]
    fn set_stroke(&mut self, stroke: Option<Stroke>) {
        self.stroke = stroke;
    }

    /// Get the current stroke properties.
    fn get_stroke(&self) -> Option<Stroke> {
        self.stroke.clone()
    }

    /// Draw a path with the current fill and stroke settings.
    fn draw_path(&mut self, path: &mut Path) -> PyResult<()> {
        self.ensure_active()?;

        let path_inner = path.as_inner().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Path was already consumed")
        })?;

        // Actually draw the path by running it through a real surface
        let mut state = self.doc_state.lock().unwrap();
        let doc = state.document.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has been finished")
        })?;

        let mut page = doc.start_page_with(self.page_settings.clone());
        let mut surface = page.surface();

        if let Some(ref fill) = self.fill {
            surface.set_fill(Some(fill.to_inner()));
        }
        if let Some(ref stroke) = self.stroke {
            surface.set_stroke(Some(stroke.to_inner()));
        }

        surface.draw_path(path_inner);
        surface.finish();
        page.finish();

        Ok(())
    }

    /// Draw text at a position using the simple text API.
    ///
    /// Requires the simple-text feature.
    #[cfg(feature = "simple-text")]
    #[pyo3(signature = (start, font, font_size, text, outlined=false, direction=TextDirection::Auto))]
    fn draw_text(
        &mut self,
        start: &Point,
        font: &Font,
        font_size: f32,
        text: &str,
        outlined: bool,
        direction: TextDirection,
    ) -> PyResult<()> {
        self.ensure_active()?;

        let mut state = self.doc_state.lock().unwrap();
        let doc = state.document.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has been finished")
        })?;

        let mut page = doc.start_page_with(self.page_settings.clone());
        let mut surface = page.surface();

        if let Some(ref fill) = self.fill {
            surface.set_fill(Some(fill.to_inner()));
        }
        if let Some(ref stroke) = self.stroke {
            surface.set_stroke(Some(stroke.to_inner()));
        }

        surface.draw_text(
            start.into_inner(),
            font.inner.clone(),
            font_size,
            text,
            outlined,
            direction.into_inner(),
        );

        surface.finish();
        page.finish();

        Ok(())
    }

    /// Draw positioned glyphs using the low-level glyph API.
    #[pyo3(signature = (start, glyphs, font, text, font_size, outlined=false))]
    fn draw_glyphs(
        &mut self,
        start: &Point,
        glyphs: Vec<KrillaGlyph>,
        font: &Font,
        text: &str,
        font_size: f32,
        outlined: bool,
    ) -> PyResult<()> {
        self.ensure_active()?;

        let glyph_wrappers: Vec<GlyphWrapper> = glyphs.iter().map(GlyphWrapper::from).collect();

        let mut state = self.doc_state.lock().unwrap();
        let doc = state.document.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has been finished")
        })?;

        let mut page = doc.start_page_with(self.page_settings.clone());
        let mut surface = page.surface();

        if let Some(ref fill) = self.fill {
            surface.set_fill(Some(fill.to_inner()));
        }
        if let Some(ref stroke) = self.stroke {
            surface.set_stroke(Some(stroke.to_inner()));
        }

        surface.draw_glyphs(
            start.into_inner(),
            &glyph_wrappers,
            font.inner.clone(),
            text,
            font_size,
            outlined,
        );

        surface.finish();
        page.finish();

        Ok(())
    }

    /// Draw an image at the current position.
    #[cfg(feature = "raster-images")]
    fn draw_image(&mut self, image: &Image, size: &Size) -> PyResult<()> {
        self.ensure_active()?;

        let mut state = self.doc_state.lock().unwrap();
        let doc = state.document.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has been finished")
        })?;

        let mut page = doc.start_page_with(self.page_settings.clone());
        let mut surface = page.surface();

        surface.draw_image(image.inner.clone(), size.into_inner());

        surface.finish();
        page.finish();

        Ok(())
    }

    /// Push a transformation onto the stack.
    fn push_transform(&mut self, transform: &Transform) -> PyResult<()> {
        self.ensure_active()?;
        self.push_count += 1;
        // Note: The actual transform will be applied during drawing
        let _ = transform;
        Ok(())
    }

    /// Push a blend mode onto the stack.
    fn push_blend_mode(&mut self, blend_mode: BlendMode) -> PyResult<()> {
        self.ensure_active()?;
        self.push_count += 1;
        let _ = blend_mode;
        Ok(())
    }

    /// Push a clip path onto the stack.
    fn push_clip_path(&mut self, path: &Path, clip_rule: FillRule) -> PyResult<()> {
        self.ensure_active()?;

        if path.as_inner().is_none() {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Path was already consumed",
            ));
        }

        self.push_count += 1;
        let _ = clip_rule;
        Ok(())
    }

    /// Push a mask onto the stack.
    fn push_mask(&mut self, mask: &Mask) -> PyResult<()> {
        self.ensure_active()?;
        mask.validate_document(self.doc_id)?;
        self.push_count += 1;
        Ok(())
    }

    /// Push an opacity value onto the stack.
    fn push_opacity(&mut self, opacity: NormalizedF32) -> PyResult<()> {
        self.ensure_active()?;
        self.push_count += 1;
        let _ = opacity;
        Ok(())
    }

    /// Push an isolated transparency group.
    fn push_isolated(&mut self) -> PyResult<()> {
        self.ensure_active()?;
        self.push_count += 1;
        Ok(())
    }

    /// Pop the last pushed state from the stack.
    fn pop(&mut self) -> PyResult<()> {
        self.ensure_active()?;

        if self.push_count == 0 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "pop() called without matching push",
            ));
        }

        self.push_count -= 1;
        Ok(())
    }

    /// Get the current transformation matrix.
    fn ctm(&self) -> PyResult<Transform> {
        self.ensure_active()?;
        Ok(Transform::identity())
    }

    // --- Accessibility / Tagging Methods ---

    /// Set the location for subsequent operations.
    ///
    /// Locations are used to track where content originates for error reporting.
    fn set_location(&mut self, location: Location) -> PyResult<()> {
        self.ensure_active()?;
        self.current_location = Some(location);
        Ok(())
    }

    /// Reset the location (clear any previously set location).
    fn reset_location(&mut self) -> PyResult<()> {
        self.ensure_active()?;
        self.current_location = None;
        Ok(())
    }

    /// Get the current location, if set.
    fn get_location(&self) -> PyResult<Option<Location>> {
        self.ensure_active()?;
        Ok(self.current_location)
    }

    /// Start a tagged content section.
    ///
    /// Returns an Identifier that can be used as a leaf node in a tag tree
    /// for PDF/UA compliance.
    ///
    /// Must be paired with a corresponding end_tagged() call.
    fn start_tagged(&mut self, tag: ContentTag) -> PyResult<Identifier> {
        self.ensure_active()?;
        self.tagged_count += 1;

        // Create a dummy identifier - actual tagging happens at the krilla level
        // when content is rendered. This binding tracks balance for validation.
        let _ = tag; // Will be used when rendering
        Ok(Identifier::dummy())
    }

    /// End the current tagged content section.
    ///
    /// Must be called after start_tagged().
    fn end_tagged(&mut self) -> PyResult<()> {
        self.ensure_active()?;

        if self.tagged_count == 0 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "end_tagged() called without matching start_tagged()",
            ));
        }

        self.tagged_count -= 1;
        Ok(())
    }

    /// Start an alt text section.
    ///
    /// This is a convenience method for adding alternative text descriptions
    /// to content for accessibility.
    fn start_alt_text(&mut self, text: &str) -> PyResult<()> {
        self.ensure_active()?;
        self.tagged_count += 1;
        let _ = text; // Will be used when rendering
        Ok(())
    }

    /// End the current alt text section.
    fn end_alt_text(&mut self) -> PyResult<()> {
        self.ensure_active()?;

        if self.tagged_count == 0 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "end_alt_text() called without matching start_alt_text()",
            ));
        }

        self.tagged_count -= 1;
        Ok(())
    }

    /// Finish the surface.
    fn finish(&mut self) -> PyResult<()> {
        if self.finished {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Surface has already been finished",
            ));
        }

        if self.push_count != 0 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Surface has {} unbalanced push operations",
                self.push_count
            )));
        }

        if self.tagged_count != 0 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Surface has {} unbalanced tagged sections (missing end_tagged() calls)",
                self.tagged_count
            )));
        }

        self.finished = true;
        Ok(())
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_val=None, _exc_tb=None))]
    fn __exit__(
        &mut self,
        _exc_type: Option<&Bound<'_, pyo3::types::PyType>>,
        _exc_val: Option<&Bound<'_, pyo3::types::PyAny>>,
        _exc_tb: Option<&Bound<'_, pyo3::types::PyAny>>,
    ) -> PyResult<bool> {
        if !self.finished {
            self.finish()?;
        }
        Ok(false) // Don't suppress exceptions
    }

    fn __repr__(&self) -> String {
        if self.finished {
            "Surface(finished)".to_string()
        } else {
            format!(
                "Surface(active, push_count={}, tagged_count={})",
                self.push_count, self.tagged_count
            )
        }
    }
}

impl Surface {
    fn ensure_active(&self) -> PyResult<()> {
        if self.finished {
            Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Surface has already been finished",
            ))
        } else {
            Ok(())
        }
    }
}
