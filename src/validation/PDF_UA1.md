# Description
PDF-UA/1 requires a version <= PDF 1.7.

See `README.md` for the meaning of each color.

# 6.1 General

- krilla only uses PDF 1.7.

# 6.2 Conforming files

- krilla writes the `pdfuaid:part` attribute. 🟢
- krilla does adhere to the file format provisions. 🟢

# 6.3 Conforming reader

- 

# 6.4 Conforming reader

-

# 7.1 General

- The fact that real content should be tagged is documented. 🟣
- The fact that artifacts should be marked is documented.  🟣
- krilla never includes artifacts in the structure tree. 🟢
- krilla role maps all non-standard structure types. 🟢
- krilla does not overwrite non-standard structure types. 🟢
- krilla doesn't support any elements that flicker, flash or blink. 🟢
- The fact that information shall not be conveyed by contrast, colour, format or layout is documented. 🟣
- krilla does not support sounds. 🔵
- krilla forces the user to provide a document title. 🟢
- krilla always sets `DisplayDocTitle` to true for this mode. 🟢
- krilla can't really control if the user provides raster-based images as content. 🟠
- krilla always writes the `Suspects` value as false. 🟢

# 7.2 Text
- The fact that logical reading order should be followed is documented. 🟣
- krilla checks that every character is mapped to a codepoint. 🟢
- The fact that the user should make use of the natural language attributes is documented. 🟣
- The fact that stretchable characters should be marked with `ActualText` is documented. 🟣

# 7.3 Graphics
- The fact that figures should be tagged (as a figure or an artifact) is documented. 🟣
- The fact that figures should be followed by a caption is documented. 🟣
- The fact that an alternate text should be provided to figures is checked. 🟢
- The fact graphics that possess semantic value only in combination with other graphics should be tagged with a single Figure tag for each group is documented. 🟣
- The fact that a more accessible representation should be used if it exists is documented. 🟣

# 7.4 Headings

7.4.1:
- The fact that headings should be tagged is documented. 🟣
- krilla does not support the T key yet. 🟢

7.4.2:
- The information there is hardly enforceable in an automated way, so not documented yet. 🟠

7.4.3:
- krilla does not support heading levels higher than 6. 🔵

7.4.4:
- The information there is hardly enforceable in an automated way, so not documented yet. 🟠

# 7.5 Tables
- The fact that tables should include headers is documented. 🟣
- krilla always requires the user to provide a table header scope. 🟢
- The fact that table tagging structures should only be used to tag content presented within logical row and/or column relationships is documented. 🟣

# 7.6 Lists
- The fact that lists should be tagged is documented. 🟣
- The fact that Li, Lbl and LBody should be used is documented. 🟣
- krilla always forces writing the `ListNumbering` attribute for lists. 🟢

# 7.7 Mathematical expressions
- The fact that mathematical expressions should be wrapped in `Formula` is documented. 🟣
- The fact that mathematical expressions should have an alternate text is checked. 🟢

# 7.8 Page headers and footers
- The fact that headers and footers should be tagged is documented. 🟣

# 7.9 Notes and references
- The fact that footnotes, endnotes, note labels and references should be tagged is documented. 🟣
- The fact that footnotes and endnotes should be tagged with `Note` is documented. 🟣
- krilla always generates an ID for notes. 🟢

# 7.10 Optional content
- krilla does currently not support optional content. 🔵

# 7.11 Embedded files
- krilla does currently not support embedded files. 🔵

# 7.12 Article threads
- The fact that the logical reading order should be preserved is documented. 🟣

# 7.13 Digital signatures
- krilla does not support digital signatures. 🔵

# 7.14 Non-interactive forms
- krilla does not support forms. 🔵

# 7.15 XFA
- krilla does not support forms. 🔵

# 7.16 Security
- krilla does not support encryption. 🔵

# 7.17 Navigation
- krilla enforces setting a document outline. 🟢
- The fact that the outline should reflect the reading order is
  documented. 🟣
- The fact that page labels should be semantically appropriate is documented. 🟣

# 7.18 Annotations
7.18.1:
- The fact that annotations should reflect the reading order is documented. 🟣
- The fact that for visual formatting, annotations should 
  be tagged according to their semantic function is not documented. 🟠
- krilla ensures that annotations have an alternate text. 🟢

7.18.2:
- krilla only supports the default annotation types. 🟢
- krilla does not use the `TrapNet` annotation. 🔵

7.18.3
- krilla always writes the `TabOrder` property for pages that have a struct parent. 🟢

7.18.4
- krilla does not support widget annotations. 🔵

7.18.5
- The best practices for link tagging are documented. 🟣
- krilla enforces an alt text for all annotations. 🟢
- krilla never writes the `IMap` key for URIs. 🔵

7.18.6
- krilla does not have any support for media and file attachments. 🔵

# 7.19 Actions
- krilla does not support adding (JavaScript) scripts. 🔵

# 7.20 XObjects
- krilla never creates reference XObjects. 🔵
- krilla doesn't allow references to XObjects for tagging. 🔵

# 7.21 Fonts

7.21.1: -

7.21.2:
- krilla ensures to always conform to the PDF specification. 🟢

7.21.3.1:
- krilla always uses Identity-H for encoding. 🟢

7.21.3.2:
- krilla always includes a `CIDToGIDMap`. 🟢

7.21.3.3
- krilla always includes a cmap. 🟢
- krilla always writes the `WMode` entry in cmaps. 🟢
- krilla never references other cmaps in a cmap. 🟢

7.21.4.1
- krilla always embeds a cmap for all used fonts. 🟢
- The fact that only legally embeddable fonts should be used is
  documented. 🟣
- krilla export only succeeds if all glyphs are available in the font (otherwise subsetting fails). 🟢

7.21.4.2
- krilla doesn't include a CharSet in font descriptors. 🟢
- krilla always includes all CIDs in `CIDSet`. 🟢

7.21.5
- Font metrics should (hopefully) be as consistent as possible. 🟢

7.21.6
- krilla only uses CID fonts and not (PDF) TrueType fonts. 🟢

7.21.7
- krilla always includes the `ToUnicode` entry with corresponding mappings. 🟢
- krilla will fail export if 0, U+FEFF or U+FFEE is included. 🟢

7.21.8
- krilla will fail export if the .notdef glyph is included. 🟢