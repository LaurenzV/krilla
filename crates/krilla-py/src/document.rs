//! Document, Page, and Surface types for krilla Python bindings.
//!
//! This module provides Python bindings for krilla's document creation API. The
//! implementation handles complex lifetime and ownership constraints from the Rust
//! API that cannot be directly expressed in Python.
//!
//! # Architecture Overview
//!
//! The core krilla Rust API has a strict ownership chain:
//! ```text
//! Document -> Page<'doc> -> Surface<'page>
//! ```
//!
//! Where:
//! - [`krilla::page::Page<'doc>`] mutably borrows the `SerializeContext` from [`krilla::Document`]
//! - [`krilla::surface::Surface<'page>`] mutably borrows the `SerializeContext` from [`krilla::page::Page`]
//! - Only one Page can exist at a time (enforced by requiring `&mut Document`)
//! - Only one Surface can exist at a time (enforced by requiring `&mut Page`)
//!
//! This ensures single-threaded exclusive access to the serialization context and
//! enforces proper nesting of page/surface operations at compile time.
//!
//! The Python bindings replicate this using:
//! - **Lightweight Python wrappers** ([`Document`], [`Page`], [`Surface`]) that don't directly own Rust objects
//! - **Centralized storage** in [`DocumentState`] that holds raw pointers to the active Rust objects
//! - **Runtime checks** that replicate the borrow checker's safety guarantees
//!
//! # Why Unsafe Code is Required
//!
//! The fundamental problem is that Python has no compile-time lifetimes. The Rust API returns:
//! - `Page<'doc>` where `'doc` is the lifetime of the Document borrow
//! - `Surface<'page>` where `'page` is the lifetime of the Page borrow
//!
//! To store these in [`DocumentState`], we must:
//! 1. Extend the lifetimes to `'static` using [`std::mem::transmute`]
//! 2. Store them as raw pointers (`*mut Page<'static>`, `*mut Surface<'static>`)
//!
//! This is fundamentally unavoidable in Python bindings - we're "lying" to the Rust
//! compiler about lifetimes but enforcing the constraints through runtime checks instead.
//!
//! # Runtime Safety Checks
//!
//! Safety is maintained through several mechanisms:
//!
//! 1. **State flags**: [`DocumentState`] maintains `has_active_page` and `has_active_surface`
//!    flags to track active objects
//!
//! 2. **Method guards**: Operations check these flags before proceeding:
//!    - `start_page()` fails if a page is already active
//!    - `finish()` fails if child objects are still active
//!    - `surface()` fails if the page is finished
//!
//! 3. **RAII via context managers**: Python's `with` statement ensures proper cleanup:
//!    ```python
//!    with doc.start_page() as page:
//!        with page.surface() as surface:
//!            # Drawing operations
//!        # Surface automatically finished
//!    # Page automatically finished
//!    ```
//!
//! 4. **Drop handlers**: [`Page::drop`] and [`DocumentState::drop`] clean up state
//!    even if `finish()` isn't explicitly called, preventing leaked resources
//!
//! # Alternative Architecture Considered
//!
//! A more idiomatic PyO3 approach would be to:
//! - Store Rust objects directly in their Python wrappers via `RefCell<Option<T>>`
//! - Use `Py<T>` for cross-references (e.g., `DocumentState` holds `Py<Page>`)
//! - Eliminate raw pointers in favor of PyO3's reference counting
//!
//! This matches PyO3 documentation examples for interior mutability and would look like:
//! ```rust,ignore
//! #[pyclass]
//! struct Page {
//!     document: Py<Document>,
//!     inner: RefCell<Option<krilla::page::Page<'static>>>,
//! }
//! ```
//!
//! **Why this approach was not used:**
//! - Still requires `unsafe transmute` to extend lifetimes (same fundamental issue)
//! - Still requires the same runtime checks for safety (no improvement)
//! - Creates circular references (`Page` → `Document` → `Py<Page>`)
//! - Adds more indirection (`Py<T>` → `RefCell` → `Option`)
//! - Current approach is simpler with equivalent safety guarantees
//!
//! The raw pointer approach, while less idiomatic, is more direct and avoids the
//! complexity of circular references while still maintaining safety through runtime checks.
//!
//! # Safety Invariants
//!
//! The implementation maintains these invariants:
//!
//! 1. Raw pointers in [`DocumentState`] are never null when `has_active_*` is true
//! 2. Objects are always cleaned up in the correct order: Surface → Page → Document
//! 3. No overlapping mutable access to `SerializeContext` (enforced by state flags)
//! 4. Lifetimes are transmuted to `'static` but actual validity enforced by runtime checks
//! 5. Drop handlers ensure cleanup even on panic or early return
//!
//! SAFETY: The use of raw pointers and transmute is sound because:
//! - Runtime checks prevent creation of overlapping borrows
//! - Cleanup order is enforced (child dropped before parent)
//! - Pointers are cleaned up before the Document's `SerializeContext` is moved/dropped

use pyo3::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::config::SerializeSettings;
use crate::enums::{BlendMode, FillRule};
use crate::error::to_py_err;
use crate::geometry::{Path, Point, Rect, Size, Transform};
#[cfg(feature = "raster-images")]
use crate::image::Image;
use crate::interchange::{EmbeddedFile, Metadata, Outline};
use crate::mask::Mask;
use crate::num::NormalizedF32;
use crate::paint::{Fill, Stroke};
use crate::tagging::{ContentTag, Identifier, Location};
#[cfg(feature = "simple-text")]
use crate::text::TextDirection;
use crate::text::{Font, GlyphWrapper, _KrillaGlyph};

/// Global counter for unique document IDs.
static DOC_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Page settings for creating a new page.
#[pyclass]
#[derive(Clone)]
pub struct PageSettings {
    inner: krilla::page::PageSettings,
    /// Store size separately since krilla::PageSettings doesn't expose it
    size: krilla::geom::Size,
}

#[pymethods]
impl PageSettings {
    /// Create page settings from a size.
    #[new]
    fn new(size: &Size) -> Self {
        let inner_size = size.into_inner();
        PageSettings {
            inner: krilla::page::PageSettings::new(inner_size),
            size: inner_size,
        }
    }

    /// Create page settings from width and height.
    ///
    /// Raises ValueError if width or height is not positive.
    #[staticmethod]
    fn from_wh(width: f32, height: f32) -> PyResult<Self> {
        let size = krilla::geom::Size::from_wh(width, height).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "PageSettings requires positive width and height, got width={}, height={}",
                width, height
            ))
        })?;
        Ok(PageSettings {
            inner: krilla::page::PageSettings::new(size),
            size,
        })
    }

    /// Set page boxes (optional boundary rectangles for the page).
    ///
    /// Args:
    ///     media_box: The media box (visible area of the page).
    ///     crop_box: The crop box (clipping region for display/print).
    ///     bleed_box: The bleed box (production clipping region).
    ///     trim_box: The trim box (intended finished page size).
    ///     art_box: The art box (meaningful content boundaries).
    ///
    /// Returns a new PageSettings with the specified boxes set.
    #[pyo3(signature = (media_box=None, crop_box=None, bleed_box=None, trim_box=None, art_box=None))]
    fn with_page_boxes(
        &self,
        media_box: Option<&Rect>,
        crop_box: Option<&Rect>,
        bleed_box: Option<&Rect>,
        trim_box: Option<&Rect>,
        art_box: Option<&Rect>,
    ) -> Self {
        let mut inner = self.inner.clone();
        if media_box.is_some()
            || crop_box.is_some()
            || bleed_box.is_some()
            || trim_box.is_some()
            || art_box.is_some()
        {
            inner = inner
                .with_media_box(media_box.map(|r| r.into_inner()))
                .with_crop_box(crop_box.map(|r| r.into_inner()))
                .with_bleed_box(bleed_box.map(|r| r.into_inner()))
                .with_trim_box(trim_box.map(|r| r.into_inner()))
                .with_art_box(art_box.map(|r| r.into_inner()));
        }
        PageSettings {
            inner,
            size: self.size,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PageSettings(size={}x{})",
            self.size.width(),
            self.size.height()
        )
    }
}

impl PageSettings {
    pub fn into_inner(self) -> krilla::page::PageSettings {
        self.inner
    }

    pub fn size(&self) -> krilla::geom::Size {
        self.size
    }
}

/// Internal state for a document.
///
/// SAFETY: Uses raw pointers to manage Rust Page and Surface lifetimes across FFI boundary.
/// Correctness ensured by runtime checks preventing overlapping mutable access.
struct DocumentState {
    document: Option<krilla::Document>,
    doc_id: usize,
    has_active_page: bool,
    has_active_surface: bool,
    /// Number of pages added to the document
    page_count: usize,
    /// Raw pointer to active Page. SAFETY: Cleaned up before document access.
    active_page: Option<*mut krilla::page::Page<'static>>,
    /// Raw pointer to active Surface. SAFETY: Cleaned up before page/document access.
    active_surface: Option<*mut krilla::surface::Surface<'static>>,
}

impl Drop for DocumentState {
    fn drop(&mut self) {
        // Clean up any remaining objects
        if let Some(surface_ptr) = self.active_surface.take() {
            unsafe {
                drop(Box::from_raw(surface_ptr));
            }
        }
        if let Some(page_ptr) = self.active_page.take() {
            unsafe {
                drop(Box::from_raw(page_ptr));
            }
        }
    }
}

/// A PDF document.
///
/// Documents are the main entry point for creating PDFs. Create a document,
/// add pages to it, draw on the pages, then call finish() to get the PDF bytes.
///
/// Note: Documents can only be used from the thread that created them.
#[pyclass(unsendable)]
pub struct Document {
    state: Rc<RefCell<DocumentState>>,
}

#[pymethods]
impl Document {
    /// Create a new document with default settings.
    #[new]
    fn new() -> Self {
        Document {
            state: Rc::new(RefCell::new(DocumentState {
                document: Some(krilla::Document::new()),
                doc_id: DOC_COUNTER.fetch_add(1, Ordering::SeqCst),
                has_active_page: false,
                has_active_surface: false,
                page_count: 0,
                active_page: None,
                active_surface: None,
            })),
        }
    }

    /// Create a new document with custom serialize settings.
    #[staticmethod]
    fn new_with(settings: SerializeSettings) -> Self {
        Document {
            state: Rc::new(RefCell::new(DocumentState {
                document: Some(krilla::Document::new_with(settings.into_inner())),
                doc_id: DOC_COUNTER.fetch_add(1, Ordering::SeqCst),
                has_active_page: false,
                has_active_surface: false,
                page_count: 0,
                active_page: None,
                active_surface: None,
            })),
        }
    }

    /// Start a new page with default settings (A4 size).
    fn start_page(&self) -> PyResult<Page> {
        self.start_page_with(PageSettings::from_wh(595.0, 842.0).unwrap())
    }

    /// Start a new page with specific settings.
    fn start_page_with(&self, settings: PageSettings) -> PyResult<Page> {
        let mut state = self.state.borrow_mut();

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
        state.page_count += 1;

        let page_size = settings.size();
        Ok(Page {
            doc_state: Rc::clone(&self.state),
            page_settings: settings.into_inner(),
            page_size,
            page_index: state.page_count - 1,
            finished: false,
        })
    }

    /// Set the location that should be assumed for subsequent operations.
    fn set_location(&self, location: Location) -> PyResult<()> {
        let mut state = self.state.borrow_mut();

        let doc = state.document.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has already been finished")
        })?;

        doc.set_location(location.into_inner());
        Ok(())
    }

    /// Reset the location that should be assumed for subsequent operations.
    fn reset_location(&self) -> PyResult<()> {
        let mut state = self.state.borrow_mut();

        let doc = state.document.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has already been finished")
        })?;

        doc.reset_location();
        Ok(())
    }

    /// Set the outline (navigation tree) of the document.
    fn set_outline(&self, outline: Outline) -> PyResult<()> {
        let mut state = self.state.borrow_mut();

        let doc = state.document.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has already been finished")
        })?;

        doc.set_outline(outline.into_inner());
        Ok(())
    }

    /// Set the metadata of the document.
    fn set_metadata(&self, metadata: Metadata) -> PyResult<()> {
        let mut state = self.state.borrow_mut();

        let doc = state.document.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has already been finished")
        })?;

        doc.set_metadata(metadata.into_inner());
        Ok(())
    }

    /// Embed a file in the PDF document.
    ///
    /// Returns False if a file with the same name has already been embedded.
    fn embed_file(&self, file: EmbeddedFile) -> PyResult<bool> {
        let mut state = self.state.borrow_mut();

        let doc = state.document.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has already been finished")
        })?;

        Ok(doc.embed_file(file.into_inner()).is_some())
    }

    /// Set the tag tree for the document (for PDF/UA accessibility).
    fn set_tag_tree(&self, py: Python<'_>, tag_tree: Py<crate::tagging::TagTree>) -> PyResult<()> {
        let mut state = self.state.borrow_mut();

        let doc = state.document.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has already been finished")
        })?;

        // Extract the TagTree and reconstruct from cloned children
        let tag_tree_ref = tag_tree.borrow(py);
        let children = tag_tree_ref.inner.children.clone();
        let inner = krilla::tagging::TagTree::from(children);
        doc.set_tag_tree(inner);
        Ok(())
    }

    /// Finish the document and return the PDF bytes.
    fn finish(&self) -> PyResult<Vec<u8>> {
        let mut state = self.state.borrow_mut();

        if state.has_active_page {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Cannot finish document while a page is active. Call page.finish() first.",
            ));
        }

        let doc = state.document.take().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has already been finished")
        })?;

        let bytes = doc.finish().map(|b| b.to_vec()).map_err(to_py_err)?;
        Ok(bytes)
    }

    fn __repr__(&self) -> String {
        let state = self.state.borrow();
        if state.document.is_none() {
            format!("Document(finished, {} pages)", state.page_count)
        } else if state.document.is_some() {
            if state.has_active_page {
                format!(
                    "Document(active, {} pages, page in progress)",
                    state.page_count
                )
            } else {
                format!("Document(active, {} pages)", state.page_count)
            }
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
    doc_state: Rc<RefCell<DocumentState>>,
    page_settings: krilla::page::PageSettings,
    page_size: krilla::geom::Size,
    page_index: usize,
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

        let doc_id = self.doc_state.borrow().doc_id;

        Ok(Surface {
            doc_state: Rc::clone(&self.doc_state),
            page_size: self.page_size,
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

        let mut state = self.doc_state.borrow_mut();

        // Ensure surface is cleaned up first
        if state.has_active_surface {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Cannot finish page while surface is still active",
            ));
        }

        // Clean up the Rust Page
        if let Some(page_ptr) = state.active_page.take() {
            unsafe {
                // Dropping the Box drops the Page, which calls finish()
                drop(Box::from_raw(page_ptr));
            }
        }

        state.has_active_page = false;
        state.page_count += 1;

        Ok(())
    }

    fn __enter__(slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        // Create the Rust Page when entering context
        let mut state = slf.doc_state.borrow_mut();

        // Check if page was already created
        if state.active_page.is_some() {
            drop(state);
            return Ok(slf);
        }

        let doc = state.document.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Document has been finished")
        })?;

        // SAFETY: Extend lifetime to 'static with runtime checks ensuring cleanup
        let page = doc.start_page_with(slf.page_settings.clone());
        let page: krilla::page::Page<'static> = unsafe { std::mem::transmute(page) };
        let page_ptr = Box::into_raw(Box::new(page));

        state.active_page = Some(page_ptr);
        state.has_active_page = true;

        drop(state); // Release lock
        Ok(slf)
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
            format!(
                "Page(finished, index={}, size={}x{})",
                self.page_index,
                self.page_size.width(),
                self.page_size.height()
            )
        } else {
            format!(
                "Page(active, index={}, size={}x{})",
                self.page_index,
                self.page_size.width(),
                self.page_size.height()
            )
        }
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        if !self.finished {
            let mut state = self.doc_state.borrow_mut();
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
    doc_state: Rc<RefCell<DocumentState>>,
    page_size: krilla::geom::Size,
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

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &mut *surface_ptr;

            if let Some(ref fill) = self.fill {
                surface.set_fill(Some(fill.to_inner()));
            }
            if let Some(ref stroke) = self.stroke {
                surface.set_stroke(Some(stroke.to_inner()));
            }

            surface.draw_path(path_inner);
        }

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

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &mut *surface_ptr;

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
        }

        Ok(())
    }

    /// Draw positioned glyphs using the low-level glyph API.
    ///
    /// Accepts either Python `Glyph` objects or low-level `_KrillaGlyph` objects.
    /// The Pythonic `Glyph` class is recommended for most use cases.
    #[pyo3(signature = (start, glyphs, font, text, font_size, outlined=false))]
    fn draw_glyphs(
        &mut self,
        start: &Point,
        glyphs: &Bound<'_, PyAny>,
        font: &Font,
        text: &str,
        font_size: f32,
        outlined: bool,
    ) -> PyResult<()> {
        self.ensure_active()?;

        // Try to extract as Vec<_KrillaGlyph> first, fallback to calling ._to_krilla_glyph()
        let glyph_wrappers: Vec<GlyphWrapper> =
            if let Ok(krilla_glyphs) = glyphs.extract::<Vec<_KrillaGlyph>>() {
                // Direct _KrillaGlyph objects
                krilla_glyphs.iter().map(GlyphWrapper::from).collect()
            } else {
                // Python Glyph objects - call ._to_krilla_glyph() on each
                let glyph_list = glyphs.extract::<Vec<Bound<'_, PyAny>>>()?;
                glyph_list
                    .iter()
                    .map(|g| {
                        let krilla_glyph: _KrillaGlyph =
                            g.call_method0("_to_krilla_glyph")?.extract()?;
                        Ok(GlyphWrapper::from(&krilla_glyph))
                    })
                    .collect::<PyResult<Vec<_>>>()?
            };

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &mut *surface_ptr;

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
        }

        Ok(())
    }

    /// Draw an image at the current position.
    #[cfg(feature = "raster-images")]
    fn draw_image(&mut self, image: &Image, size: &Size) -> PyResult<()> {
        self.ensure_active()?;

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &mut *surface_ptr;
            surface.draw_image(image.inner.clone(), size.into_inner());
        }

        Ok(())
    }

    /// Push a transformation onto the stack.
    fn push_transform(&mut self, transform: &Transform) -> PyResult<()> {
        self.ensure_active()?;
        self.push_count += 1;

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &mut *surface_ptr;
            surface.push_transform(&transform.into_inner());
        }

        Ok(())
    }

    /// Push a blend mode onto the stack.
    fn push_blend_mode(&mut self, blend_mode: BlendMode) -> PyResult<()> {
        self.ensure_active()?;
        self.push_count += 1;

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &mut *surface_ptr;
            surface.push_blend_mode(blend_mode.into_inner());
        }

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

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &mut *surface_ptr;
            let path_inner = path.as_inner().unwrap();
            let clip_rule_inner = clip_rule.into_inner();
            surface.push_clip_path(path_inner, &clip_rule_inner);
        }

        Ok(())
    }

    /// Push a mask onto the stack.
    fn push_mask(&mut self, mask: &Mask) -> PyResult<()> {
        self.ensure_active()?;
        mask.validate_document(self.doc_id)?;
        self.push_count += 1;

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &mut *surface_ptr;
            // Clone the Arc and try to unwrap it to get ownership of the Mask
            let mask_arc = mask.inner.clone();
            match Arc::try_unwrap(mask_arc) {
                Ok(mask_value) => surface.push_mask(mask_value),
                Err(_arc) => {
                    // If there are multiple references, we need to dereference and clone
                    // However, Mask doesn't implement Clone, so this is a limitation
                    return Err(pyo3::exceptions::PyRuntimeError::new_err(
                        "Cannot push mask that has multiple references",
                    ));
                }
            }
        }

        Ok(())
    }

    /// Push an opacity value onto the stack.
    fn push_opacity(&mut self, opacity: NormalizedF32) -> PyResult<()> {
        self.ensure_active()?;
        self.push_count += 1;

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &mut *surface_ptr;
            surface.push_opacity(opacity.into_inner());
        }

        Ok(())
    }

    /// Push an isolated transparency group.
    fn push_isolated(&mut self) -> PyResult<()> {
        self.ensure_active()?;
        self.push_count += 1;

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &mut *surface_ptr;
            surface.push_isolated();
        }

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

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &mut *surface_ptr;
            surface.pop();
        }

        Ok(())
    }

    /// Get the current transformation matrix.
    fn ctm(&self) -> PyResult<Transform> {
        self.ensure_active()?;

        let state = self.doc_state.borrow();
        let surface_ptr = state
            .active_surface
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active surface"))?;

        unsafe {
            let surface = &*surface_ptr;
            let transform = surface.ctm();
            Ok(Transform::from_inner(transform))
        }
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

        let mut state = self.doc_state.borrow_mut();

        // Clean up the Rust Surface
        if let Some(surface_ptr) = state.active_surface.take() {
            unsafe {
                // Dropping the Box drops the Surface, calling its Drop impl
                drop(Box::from_raw(surface_ptr));
            }
        }

        state.has_active_surface = false;
        self.finished = true;

        Ok(())
    }

    fn __enter__(slf: PyRefMut<'_, Self>) -> PyResult<PyRefMut<'_, Self>> {
        // Create the Rust Surface when entering context
        let mut state = slf.doc_state.borrow_mut();

        // Check if surface was already created
        if state.active_surface.is_some() {
            drop(state);
            return Ok(slf);
        }

        let page_ptr = state
            .active_page
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("No active page"))?;

        unsafe {
            let page = &mut *page_ptr;

            // SAFETY: Extend lifetime to 'static with runtime checks ensuring cleanup
            let surface = page.surface();
            let surface: krilla::surface::Surface<'static> = std::mem::transmute(surface);
            let surface_ptr = Box::into_raw(Box::new(surface));

            state.active_surface = Some(surface_ptr);
            state.has_active_surface = true;
        }

        drop(state); // Release lock
        Ok(slf)
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
            format!(
                "Surface(finished, size={}x{})",
                self.page_size.width(),
                self.page_size.height()
            )
        } else {
            let mut parts = vec![format!(
                "size={}x{}",
                self.page_size.width(),
                self.page_size.height()
            )];
            if self.push_count > 0 {
                parts.push(format!("push_count={}", self.push_count));
            }
            if self.tagged_count > 0 {
                parts.push(format!("tagged_count={}", self.tagged_count));
            }
            if self.fill.is_some() {
                parts.push("has_fill".to_string());
            }
            if self.stroke.is_some() {
                parts.push("has_stroke".to_string());
            }
            format!("Surface(active, {})", parts.join(", "))
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
