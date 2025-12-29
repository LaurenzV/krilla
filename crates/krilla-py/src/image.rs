//! Image types for krilla Python bindings.
//!
//! This module is only available when the `raster-images` feature is enabled.

use pyo3::prelude::*;

/// A raster image for embedding in PDFs.
#[pyclass]
#[derive(Clone)]
pub struct Image {
    pub(crate) inner: krilla::image::Image,
}

#[pymethods]
impl Image {
    /// Load an image from PNG data.
    ///
    /// Args:
    ///     data: PNG file contents
    ///     interpolate: Whether to interpolate when scaling
    ///
    /// Returns:
    ///     An Image object, or raises an exception if loading fails.
    #[staticmethod]
    #[pyo3(signature = (data, interpolate=true))]
    fn from_png(data: &[u8], interpolate: bool) -> PyResult<Self> {
        krilla::image::Image::from_png(data.to_vec().into(), interpolate)
            .map(|img| Image { inner: img })
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    /// Load an image from JPEG data.
    ///
    /// Args:
    ///     data: JPEG file contents
    ///     interpolate: Whether to interpolate when scaling
    ///
    /// Returns:
    ///     An Image object, or raises an exception if loading fails.
    #[staticmethod]
    #[pyo3(signature = (data, interpolate=true))]
    fn from_jpeg(data: &[u8], interpolate: bool) -> PyResult<Self> {
        krilla::image::Image::from_jpeg(data.to_vec().into(), interpolate)
            .map(|img| Image { inner: img })
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    /// Load an image from GIF data.
    ///
    /// Args:
    ///     data: GIF file contents
    ///     interpolate: Whether to interpolate when scaling
    ///
    /// Returns:
    ///     An Image object, or raises an exception if loading fails.
    #[staticmethod]
    #[pyo3(signature = (data, interpolate=true))]
    fn from_gif(data: &[u8], interpolate: bool) -> PyResult<Self> {
        krilla::image::Image::from_gif(data.to_vec().into(), interpolate)
            .map(|img| Image { inner: img })
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    /// Load an image from WebP data.
    ///
    /// Args:
    ///     data: WebP file contents
    ///     interpolate: Whether to interpolate when scaling
    ///
    /// Returns:
    ///     An Image object, or raises an exception if loading fails.
    #[staticmethod]
    #[pyo3(signature = (data, interpolate=true))]
    fn from_webp(data: &[u8], interpolate: bool) -> PyResult<Self> {
        krilla::image::Image::from_webp(data.to_vec().into(), interpolate)
            .map(|img| Image { inner: img })
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    /// Create an image from raw RGBA8 data.
    ///
    /// Args:
    ///     data: Raw RGBA8 pixel data (4 bytes per pixel)
    ///     width: Image width in pixels
    ///     height: Image height in pixels
    ///
    /// Returns:
    ///     An Image object, or raises an exception if the data is invalid.
    #[staticmethod]
    fn from_rgba8(data: Vec<u8>, width: u32, height: u32) -> PyResult<Self> {
        let expected_len = (width * height * 4) as usize;
        if data.len() != expected_len {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Expected {} bytes for {}x{} RGBA8 image, got {}",
                expected_len,
                width,
                height,
                data.len()
            )));
        }

        Ok(Image {
            inner: krilla::image::Image::from_rgba8(data, width, height),
        })
    }

    /// Load an image from a Python Pillow (PIL) Image.
    ///
    /// This method accepts any Python object that has a `.save()` method compatible
    /// with PIL.Image.Image. The image is converted to PNG format in memory and then
    /// loaded.
    ///
    /// Args:
    ///     image: A PIL.Image.Image object or compatible object
    ///     interpolate: Whether to interpolate when scaling
    ///
    /// Returns:
    ///     An Image object, or raises an exception if conversion fails.
    ///
    /// Example:
    ///     >>> from PIL import Image as PILImage
    ///     >>> from krilla import Image
    ///     >>> pil_img = PILImage.new('RGB', (100, 100), color='red')
    ///     >>> krilla_img = Image.from_pil(pil_img)
    #[staticmethod]
    #[pyo3(signature = (image, interpolate=true))]
    fn from_pil(py: Python<'_>, image: &Bound<'_, PyAny>, interpolate: bool) -> PyResult<Self> {
        // Create a BytesIO buffer in memory
        let io = py.import("io")?;
        let bytes_io_class = io.getattr("BytesIO")?;
        let bytes_io = bytes_io_class.call0()?;

        // Save the PIL image to the buffer as PNG
        image.call_method1("save", (&bytes_io, "PNG"))?;

        // Get the bytes from the buffer
        bytes_io.call_method1("seek", (0,))?;
        let png_bytes = bytes_io.call_method0("getvalue")?;
        let png_data: &[u8] = png_bytes.extract()?;

        // Use the existing from_png method to load the PNG data
        Self::from_png(png_data, interpolate)
    }

    /// Get the image dimensions.
    ///
    /// Returns:
    ///     A tuple of (width, height) in pixels.
    fn size(&self) -> (u32, u32) {
        self.inner.size()
    }

    /// Get the image width.
    #[getter]
    fn width(&self) -> u32 {
        self.inner.size().0
    }

    /// Get the image height.
    #[getter]
    fn height(&self) -> u32 {
        self.inner.size().1
    }

    fn __repr__(&self) -> String {
        let (w, h) = self.inner.size();
        format!("Image({}x{})", w, h)
    }
}

impl Image {
    pub fn into_inner(self) -> krilla::image::Image {
        self.inner
    }
}
