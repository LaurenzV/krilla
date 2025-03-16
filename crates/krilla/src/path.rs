//! Path-related properties.

use crate::{Rect, Transform};

/// A path.
pub struct Path(pub(crate) tiny_skia_path::Path);

impl Path {
    /// Apply a transformation to the path.
    pub fn transform(self, transform: Transform) -> Option<Self> {
        Some(Self(self.0.transform(transform.to_tsp())?))
    }
}

/// A path builder.
#[derive(Default)]
pub struct PathBuilder(tiny_skia_path::PathBuilder);

impl PathBuilder {
    /// Create a new path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds beginning of a contour.
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y)
    }

    /// Adds a line from the last point.
    pub fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y)
    }

    /// Adds a quad curve from the last point to `x`, `y`.
    pub fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1, y1, x, y)
    }
    /// Adds a cubic curve from the last point to `x`, `y`.
    pub fn cubic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.cubic_to(x1, y1, x2, y2, x, y)
    }

    /// Close the current contour.
    pub fn close(&mut self) {
        self.0.close()
    }

    /// Push a rectangle to the path.
    pub fn push_rect(&mut self, rect: Rect) {
        self.0.push_rect(rect.to_tsp())
    }

    /// Finish the current path.
    pub fn finish(self) -> Option<Path> {
        Some(Path(self.0.finish()?))
    }
}
