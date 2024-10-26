# Description
PDF-A/1 requires PDF a version <= 1.4 and defines three conformance levels, 
in the following order from less strict to more strict:
- Level B
- Level A

Level B is a subset of level A.

See `README.md` for the meaning of each color.

# Level B

## 6.1 File structure

6.1.2: `pdf-writer` always write the file header as described in the spec. Even if the
user enables `ascii_compatible`, we still write a binary header marker. 🟢

6.1.3: 
- We always set the file ID. 🟢
- We do not support encryption. 🔵
- We do not support linearization. 🔵

6.1.4: `pdf-writer` always write the xref section as described in the spec. 🟢

6.1.5: krilla ensures that the document information dictionary is consistent with
the XMP metadata.

6.1.6: `pdf-writer` always writes hex strings 
with an even number of characters and without whitespaces. 🟢

6.1.7: 
- `pdf-writer` always write streams as described in the spec. 🟢
- krilla does never write streams referencing external files. 🔵
- krilla does never use `LZWDecode` or `Crypt`. 🔵
- krilla never writes F, FFilter or FDecodeParams in streams. 🔵

6.1.8: `pdf-writer` always writes indirect objects as described. 🟢

6.1.9: krilla doesn't support linearization. 🔵

6.1.10: krilla doesn't support `LZWDecode`. 🔵

6.1.11: krilla doesn't support embedded files. 🔵

6.1.12:
- `pdf-writer` uses i32 for integers. 🟢
- krilla does not support the checking for these real numbers. 🔴 
- krilla always uses the `new_str` and `new_text_str` methods of the SerializerContext to create them, 
  which returns a validation error in case it's too long. 🔵
- krilla trims the names of fonts, and all other names cannot be longer than 127. 🔵
- krilla does not check the maximum length of arrays. 🔴
- krilla does not check the maximum entries of a dictionary. 🔴
- krilla fails export if more than 8388607 indirect objects exist. 🟢
- krilla fails export if a higher nesting-level than 28 exists. 🔵
- krilla does not use the DeviceN color space. 🟢
- krilla only uses u16 for CIDs. 🟢

6.1.13: krilla does not support optional content. 🔵

## 6.2 Graphics

6.2.2:
- krilla always writes a valid output intent. 🟢
- krilla always only writes one output intent. 🟢

6.2.3.1: krilla overrides the `no_device_cs` property if PDF/A is selected, and
in case CMYK is used but no profile was provided, export fails.

6.2.3.2: 
- sRGB/sGrey ICC profiles conform to ICC v2 specification. 🟢

6.2.4.3: 
- krilla always uses sRGB for RGB, and in addition also embeds an sRGB output intent. 🟢
- krilla always uses sGrey (except for encoding the alpha channel in images, where DeviceGray
  is required), and in addition always embeds an output intent. 🟢
- krilla always uses an CMYK ICC profile, and always sets CMYK as the output intent. 🟢
  It fails export if no CMYK ICC profile was provided. 🟢

6.2.3.4: krilla does not support DeviceN/Separation color spaces. 🔵

6.2.4: krilla does not use the `Alternates`/`Interpolate`/`Intent` keys for images. 🔵

6.2.5: krilla does not use the `OPI`/`Subtype2`/`PS` keys for XObjects. 🔵

6.2.6: krilla does not use reference XObjects. 🔵

6.2.7: The spec only talks about PostScript XObjects, which we don't really use.
We only use PostScript functions. In any case, to be on the safe side, krilla fails exports
when a PostScript function is used. 🟢

6.2.8: krilla does not use the transfer functions, halftones, TR/HTP/RI/FL keys. 🔵

6.2.9: krilla does never define a rendering intent. 🔵

6.2.10: krilla only uses operators defined in the reference and never uses BX/EX. 🔵

# 6.3 Fonts

6.3.2: krilla follows the specification when defining fonts.

6.3.3.1: krilla always writes the same Registry/Ordering for fonts.

6.3.3.2: krilla always includes a `CIDToGIDMap` for Type2 CID fonts.

6.3.3.3: krilla always writes the `WMode` entry for cmaps and never references any other ones.

6.3.4: krilla embeds all fonts that are used.

6.3.5:
- krilla does currently not write the `CIDSet` attribute.
- krilla does currently not write the `CharSet` attribute.

6.3.6: krilla always adds the `Widths` entry and makes them consistent to the font program.

6.3.7: krilla only embeds CID fonts.


# 6.4 Transparency

- krilla does currently not forbid SMasks.
- krilla does currently not check for the S key.
- krilla does currently not forbid CA/ca/BM.

## 6.5 Annotations

6.5.2: krilla does not support any non-standard annotation types, nor `FileAttachment`, `Sound` or `Movie`. 🔵

6.5.3: 
- krilla does not use the CA key of annotation dictionaries.
- krilla always sets the `F` flag for annotations. 🟢
- krilla does not support text annotations.
- krilla does not set the C key of annotations.
- krilla does never write an appearence dictionary.


## 6.6 Action

6.6.1:
- krilla does not support the `Launch`, `Sound`, `Movie`, `ResetForm`, `ImportData`, `JavaScript` actions. 🔵
- krilla does not use the `set-state` and `no-op` actions. 🔵
- krilla does not support named actions. 🔵
- krilla does not support interactive forms. 🔵

6.6.2:
- krilla does not support widget annotations. 🔵
- krilla does not write an AA entry in the document catalog. 🟢
- krilla does not write an AA entry for pages. 🟢

6.6.3: -

## 6.6 Metadata

6.7.2: 
- krilla overrides the `xmp_metadata` attribute for PDF/A exports so that it's always contained. 🟢
- `xmp-writer` always creates conforming XMP streams. 🟢
- We never apply a filter to the metadata stream dictioanry.

6.7.3: 
- krilla ensures that XMP metadata and document info dictionary are consistent. 🟢
- Authors are encoded as a length-1 text array. 🟢
- krilla ensures consistency between PDF dates and XMP dates.

6.7.4: Normalization is currently not checked.

6.7.5: We never include the `bytes` or `encoding` attributes. 🟢

6.7.6: krilla writes pdfaid:conformance and pdfaid:part as specified. 🟢

6.7.7: krilla does not write file provenance information.

6.7.8: krilla does not write the extension schemas.

6.7.9: `xmp-writer` always produces valid XMP packets.

6.7.10: krilla does not write font metadata.

6.7.11: krilla writes the conformance level identification.

# Level A

6.3.8: krilla always embeds a unicode character map. 🟢

# 6.8 Logical Structure

6.8.2.1: General conformance to tagged PDF cannot be checked, but is documented. 🟣

6.8.2.2: krilla always writes the mark info dictionary for this export mode. 🟢

6.8.3.1: The need to specify artifacts is documented. 🟣

6.8.3.2: The need to specify word boundaries is documented. 🟣

6.8.3.3: The need to specify the structure hierarchy is documented. 🟣

6.8.3.4: krilla maps all non-standard structure types. 🟢

6.8.4:
- krilla requires the user to set the document language in that export mode. 🟢
- krilla forces the user to specify the language on each span. 🟢
- The need for correctness of language tags is documented. 🟣

6.8.5: The need to document images and formulas with alt text is documented. 🟣

6.8.6: krilla currently does not support any non-textual annotations. 🔵


NOTE: krilla will discard any alt text / abbreviations specifications in PDF/A1, because it's based on PDF 1.4,
and we define them inline as properties to spans, which
is actually only available for PDF >= 1.5. Maybe we
can fix this in the future.
6.8.7: The requirement to specify actual text is documented. 🟣

6.8.8: The requirement to specify the expansions of abbreviations is documented. 🟣

# 6.9 Interactive Forms
krilla does not support interactive forms. 🔵


