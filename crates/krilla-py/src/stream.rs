//! Stream types for krilla Python bindings.
//!
//! This module provides StreamBuilder for creating streams that can be used
//! with masks and patterns. Due to Rust lifetime constraints, we use a
//! command-buffer approach: drawing operations are collected and then
//! replayed when finish() is called.

use pyo3::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::enums::FillRule;
use crate::geometry::{Path, Size, Transform};
use crate::num::NormalizedF32;
use crate::paint::{Fill, Stroke};

/// An encoded stream of drawing instructions.
///
/// Streams are created using StreamBuilder and can be used to create
/// masks, patterns, and graphics.
#[pyclass]
#[derive(Clone)]
pub struct Stream {
    pub(crate) inner: krilla::stream::Stream,
}

#[pymethods]
impl Stream {
    fn __repr__(&self) -> String {
        "Stream(...)".to_string()
    }
}

impl Stream {
    pub fn into_inner(self) -> krilla::stream::Stream {
        self.inner
    }

    pub fn from_inner(inner: krilla::stream::Stream) -> Self {
        Stream { inner }
    }
}

/// A drawing operation that can be replayed on a surface.
/// Path operations store the path directly and are consumed during replay.
pub(crate) enum DrawOp {
    SetFill(Option<Fill>),
    SetStroke(Option<Stroke>),
    DrawPath(Option<krilla::geom::Path>),
    PushTransform(krilla::geom::Transform),
    PushClipPath(Option<krilla::geom::Path>, krilla::paint::FillRule),
    PushOpacity(krilla::num::NormalizedF32),
    PushIsolated,
    Pop,
}

/// Internal state for StreamBuilder.
pub(crate) struct StreamBuilderState {
    size: krilla::geom::Size,
    operations: Vec<DrawOp>,
    current_fill: Option<Fill>,
    current_stroke: Option<Stroke>,
    push_count: usize,
    finished: bool,
}

/// Builder for creating streams.
///
/// StreamBuilder provides a drawing context for creating content that can
/// be used with masks and patterns.
///
/// Example:
/// ```python
/// builder = StreamBuilder(Size.from_wh(100, 100))
/// surface = builder.surface()
/// surface.set_fill(Fill(paint=Paint.from_rgb(color.rgb(255, 0, 0))))
/// pb = PathBuilder()
/// pb.push_rect(Rect.from_xywh(0, 0, 100, 100))
/// surface.draw_path(pb.finish())
/// surface.finish()
/// stream = builder.finish()
/// ```
#[pyclass(unsendable)]
pub struct StreamBuilder {
    state: Rc<RefCell<StreamBuilderState>>,
}

#[pymethods]
impl StreamBuilder {
    /// Create a new StreamBuilder with the given size.
    #[new]
    fn new(size: &Size) -> PyResult<Self> {
        let inner_size = size.into_inner();
        Ok(StreamBuilder {
            state: Rc::new(RefCell::new(StreamBuilderState {
                size: inner_size,
                operations: Vec::new(),
                current_fill: None,
                current_stroke: None,
                push_count: 0,
                finished: false,
            })),
        })
    }

    /// Get the drawing surface for this stream builder.
    ///
    /// Draw operations on the returned surface will be included in the
    /// stream when finish() is called.
    fn surface(&self) -> PyResult<StreamSurface> {
        let state = self.state.borrow();
        if state.finished {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "StreamBuilder has already been finished",
            ));
        }
        drop(state);

        Ok(StreamSurface {
            builder_state: Rc::clone(&self.state),
            finished: false,
        })
    }

    /// Finish the stream builder and return the stream.
    ///
    /// This replays all recorded drawing operations to produce the final stream.
    fn finish(&self) -> PyResult<Stream> {
        let mut state = self.state.borrow_mut();

        if state.finished {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "StreamBuilder has already been finished",
            ));
        }

        if state.push_count != 0 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "StreamBuilder has {} unbalanced push operations",
                state.push_count
            )));
        }

        state.finished = true;

        // Create a temporary document to build the stream
        let mut doc = krilla::Document::new();
        let page_settings = krilla::page::PageSettings::new(state.size);
        let mut page = doc.start_page_with(page_settings);
        let mut surface = page.surface();
        let mut stream_builder = surface.stream_builder();

        // Get the stream builder's surface and replay operations
        {
            let mut stream_surface = stream_builder.surface();

            for op in state.operations.iter_mut() {
                match op {
                    DrawOp::SetFill(fill) => {
                        stream_surface.set_fill(fill.as_ref().map(|f| f.to_inner()));
                    }
                    DrawOp::SetStroke(stroke) => {
                        stream_surface.set_stroke(stroke.as_ref().map(|s| s.to_inner()));
                    }
                    DrawOp::DrawPath(path_opt) => {
                        if let Some(path) = path_opt.take() {
                            stream_surface.draw_path(&path);
                        }
                    }
                    DrawOp::PushTransform(transform) => {
                        stream_surface.push_transform(transform);
                    }
                    DrawOp::PushClipPath(path_opt, rule) => {
                        if let Some(path) = path_opt.take() {
                            stream_surface.push_clip_path(&path, rule);
                        }
                    }
                    DrawOp::PushOpacity(opacity) => {
                        stream_surface.push_opacity(*opacity);
                    }
                    DrawOp::PushIsolated => {
                        stream_surface.push_isolated();
                    }
                    DrawOp::Pop => {
                        stream_surface.pop();
                    }
                }
            }

            stream_surface.finish();
        }

        let stream = stream_builder.finish();

        // Clean up - we don't need the page content
        surface.finish();
        page.finish();

        Ok(Stream::from_inner(stream))
    }

    fn __repr__(&self) -> String {
        let state = self.state.borrow();
        if state.finished {
            "StreamBuilder(finished)".to_string()
        } else {
            format!(
                "StreamBuilder(size={}x{}, ops={})",
                state.size.width(),
                state.size.height(),
                state.operations.len()
            )
        }
    }
}

/// A drawing surface for a StreamBuilder.
///
/// This surface records drawing operations that will be replayed when
/// the parent StreamBuilder's finish() method is called.
#[pyclass(unsendable)]
pub struct StreamSurface {
    builder_state: Rc<RefCell<StreamBuilderState>>,
    finished: bool,
}

#[pymethods]
impl StreamSurface {
    /// Set the fill properties for subsequent draw operations.
    #[pyo3(signature = (fill=None))]
    fn set_fill(&mut self, fill: Option<Fill>) -> PyResult<()> {
        self.ensure_active()?;
        let mut state = self.builder_state.borrow_mut();
        state.current_fill = fill.clone();
        state.operations.push(DrawOp::SetFill(fill));
        Ok(())
    }

    /// Get the current fill properties.
    fn get_fill(&self) -> PyResult<Option<Fill>> {
        self.ensure_active()?;
        Ok(self.builder_state.borrow().current_fill.clone())
    }

    /// Set the stroke properties for subsequent draw operations.
    #[pyo3(signature = (stroke=None))]
    fn set_stroke(&mut self, stroke: Option<Stroke>) -> PyResult<()> {
        self.ensure_active()?;
        let mut state = self.builder_state.borrow_mut();
        state.current_stroke = stroke.clone();
        state.operations.push(DrawOp::SetStroke(stroke));
        Ok(())
    }

    /// Get the current stroke properties.
    fn get_stroke(&self) -> PyResult<Option<Stroke>> {
        self.ensure_active()?;
        Ok(self.builder_state.borrow().current_stroke.clone())
    }

    /// Draw a path with the current fill and stroke settings.
    fn draw_path(&mut self, path: &mut Path) -> PyResult<()> {
        self.ensure_active()?;

        let path_inner = path.take_inner().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Path was already consumed")
        })?;

        let mut state = self.builder_state.borrow_mut();
        state.operations.push(DrawOp::DrawPath(Some(path_inner)));
        Ok(())
    }

    /// Push a transformation onto the stack.
    fn push_transform(&mut self, transform: &Transform) -> PyResult<()> {
        self.ensure_active()?;
        let mut state = self.builder_state.borrow_mut();
        state.push_count += 1;
        state
            .operations
            .push(DrawOp::PushTransform(transform.into_inner()));
        Ok(())
    }

    /// Push a clip path onto the stack.
    fn push_clip_path(&mut self, path: &mut Path, clip_rule: FillRule) -> PyResult<()> {
        self.ensure_active()?;

        let path_inner = path.take_inner().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Path was already consumed")
        })?;

        let mut state = self.builder_state.borrow_mut();
        state.push_count += 1;
        state
            .operations
            .push(DrawOp::PushClipPath(Some(path_inner), clip_rule.into_inner()));
        Ok(())
    }

    /// Push an opacity value onto the stack.
    fn push_opacity(&mut self, opacity: NormalizedF32) -> PyResult<()> {
        self.ensure_active()?;
        let mut state = self.builder_state.borrow_mut();
        state.push_count += 1;
        state
            .operations
            .push(DrawOp::PushOpacity(opacity.into_inner()));
        Ok(())
    }

    /// Push an isolated transparency group.
    fn push_isolated(&mut self) -> PyResult<()> {
        self.ensure_active()?;
        let mut state = self.builder_state.borrow_mut();
        state.push_count += 1;
        state.operations.push(DrawOp::PushIsolated);
        Ok(())
    }

    /// Pop the last pushed state from the stack.
    fn pop(&mut self) -> PyResult<()> {
        self.ensure_active()?;
        let mut state = self.builder_state.borrow_mut();

        if state.push_count == 0 {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "pop() called without matching push",
            ));
        }

        state.push_count -= 1;
        state.operations.push(DrawOp::Pop);
        Ok(())
    }

    /// Finish the surface.
    fn finish(&mut self) -> PyResult<()> {
        if self.finished {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "StreamSurface has already been finished",
            ));
        }

        // Note: We don't check push_count here because StreamBuilder.finish() will check it
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
        Ok(false)
    }

    fn __repr__(&self) -> String {
        if self.finished {
            "StreamSurface(finished)".to_string()
        } else {
            "StreamSurface(active)".to_string()
        }
    }
}

impl StreamSurface {
    fn ensure_active(&self) -> PyResult<()> {
        if self.finished {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "StreamSurface has already been finished",
            ));
        }
        if self.builder_state.borrow().finished {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Parent StreamBuilder has been finished",
            ));
        }
        Ok(())
    }
}
