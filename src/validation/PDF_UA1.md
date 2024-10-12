# Description
PDF-UA/1 requires PDF 1.7.

See `README.md` for the meaning of each color.

# 6.1 General

- krilla only uses PDF 1.7.

# 6.2 Conforming files

- krilla  writes the `pdfuaid:part` attribute.
- krilla does not adhere to the file format providions.

# 6.3 Conforming reader

- 

# 6.4 Conforming reader

- 

# 6.3 Conforming reader

- 

# 7.1 General

- The fact that real content should be tagged is documented.
- The fact that artifacts should be marked are required.
- krilla never includes artifacts in the structure tree.
- krilla role maps all non-standard structure types.
- krilla does not overwrite non-standard structure types.
- krilla doesn't support any elements that flicker, flash or blink.
- The fact that information shall not be conveyed by contrast, colour, format or layout is documented.
- krilla does not support sounds.
- krilla forces the user to provide a document title.
- krilla always sets `DisplayDocTitle` to true for this mode.
- krilla can't really control if the user provides raster-based images as content.
- krilla always writes the `Suspects` value as false.

# 7.2 Text
