//! Geometry types for krilla Python bindings.

use pyo3::prelude::*;

/// A 2D point.
#[pyclass(frozen)]
#[derive(Clone, Copy)]
pub struct Point {
    inner: krilla::geom::Point,
}

#[pymethods]
impl Point {
    /// Create a new point from x and y coordinates.
    #[staticmethod]
    fn from_xy(x: f32, y: f32) -> Self {
        Point {
            inner: krilla::geom::Point::from_xy(x, y),
        }
    }

    /// The x coordinate.
    #[getter]
    fn x(&self) -> f32 {
        self.inner.x
    }

    /// The y coordinate.
    #[getter]
    fn y(&self) -> f32 {
        self.inner.y
    }

    fn __repr__(&self) -> String {
        format!("Point(x={}, y={})", self.inner.x, self.inner.y)
    }

    fn __eq__(&self, other: &Point) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.x.to_bits().hash(&mut hasher);
        self.inner.y.to_bits().hash(&mut hasher);
        hasher.finish()
    }
}

impl Point {
    pub fn into_inner(self) -> krilla::geom::Point {
        self.inner
    }
}

/// A 2D size with width and height.
#[pyclass(frozen)]
#[derive(Clone, Copy)]
pub struct Size {
    inner: krilla::geom::Size,
}

#[pymethods]
impl Size {
    /// Create a new size from width and height.
    ///
    /// Returns None if width or height is not positive.
    #[staticmethod]
    fn from_wh(width: f32, height: f32) -> Option<Self> {
        krilla::geom::Size::from_wh(width, height).map(|s| Size { inner: s })
    }

    /// The width.
    #[getter]
    fn width(&self) -> f32 {
        self.inner.width()
    }

    /// The height.
    #[getter]
    fn height(&self) -> f32 {
        self.inner.height()
    }

    fn __repr__(&self) -> String {
        format!("Size(width={}, height={})", self.width(), self.height())
    }

    fn __eq__(&self, other: &Size) -> bool {
        self.inner == other.inner
    }
}

impl Size {
    pub fn into_inner(self) -> krilla::geom::Size {
        self.inner
    }

    pub fn from_inner(inner: krilla::geom::Size) -> Self {
        Size { inner }
    }
}

/// A rectangle defined by left, top, right, bottom coordinates.
#[pyclass(frozen)]
#[derive(Clone, Copy)]
pub struct Rect {
    inner: krilla::geom::Rect,
}

#[pymethods]
impl Rect {
    /// Create a new rectangle from x, y, width, height.
    ///
    /// Returns None if width or height is not positive.
    #[staticmethod]
    fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Option<Self> {
        krilla::geom::Rect::from_xywh(x, y, width, height).map(|r| Rect { inner: r })
    }

    /// Create a new rectangle from left, top, right, bottom.
    ///
    /// Returns None if the resulting width or height is not positive.
    #[staticmethod]
    fn from_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Option<Self> {
        krilla::geom::Rect::from_ltrb(left, top, right, bottom).map(|r| Rect { inner: r })
    }

    /// Left edge x coordinate.
    #[getter]
    fn left(&self) -> f32 {
        self.inner.left()
    }

    /// Top edge y coordinate.
    #[getter]
    fn top(&self) -> f32 {
        self.inner.top()
    }

    /// Right edge x coordinate.
    #[getter]
    fn right(&self) -> f32 {
        self.inner.right()
    }

    /// Bottom edge y coordinate.
    #[getter]
    fn bottom(&self) -> f32 {
        self.inner.bottom()
    }

    /// Width of the rectangle.
    #[getter]
    fn width(&self) -> f32 {
        self.inner.width()
    }

    /// Height of the rectangle.
    #[getter]
    fn height(&self) -> f32 {
        self.inner.height()
    }

    /// Transform the rectangle by an affine transformation.
    fn transform(&self, transform: &Transform) -> Option<Rect> {
        self.inner
            .transform(transform.inner)
            .map(|r| Rect { inner: r })
    }

    fn __repr__(&self) -> String {
        format!(
            "Rect(left={}, top={}, right={}, bottom={})",
            self.left(),
            self.top(),
            self.right(),
            self.bottom()
        )
    }

    fn __eq__(&self, other: &Rect) -> bool {
        self.inner == other.inner
    }
}

impl Rect {
    pub fn into_inner(self) -> krilla::geom::Rect {
        self.inner
    }

    pub fn from_inner(inner: krilla::geom::Rect) -> Self {
        Rect { inner }
    }
}

/// A 2D affine transformation matrix.
#[pyclass(frozen)]
#[derive(Clone, Copy)]
pub struct Transform {
    pub(crate) inner: krilla::geom::Transform,
}

#[pymethods]
impl Transform {
    /// Create an identity transformation (no change).
    #[staticmethod]
    pub fn identity() -> Self {
        Transform {
            inner: krilla::geom::Transform::identity(),
        }
    }

    /// Create a transformation from matrix components.
    ///
    /// The matrix is: [[sx, kx, tx], [ky, sy, ty], [0, 0, 1]]
    #[staticmethod]
    fn from_row(sx: f32, ky: f32, kx: f32, sy: f32, tx: f32, ty: f32) -> Self {
        Transform {
            inner: krilla::geom::Transform::from_row(sx, ky, kx, sy, tx, ty),
        }
    }

    /// Create a translation transformation.
    #[staticmethod]
    fn from_translate(tx: f32, ty: f32) -> Self {
        Transform {
            inner: krilla::geom::Transform::from_translate(tx, ty),
        }
    }

    /// Create a scaling transformation.
    #[staticmethod]
    fn from_scale(sx: f32, sy: f32) -> Self {
        Transform {
            inner: krilla::geom::Transform::from_scale(sx, sy),
        }
    }

    /// Create a skewing transformation.
    #[staticmethod]
    fn from_skew(kx: f32, ky: f32) -> Self {
        Transform {
            inner: krilla::geom::Transform::from_skew(kx, ky),
        }
    }

    /// Create a rotation transformation (angle in degrees).
    #[staticmethod]
    fn from_rotate(angle: f32) -> Self {
        Transform {
            inner: krilla::geom::Transform::from_rotate(angle),
        }
    }

    /// Create a rotation transformation around a point (angle in degrees).
    #[staticmethod]
    fn from_rotate_at(angle: f32, tx: f32, ty: f32) -> Self {
        Transform {
            inner: krilla::geom::Transform::from_rotate_at(angle, tx, ty),
        }
    }

    /// Scale x component.
    #[getter]
    fn sx(&self) -> f32 {
        self.inner.sx()
    }

    /// Scale y component.
    #[getter]
    fn sy(&self) -> f32 {
        self.inner.sy()
    }

    /// Skew x component.
    #[getter]
    fn kx(&self) -> f32 {
        self.inner.kx()
    }

    /// Skew y component.
    #[getter]
    fn ky(&self) -> f32 {
        self.inner.ky()
    }

    /// Translate x component.
    #[getter]
    fn tx(&self) -> f32 {
        self.inner.tx()
    }

    /// Translate y component.
    #[getter]
    fn ty(&self) -> f32 {
        self.inner.ty()
    }

    /// Compute the inverse transformation.
    fn invert(&self) -> Option<Transform> {
        self.inner.invert().map(|t| Transform { inner: t })
    }

    fn __repr__(&self) -> String {
        format!(
            "Transform(sx={}, ky={}, kx={}, sy={}, tx={}, ty={})",
            self.sx(),
            self.ky(),
            self.kx(),
            self.sy(),
            self.tx(),
            self.ty()
        )
    }

    fn __eq__(&self, other: &Transform) -> bool {
        self.inner == other.inner
    }
}

impl Transform {
    pub fn into_inner(self) -> krilla::geom::Transform {
        self.inner
    }

    pub fn from_inner(inner: krilla::geom::Transform) -> Self {
        Transform { inner }
    }
}

/// A geometric path consisting of line segments and curves.
///
/// Paths are created using PathBuilder and are immutable once created.
#[pyclass]
pub struct Path {
    // Option because PathBuilder.finish() consumes the path
    pub(crate) inner: Option<krilla::geom::Path>,
}

#[pymethods]
impl Path {
    /// Transform the path by an affine transformation.
    ///
    /// Returns a new transformed path, or None if the transformation fails.
    /// Note: This consumes the original path.
    fn transform(&mut self, transform: &Transform) -> PyResult<Option<Path>> {
        let path = self
            .inner
            .take()
            .ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("Path was already consumed"))?;

        Ok(path.transform(transform.inner).map(|p| Path { inner: Some(p) }))
    }

    fn __repr__(&self) -> String {
        if self.inner.is_some() {
            "Path(...)".to_string()
        } else {
            "Path(<consumed>)".to_string()
        }
    }
}

impl Path {
    /// Take the inner path, leaving None in its place.
    pub fn take(&mut self) -> Option<krilla::geom::Path> {
        self.inner.take()
    }

    /// Borrow the inner path.
    pub fn as_inner(&self) -> Option<&krilla::geom::Path> {
        self.inner.as_ref()
    }
}

/// Builder for creating geometric paths.
///
/// Use the various methods to construct a path, then call finish() to get the Path.
/// Note that finish() consumes the builder.
#[pyclass]
pub struct PathBuilder {
    inner: Option<krilla::geom::PathBuilder>,
}

#[pymethods]
impl PathBuilder {
    /// Create a new empty path builder.
    #[new]
    fn new() -> Self {
        PathBuilder {
            inner: Some(krilla::geom::PathBuilder::new()),
        }
    }

    /// Move to a new point, starting a new subpath.
    fn move_to(&mut self, x: f32, y: f32) -> PyResult<()> {
        let builder = self.inner.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("PathBuilder was already consumed")
        })?;
        builder.move_to(x, y);
        Ok(())
    }

    /// Draw a line to a point.
    fn line_to(&mut self, x: f32, y: f32) -> PyResult<()> {
        let builder = self.inner.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("PathBuilder was already consumed")
        })?;
        builder.line_to(x, y);
        Ok(())
    }

    /// Draw a quadratic Bezier curve to a point.
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) -> PyResult<()> {
        let builder = self.inner.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("PathBuilder was already consumed")
        })?;
        builder.quad_to(x1, y1, x, y);
        Ok(())
    }

    /// Draw a cubic Bezier curve to a point.
    fn cubic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) -> PyResult<()> {
        let builder = self.inner.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("PathBuilder was already consumed")
        })?;
        builder.cubic_to(x1, y1, x2, y2, x, y);
        Ok(())
    }

    /// Close the current subpath.
    fn close(&mut self) -> PyResult<()> {
        let builder = self.inner.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("PathBuilder was already consumed")
        })?;
        builder.close();
        Ok(())
    }

    /// Add a rectangle to the path.
    fn push_rect(&mut self, rect: &Rect) -> PyResult<()> {
        let builder = self.inner.as_mut().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("PathBuilder was already consumed")
        })?;
        builder.push_rect(rect.into_inner());
        Ok(())
    }

    /// Finish building the path and return it.
    ///
    /// This consumes the builder. Returns None if the path is empty or invalid.
    fn finish(&mut self) -> PyResult<Option<Path>> {
        let builder = self.inner.take().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("PathBuilder was already consumed")
        })?;
        Ok(builder.finish().map(|p| Path { inner: Some(p) }))
    }

    fn __repr__(&self) -> String {
        if self.inner.is_some() {
            "PathBuilder(...)".to_string()
        } else {
            "PathBuilder(<consumed>)".to_string()
        }
    }
}
