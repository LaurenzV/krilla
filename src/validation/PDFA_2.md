# Description
PDF-A/2 requires PDF 1.7 and defines three conformance levels, 
in the following order from less strict to more strict:
- Level B
- Level U
- Level A

Level U is a subset of level A, and level B is a subset of level U.

See `README.md` for the meaning of each subclause.

## Level B

## 6.1 File structure

6.1.2: `pdf-writer` always write the file header as described in the spec. 🟢

6.1.3: 
- We always set the file ID. 🟢
- We do not support encryption. 🔵

6.1.4: `pdf-writer` always write the xref section as described in the spec. 🟢

6.1.5: -

6.1.6: `pdf-writer` always writes hex strings with an even number of characters. 🟢

6.1.7: 
- `pdf-writer` always write streams as described in the spec. 🟢
- krilla does never write streams referencing external files. 🔵
- krilla does never use `LZWDecode` or `Crypt`. 🔵

6.1.8: krilla only ever writes UTF-8 strings as names. 🟢

6.1.9: `pdf-writer` always writes indirect objects as described in the spec. 🟢

6.1.10: krilla does never use inline image dictionaries. 🔵

6.1.11: -

6.1.12: krilla doesn't support permissions. 🔵

6.1.13:
- `pdf-writer` uses i32 for integers. 🟢
- `pdf-writer` uses f32 for real numbers. 🟢
- krilla always uses the `new_str` and `new_text_str` methods of the SerializerContext to create them, 
  which returns a validation error in case it's too long. 🔵
- krilla trims the names of fonts, and all other names cannot be longer than 127. 🔵
- krilla fails export if more than 8388607 indirect objects exist. 🟢
- krilla fails export if a higher nesting-level than 28 exists. 🟢
- krilla does not use the DeviceN color space. 🟢
- krilla only uses u16 for CIDs. 🟢

## 6.2 Graphics

6.2.2:
- krilla doesn't use non-standard operators. 🟢
- krilla doesn't use the `ri` or `i` operator. 🟢
- krilla ensure that content stream has their own associated resource dictionary
(TODO: what about glyph streams in Type3 fonts?). 🟢

6.2.3: krilla never writes an `OutputIntent`. 🔴

6.2.4.1: krilla does not enforce using a device-independent color space / output intent. 🔴

6.2.4.2: 
- srgb/sgrey ICC profiles conform to ICC v4 specification. 🟢
- krilla does not support overprinting. 🔵

6.2.4.3: currently not fulfilled. 🔴

6.2.4.4: krilla does not support DeviceN/Separation color spaces. 🔵

6.2.4.5: currently not fulfilled. 🔴

6.2.5: krilla does not use the transfer functions, halftones, TR/HTP/RI/FL keys. 🔵

6.2.6: krilla does never define a rendering intent. 🔵

6.2.7: krilla is not a reader. 🔵

6.2.8.1: krilla does not use the `Alternates`/`Interpolate`/`Intent` keys for images. 🔵

6.2.8.2: krilla does not support thumbnails. 🔵

6.2.8.3: krilla embeds JPEG images by converting them to a sampled representation. 🔵

6.2.9.1: krilla does not use the `OPI`/`Subtype2`/`PS` keys for XObjects. 🔵

6.2.9.2: krilla does not use reference XObjects. 🔵

6.2.9.3: TODOL does this just apply to PostScript objects or all PostScript?? 🔴

6.2.10: All pages that include transparency always have a group key. 🟢

6.2.11.1: -

6.2.11.2: krilla has made sure that the spec is followed in this regard. 🟢

6.2.11.3.1: krilla always uses `Identity-H` as encoding. 🟢

6.2.11.3.2: krilla always writes the `CIDToGidMap` entry. 🟢

6.2.11.3.3: krilla always writes the `WMode` entry for cmaps and never references any other ones. 🟢

6.2.11.4.1: 
- krilla always embeds the used fonts. 6.2.11.4.2:
- krilla does not verify the "legality" of the embedded font. 🔴

6.2.11.4.2: 
- krilla never writes the `CharSet` attribute. 🔵
- krilla never writes the `CIDSet` attribute. 🔵

6.2.11.5: krilla copies the font metrics directly from the font. 🟢

6.2.11.6:
- krilla embeds all fonts as symbolic. 🟢
- krilla does not write the `Encoding` entry for TrueType fonts.
- krilla only writes CIDFonts instead of TrueType fonts directly, so cmap is not needed. 🟢

## 6.3 Annotations


6.3.1: krilla does not support any non-standard annotation types, nor `3D`, `Sound`, `Screen` or `Movie`. 🔵

6.3.2: krilla currently does not set the approproiate annotation flags. 🔴

6.3.3: krilla does not support appearence streams. 🔵

6.3.4: -

## 6.4 Interactive forms

6.4.1: krilla does not support interactive forms. 🔵

6.4.2: krilla does not support interactive forms. 🔵

6.4.3: krilla does not support digital signatures. 🔵

## 6.5 Action

6.5.1:
- krilla does not support the `Launch`, `Sound`, `Movie`, `ResetForm`, `ImportData`, `Hide`, `SetOCGState`, `Rendition`, `Trans`,
  `GoTo3DView` and `JavaScript` actions. 🔵
- krilla does not use the `set-state` and `no-op` actions. 🔵
- krilla does not support named actions. 🔵

6.5.2:
- krilla does not support widget annotations. 🔵
- krilla does not write an AA entry in the document catalog. 🟢
- krilla does not write an AA entry for pages. 🟢

6.5.3: -

## 6.6 Metadata

TODO