## [Unreleased]

### Added

### Changed

### Fixed

## [0.4.0] - 2024-04-02

### Added
- Added support for creating tagged PDFs.
- Added support for PDF/A2-A, PDF/A3-A, PDF/A4.
- Added support for PDF/UA-1.
- Added support for creating documents with PDF versions 1.4 to 2.0.
- Improved support for images (including ICC profiles).
- Added support for optional multi-threaded export.
- Added `CropBox` `BleedBox` `TrimBox` `ArtBox` to page boundary settings.
- Added support for the `Interpolation` attribute in images.
- Added support for attaching arbitrary files to PDFs.
- Added support for quad points in link annotations.
- Added feature to set "locations" while drawing to a surface to associate
  errors with specific contexts.

### Changed
- Color space handling has been overhauled.
- Removed variable font support, as its implementation was not optimal.
- Merge `PdfVersion` and `PdfStandard` into a `Configuration` struct.
- Fills and strokes are now optionall encoded as 1 instead of separately.
- SVG rendering has been extracted into an external `krilla-svg` crate.
- The public API has been overhauled a lot.
- Gradient stops don't take generics anymore.

### Fixed
- Fixed a potential overflow when writing CMAP.
- Fixed a spacing issue when writing text.
- Various bug fixes for embedding certain images and fonts in certain export modes.

## [0.3.0] - 2024-10-01
### Added
- Improved support for stroking text.
- Error handling has been revamped.
- Added support for using CMYK ICC profiles.
- Added support for changing the text direction.
- Support the `currentColor` attribute of SVG fonts.
- Add initial support for validated export. 
  Currently, only PDF/A2-U, PDF/A2-B, PDF/A3-U, PDF/A3-B are supported.

### Fixed
- Fixed bug with gradients on text not working properly for some spread methods.

## [0.2.0] - 2024-09-12
### Added
- Support writing outlined glyphs.
- Support for vertical text writing.
- Support for adding document metadata.

### Changed
- Streamlined how colors are created.
- SVG settings are now passed with `draw_svg` instead of in `SerializeSettings`.
- Removed some unused errors.

### Fixed