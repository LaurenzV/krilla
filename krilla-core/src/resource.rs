use crate::paint::LinearGradient;
use std::sync::Arc;

pub struct GraphicsState {}

pub enum PdfColorSpace {
    SRGB,
    D65Gray,
    LinearGradient(Arc<LinearGradient>),
    RadialGradient(Arc<LinearGradient>),
}

pub enum Resource {}
