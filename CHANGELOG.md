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