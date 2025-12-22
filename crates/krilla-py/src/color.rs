//! Color types for krilla Python bindings.

use pyo3::prelude::*;

/// An RGB color with 8-bit components.
#[pyclass(name = "RgbColor", frozen)]
#[derive(Clone, Copy)]
pub struct RgbColor {
    pub(crate) inner: krilla::color::rgb::Color,
}

#[pymethods]
impl RgbColor {
    /// Create a new RGB color.
    #[new]
    fn new(red: u8, green: u8, blue: u8) -> Self {
        RgbColor {
            inner: krilla::color::rgb::Color::new(red, green, blue),
        }
    }

    /// Create black color.
    #[staticmethod]
    fn black() -> Self {
        RgbColor {
            inner: krilla::color::rgb::Color::black(),
        }
    }

    /// Create white color.
    #[staticmethod]
    fn white() -> Self {
        RgbColor {
            inner: krilla::color::rgb::Color::white(),
        }
    }

    /// Red component (0-255).
    #[getter]
    fn red(&self) -> u8 {
        self.inner.red()
    }

    /// Green component (0-255).
    #[getter]
    fn green(&self) -> u8 {
        self.inner.green()
    }

    /// Blue component (0-255).
    #[getter]
    fn blue(&self) -> u8 {
        self.inner.blue()
    }

    fn __repr__(&self) -> String {
        format!(
            "rgb({}, {}, {})",
            self.inner.red(),
            self.inner.green(),
            self.inner.blue()
        )
    }

    fn __eq__(&self, other: &RgbColor) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

/// A grayscale (luma) color with 8-bit component.
#[pyclass(name = "LumaColor", frozen)]
#[derive(Clone, Copy)]
pub struct LumaColor {
    pub(crate) inner: krilla::color::luma::Color,
    lightness_val: u8,
}

#[pymethods]
impl LumaColor {
    /// Create a new grayscale color.
    #[new]
    fn new(lightness: u8) -> Self {
        LumaColor {
            inner: krilla::color::luma::Color::new(lightness),
            lightness_val: lightness,
        }
    }

    /// Create black color.
    #[staticmethod]
    fn black() -> Self {
        Self::new(0)
    }

    /// Create white color.
    #[staticmethod]
    fn white() -> Self {
        Self::new(255)
    }

    /// Lightness component (0-255).
    #[getter]
    fn lightness(&self) -> u8 {
        self.lightness_val
    }

    fn __repr__(&self) -> String {
        format!("luma({})", self.lightness_val)
    }

    fn __eq__(&self, other: &LumaColor) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

/// A CMYK color with 8-bit components.
#[pyclass(name = "CmykColor", frozen)]
#[derive(Clone, Copy)]
pub struct CmykColor {
    pub(crate) inner: krilla::color::cmyk::Color,
    cyan_val: u8,
    magenta_val: u8,
    yellow_val: u8,
    black_val: u8,
}

#[pymethods]
impl CmykColor {
    /// Create a new CMYK color.
    #[new]
    fn new(cyan: u8, magenta: u8, yellow: u8, black: u8) -> Self {
        CmykColor {
            inner: krilla::color::cmyk::Color::new(cyan, magenta, yellow, black),
            cyan_val: cyan,
            magenta_val: magenta,
            yellow_val: yellow,
            black_val: black,
        }
    }

    /// Cyan component (0-255).
    #[getter]
    fn cyan(&self) -> u8 {
        self.cyan_val
    }

    /// Magenta component (0-255).
    #[getter]
    fn magenta(&self) -> u8 {
        self.magenta_val
    }

    /// Yellow component (0-255).
    #[getter]
    fn yellow(&self) -> u8 {
        self.yellow_val
    }

    /// Black component (0-255).
    #[getter]
    fn black(&self) -> u8 {
        self.black_val
    }

    fn __repr__(&self) -> String {
        format!(
            "cmyk({}, {}, {}, {})",
            self.cyan_val,
            self.magenta_val,
            self.yellow_val,
            self.black_val
        )
    }

    fn __eq__(&self, other: &CmykColor) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

/// A color that can be RGB, grayscale, or CMYK.
#[pyclass(frozen)]
#[derive(Clone, Copy)]
pub struct Color {
    pub(crate) inner: krilla::color::Color,
}

#[pymethods]
impl Color {
    /// Create a color from an RGB color.
    #[staticmethod]
    fn from_rgb(color: &RgbColor) -> Self {
        Color {
            inner: krilla::color::Color::Rgb(color.inner),
        }
    }

    /// Create a color from a grayscale color.
    #[staticmethod]
    fn from_luma(color: &LumaColor) -> Self {
        Color {
            inner: krilla::color::Color::Luma(color.inner),
        }
    }

    /// Create a color from a CMYK color.
    #[staticmethod]
    fn from_cmyk(color: &CmykColor) -> Self {
        Color {
            inner: krilla::color::Color::Cmyk(color.inner),
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            krilla::color::Color::Rgb(c) => format!("Color.rgb({}, {}, {})", c.red(), c.green(), c.blue()),
            krilla::color::Color::Luma(_) => "Color.luma(...)".to_string(),
            krilla::color::Color::Cmyk(_) => "Color.cmyk(...)".to_string(),
        }
    }

    fn __eq__(&self, other: &Color) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
}

impl Color {
    pub fn into_inner(self) -> krilla::color::Color {
        self.inner
    }
}

/// Convenience function to create an RGB color.
#[pyfunction]
pub fn rgb(red: u8, green: u8, blue: u8) -> RgbColor {
    RgbColor::new(red, green, blue)
}

/// Convenience function to create a grayscale color.
#[pyfunction]
pub fn luma(lightness: u8) -> LumaColor {
    LumaColor::new(lightness)
}

/// Convenience function to create a CMYK color.
#[pyfunction]
pub fn cmyk(cyan: u8, magenta: u8, yellow: u8, black: u8) -> CmykColor {
    CmykColor::new(cyan, magenta, yellow, black)
}

/// Register the color submodule.
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<RgbColor>()?;
    m.add_class::<LumaColor>()?;
    m.add_class::<CmykColor>()?;
    m.add_class::<Color>()?;
    m.add_function(wrap_pyfunction!(rgb, m)?)?;
    m.add_function(wrap_pyfunction!(luma, m)?)?;
    m.add_function(wrap_pyfunction!(cmyk, m)?)?;
    Ok(())
}
