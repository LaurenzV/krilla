# API Reference

Complete API documentation for krilla Python bindings, automatically generated from source code and type stubs.

```{toctree}
:maxdepth: 4

../autoapi/krilla/index
```

## Quick Links

### Core Types
- {py:class}`krilla.Document` - Main PDF document
- {py:class}`krilla.Page` - Individual page
- {py:class}`krilla.Surface` - Drawing surface
- {py:class}`krilla.PageSettings` - Page configuration

### Geometry
- {py:class}`krilla.Point`, {py:class}`krilla.Size`, {py:class}`krilla.Rect`
- {py:class}`krilla.Transform` - Affine transformations
- {py:class}`krilla.Path`, {py:class}`krilla.PathBuilder`

### Paint & Color
- {py:class}`krilla.Paint`, {py:class}`krilla.Fill`, {py:class}`krilla.Stroke`
- {py:class}`krilla.LinearGradient`, {py:class}`krilla.RadialGradient`, {py:class}`krilla.SweepGradient`
- {py:mod}`krilla.color` module: {py:func}`~krilla.color.rgb`, {py:func}`~krilla.color.luma`, {py:func}`~krilla.color.cmyk`

### Text & Images
- {py:class}`krilla.Font`, {py:class}`krilla.Glyph`
- {py:class}`krilla.Image` (requires `raster-images` feature)

### Configuration
- {py:class}`krilla.PdfVersion`, {py:class}`krilla.Validator`, {py:class}`krilla.Configuration`
- {py:class}`krilla.SerializeSettings`
