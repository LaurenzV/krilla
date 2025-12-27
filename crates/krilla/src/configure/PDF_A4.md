# Description
PDF/A-4 requires PDF a version 2.0 and defines three conformance levels,
in the following order from less strict to more strict:
- Level B
- Level U
- Level A

Level U is a subset of level A, and level B is a subset of level U.

See `README.md` for the meaning of each color.

## 5.1 Conforming files
- krilla does not use any deprecated features for PDF 2.0. 🟢

## 6.1 File structure
-

### 6.1.2 File header
- krilla only allows exporting with PDF 2.0 in this mode. 🟢
- krilla always writes the binary marker as written. 🟢

### 6.1.3 File trailer
- The trailer is written as defined. 🟢
- krilla does not support encryption. 🟢
- krilla does not write the `Info` attribute into the trailer. 🟢

### 6.1.4 Cross reference table
- The cross-reference table is always written as defined. 🟢

### 6.1.5 String objects
- pdf-writer always writes an even amount of digits for hexadecimal strings. 🟢

### 6.1.6 Stream objects
- krilla writes streams as mandated. 🟢
- krilla does not use the `F`, `FFilter` or `FDecodeParams` attributes. 🟢
- krilla does not use LZWDecode. 🟢
- krilla does not support the `Crypt` filter. 🟢

### 6.1.7 Name objects
- krilla does not use non-UTF8 names. 🟢 (TODO: what about user supplied ones?)

### 6.1.8 Indirect objects
- krilla always writes indirect objects as defined. 🟢

### 6.1.9 Inline image dictionaries
- krilla does not use inline images. 🟢

### 6.1.10 Linearized PDF
- krilla does not support linearization. 🟢

### 6.1.11 Permissions
- krilla does not support permissions. 🟢

### 6.1.12 Document catalog dictionary
- krilla always writes the Version entry as 2.0. 🟢

## 6.2 Graphics

### 6.2.1 General

### 6.2.2 Content streams
- krilla does not use custom content stream operators. 🟢

### 6.2.3 Output intent
- krilla always writes the output intent for this export mode. 🟢
- krilla does not use the `DestOutputProfileRef` attribute. 🟢
- krilla does not write multiple output intents (since PDF/X and PDF/E are not supported). 🟢
- krilla only uses a Display ICC profile for output intent. 🟢
- krilla only uses RGB for output intents. 🟢

### 6.2.4 Colour spaces
- krilla only uses device-independent colors in this export mode. 🟢 
- krilla uses compatible ICC profiles. 🟢
- krilla does not use the OPM entry. 🟢
- krilla never uses CMYK profiles as the destination profile. 🟢
- krilla does not use device color spaces in this mode. 🟢
- krilla ensures the Alternate space in Separation color spaces obeys the restrictions
  in the applicable clauses 🟢
- krilla does not support DeviceN color spaces. 🔵
- krilla fails export if a Separation colorant is associated with multiple different
  fallback color spaces 🟢
- krilla manages the `tintTransform` function and will always write the same function for
  the same color space 🟢

### 6.2.5 Extended graphics state
- krilla does not use the `TR`, `HTO`, `TR2` or `HT` keys. 🟢
- krilla does not use halftones. 🟢
- krilla does not use the `FL`, `BG`, `BG2`, `UCR` or `URC2` functions. 🟢

### 6.2.6 Flatness
- krilla does not use the flatness parameter. 🟢

### 6.2.7 Images
- krilla does not use the `Alternates` or `OPI` key. 🟢
- krilla disallows the `Interpolate` key in this mode. 🟢
- krilla does not support thumbnails. 🟢
- krilla does not support JPEG2000 images. 🟢

### 6.2.8 XObjects
- krilla does not use the `OPI` key in FormXObjects. 🟢
- krilla does not use reference XObjects. 🟢

### 6.2.9 Transparency
- krilla uses transparency as mandated. 🟢

### 6.2.10 Fonts
- krilla uses fonts as described in the spec. 🟢
- krilla always uses the IDENTITY-H encoding. 🟢
- krilla always embeds a CIDtoGIDMap for Type2 CID fonts. 🟢
- krilla always embeds cmaps and adds the WMode entry. 🟢
- krilla always embeds the font programs. 🟢
- krilla only uses glyphs referenced in the font. 🟢
- krilla derives the glyph width information from the font program, also for Type3 fonts. 🟢
- krilla does not use fonts in vertical writing mode. 🟢
- krilla only writes symbolic TrueType fonts. 🟢
- krilla does not use the `Encoding` entry in the font dictionary. 🟢
- krilla always writes the `ToUnicode` entry. 🟢
- krilla ensures Unicode values are always greater than 0 and not equal to U+FEFF or U+FFFE. 🟢
- krilla straight out forbids characters in the private use area. 🟢
- krilla disallows the .notdef glyph in this export mode. 🟢
- krilla checks the OpenType fsType field to ensure that fonts are legally embeddable. 🟢

## 6.3 Annotations

### 6.3.1 Annotation types
- krilla only supports link annotations. 🟢

### 6.3.2 Annotation dictionaries
- Annotation dictionaries always contain the `F` key and sets the values accordingly. 🟢
- krilla does not support text annotations. 🟢

### 6.3.3 Annotation appearances
- This section only contains provisions for readers. 🟢

### 6.3.4 Display of annotation contents
- krilla does not use appearence dictionaries. 🟢

## 6.4 Interactive forms

- krilla does not support interactive forms. 🟢

## 6.5 Digital signatures

- krilla does not support digital signatures. 🟢

## 6.6 Action

### 6.6.1 General
- krilla does not support any of the named actions. 🟢

### 6.6.2 Handling of JavaScript actions
- krilla does not support JavaScript actions. 🟢

### 6.6.3 Trigger events
- krilla does not use the `AA` entry anywhere. 🟢

### 6.6.4 Handling of GoToR, GoToE, URI and SubmitForm actions
- This section only contains provisions for readers. 🟢

## 6.7 Metadata

### 6.7.1 General

### 6.7.2 Metadata streams
- krilla always requires metadata in this export mode. 🟢
- krilla does not use the `bytes` and `encoding` attributes. 🟢
- krilla uses the outlined namespaces and prefixes. 🟢
- krilla does not currently have an associated file containing the embedded file specification. However,
  it seems like there is an issue with the spec because I can't find that table entry in the PDF 2.0 spec? 
  And no validator I tried seems to complain about this.🔴

### 6.7.3 Version identification
- krilla always writes the required version identification parts. 🟢

### 6.7.4 File identifiers
- krilla always adds a document identifier to the metadata. 🟢
- krilla does not use the history entry. 🟢

### 6.7.5 File provenance information
- krilla does not need to write file provenance information. 🟢

## 6.8 Logical structure
- Logical structure is only optional, so krilla does not make any enforcements here, but the
  recommendation is documented. 🟢

## 6.9 Embedded files
- krilla straight out forbids embedding files in this export mode. 🟢

## 6.10 Optional content
- krilla does not support optional content. 🟢

## 6.11 Use of alternate presentations and transitions
- krilla does not use the `AlternatePresentations` entry. 🟢

## 6.12 Document requirements
- krilla does not use the `Requirements` key. 🟢

## 6.13 Print scaling
- This section only contains provisions for processors. 🟢

## 6.14 Geospatial
- krilla does not support any of the geospatial features. 🟢

## 6.15 Measurement Properties
- krilla does not use any of the measurement properties. 🟢

# A4-F
- krilla allows embedding any files in this export mode. 🟢
- krilla always writes the `AFRelationship`, `Desc`, `UF` and `F` strings in this case. 🟢
- krilla always writes the corresponding file identification. 🟢

# A4-E
- krilla does not support any of the 3D features. 🟢
- krilla deals similarly with embedded files similarly to A4-F. 🟢
- krilla always writes the corresponding file identification. 🟢

