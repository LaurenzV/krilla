# Description
PDF-UA/1 requires PDF 1.7.

See `README.md` for the meaning of each color.

# 6.1 General

- krilla only uses PDF 1.7.

# 6.2 Conforming files

- krilla writes the `pdfuaid:part` attribute. 🟢
- krilla does not adhere to the file format provisions.

# 6.3 Conforming reader

- 

# 6.4 Conforming reader

- 

# 6.3 Conforming reader

- 

# 7.1 General

- The fact that real content should be tagged is documented. 🟣
- The fact that artifacts should be marked is documented.  🟣
- krilla never includes artifacts in the structure tree. 🟢
- krilla role maps all non-standard structure types. 🟢
- krilla does not overwrite non-standard structure types. 🟢
- krilla doesn't support any elements that flicker, flash or blink. 🟢
- The fact that information shall not be conveyed by contrast, colour, format or layout is documented. 
- krilla does not support sounds.
- krilla forces the user to provide a document title. 🟢
- krilla always sets `DisplayDocTitle` to true for this mode. 🟢
- krilla can't really control if the user provides raster-based images as content.
- krilla always writes the `Suspects` value as false. 🟢

# 7.2 Text
- The fact that logical reading order should be followed is documented. 🟣
- krilla does currently not check that every character is mapped.
- The fact that the user should make use of the natural language attributes is documented. 🟣
- The fact that stretchable characters should be marked with `ActualText` is documented. 🟣

# 7.3 Graphics
- The fact that figures should be tagged (as a a figure or an artifact) is documented.
- The fact that figures should be followed by a caption is documented.
- The fact that an alternate text should be provided to figures is not checked yet.
- The fact graphics that posess semantic value only in combination with other graphics should be tagged with a single Figure tag for each group is documented.
- The fact that a more accessible representation should be used if it exists is documented.

# 7.4 Headings

7.4.1:
- The fact that headings should be tagged is documented.
- krilla does not support the T key yet.

7.4.2:
- The information there is hardly enforceable in an automated way, so not documented yet.

7.4.3:
- krilla does not support heading levels higher than 6.

7.4.4:
- The information there is hardly enforceable in an automated way, so not documented yet.

# 7.5 Tables
- The fact that tables should include headers is documented.
- The scope attribute is currently not supported by krilla.
- The fact that table tagging structures should only be used to tag content presented within logical row and/or column relationships is not documented.

# 7.6 Lists
- The fact that lists should be tagged
  is documented.
- The fact that Li, Lbl and LBody should be used is documented.
- krilla does not allow creating a `ListNumbering` attribute.

# 7.7 Mathematical expressions
- The fact that mathematical expressions should be wrapped in `Formula` is documented.
- The fact that mathematical expressions should have an alternate text is not checked yet.

# 7.8 Page headers and footers
- The fact that headers and footers should be tagged is documented.

# 7.9 Notes and references
- The fact that footnotes, endnotes, note labels and references should be tagged is documented.
- The fact that footnotes and endnotes should be tagged with `Note` is documented.
- krilla does currently not support the ID key.

# 7.10 Optional content
- krilla does currently not support optional content.

# 7.11 Embedded files
- krilla does currently not support embedded files.