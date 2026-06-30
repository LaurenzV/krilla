# Description
PDF/X is a family of standards for graphic arts and prepress exchange. krilla
supports the following variants, from most restrictive to least:
- PDF/X-1a:2003 (ISO 15930-4) -- CMYK + spot only, no transparency. PDF 1.4.
- PDF/X-3:2003 (ISO 15930-6) -- ICC-based RGB allowed, no transparency. PDF 1.4.
- PDF/X-4 (ISO 15930-7) -- transparency allowed. PDF 1.6.
- PDF/X-4p (ISO 15930-7) -- like X-4, output profile referenced externally. PDF 1.6.
- PDF/X-6 (ISO 15930-9) -- based on PDF 2.0, relaxes several X-4 restrictions.
- PDF/X-6p (ISO 15930-9) -- like X-6, output profile referenced externally. PDF 2.0.

PDF/X-1a is a subset of PDF/X-3, which is a subset of PDF/X-4. PDF/X-6 is not a
strict superset of X-4: being PDF 2.0-based, it relaxes several restrictions.

A PDF/X validator can be combined with a PDF/A and/or PDF/UA one via
`ConfigurationBuilder`. See "Combined PDF/A + PDF/X validators" below.

See `README.md` for the meaning of each color.

## PDF/X-1a:2003 (ISO 15930-4)

See `crates/krilla/examples/pdf_x1a.rs` for a minimal example.

### 6.1 File structure
- krilla writes the version header as 1.4; `pdf-writer` writes the binary marker. 🟢
- krilla always sets the file ID in the trailer. 🟢
- krilla does not support encryption. 🟢
- krilla does not use `LZWDecode`, `JBIG2Decode` or `JPXDecode`. 🟢

### 6.2 Graphics
- krilla does not write PostScript XObjects, and fails export on PostScript functions via `ContainsPostScript`. 🟢
- krilla does not write halftones, the `HTP` key, transfer functions, or overprint settings. 🟢
- krilla only accepts DeviceCMYK, DeviceGray and Separation for page content, via `ContainsRgb`. 🟢
- krilla forbids every annotation at this level via `ContainsAnnotation` (only Link annotations are supported). 🟢
- krilla forbids transparency via `Transparency`. 🟢
- krilla does not write alternate images, OPI dictionaries or reference XObjects. 🟢

### 6.3 Fonts
- krilla always embeds all fonts. 🟢
- krilla forbids `.notdef` glyphs via `ContainsNotDefGlyph`. 🟢

### 6.4 Metadata
- krilla writes `/GTS_PDFXVersion` = `"PDF/X-1a:2003"` to the Info dictionary. 🟢
- krilla requires `/Title` via `NoDocumentTitle`. 🟢
- krilla requires `/CreationDate` and `/ModDate` via `MissingDocumentDate`. 🟢
- krilla writes `/Trapped` as `/True` or `/False` (never `/Unknown`), defaulting to `/False`. 🟢
- krilla writes XMP `pdfxid:GTS_PDFXVersion` and `pdf:Trapped`, consistent with the Info dict. 🟢

### 6.5 Output intent
- krilla writes exactly one output intent with `/S /GTS_PDFX`. 🟢
- krilla writes `OutputConditionIdentifier` ("Custom") with an embedded `DestOutputProfile`, and omits `RegistryName`. 🟢
- krilla requires a CMYK `cmyk_profile` via `MissingCMYKProfile`. 🟢
- The output profile must be an output device (`prtr`) profile via `InvalidOutputProfileDeviceClass`. 🟢
- The output profile must carry the `'CMYK'` colour-space signature via `InvalidOutputProfileColorSpace`. 🟢
- DeviceCMYK content requires a CMYK output intent via `OutputIntentColorSpaceMismatch`. 🟢
- The profile's ICC version must suit the PDF version (1.4 → v2) via `IncompatibleOutputProfileVersion`. 🟢

### 6.6 Actions
- krilla forbids all actions via `ContainsAction`; a Link to an in-document destination carries no action and is permitted. 🟢
- krilla does not write JavaScript, Launch, Sound, Movie or ResetForm actions. 🟢

### 6.7 Embedded files
- krilla forbids embedded files via `EmbeddedFile(Existence)`. 🟢

### 6.8 Page boxes
- krilla requires exactly one of TrimBox/ArtBox per page, via `MissingTrimOrArtBox` and `BothTrimAndArtBox`. 🟢
- Page boxes must nest (MediaBox ⊇ CropBox ⊇ BleedBox ⊇ TrimBox/ArtBox) via `PageBoxNotNested`, with positive area via `DegeneratePageBox`. 🟢
- Under PDF 1.4 no box may exceed 14400 units per side, via `PageBoxTooLarge`. 🟢

### Structural limits
- krilla enforces the PDF 1.4 limits: string (32767), name (127), array (8191), dictionary (4095), float (32767), indirect objects (8388607), q/Q nesting (28). 🟢

### Separation consistency
- A Separation colorant must map to a single tint transform, via `InconsistentSeparationFallback`. 🟢

## PDF/X-3:2003 (ISO 15930-6)

Differences from PDF/X-1a:
- krilla allows ICC-managed RGB (CalRGB, Lab, ICCBased); DeviceRGB stays forbidden via `no_device_cs`. 🟢
- krilla permits annotations outside the print area via `AnnotationInsidePrintArea`, with the `/C` border colour characterized by the output intent via `AnnotationContainsRgb`; each Link gets `AnnotationFlags::PRINT`. 🟢

## PDF/X-4 (ISO 15930-7)

See `crates/krilla/examples/pdf_x4.rs`. Differences from PDF/X-3:
- krilla writes PDF version 1.6 and allows transparency. 🟢
- DeviceGray is valid under a CMYK or grayscale output intent but not an RGB one, flagged via `OutputIntentColorSpaceMismatch`. 🟢
- krilla writes `pdfxid:GTS_PDFXVersion` = `"PDF/X-4"` in XMP and additionally in the Info dictionary. 🟢
- krilla requires a document title (`dc:title`) via `NoDocumentTitle`. 🟢
- The output profile's ICC version may be v2 or v4 up to v4.2, via `IncompatibleOutputProfileVersion`. 🟢
- krilla relaxes the PDF 1.4-only limits (array, dictionary, float) but keeps the rest (string, name, indirect objects, q/Q nesting). 🟢

## PDF/X-4p (ISO 15930-7)

See `crates/krilla/examples/pdf_x4p.rs`. Same as PDF/X-4 except:
- krilla references the profile externally via `DestOutputProfileRef`; `external_output_profile` is required, via `MissingExternalOutputProfile`. 🟢
- `ExternalOutputProfile::rgb/luma/cmyk` reject empty URLs/identifier/info and a colour-space-mismatched profile at construction, returning `ExternalOutputProfileError`. 🟢
- krilla rejects an external profile for non-`p` validators via `ExternalOutputProfileUnsupportedByValidator`. 🟢
- Generic PDF 1.x validators may warn on `DestOutputProfileRef`; this is expected. 🟣

## PDF/X-6 (ISO 15930-9)

See `crates/krilla/examples/pdf_x6p.rs`. krilla writes PDF version 2.0. Same as PDF/X-4 except:
- krilla requires a TrimBox specifically (a coexisting ArtBox is permitted) via `MissingTrimBox`. 🟢
- krilla permits annotations inside the print area (no `AnnotationInsidePrintArea`). 🟢
- krilla permits GoTo/URI actions (no `ContainsAction`). 🟢
- krilla does not require a document title (no `NoDocumentTitle`). 🟢
- krilla writes no Info dictionary; trapping, `GTS_PDFXVersion` and the required `pdfxid:rev` = `2020` go to XMP. 🟢
- The output profile's ICC version may be v4 up to v4.3, via `IncompatibleOutputProfileVersion`. 🟢
- No structural limits apply (PDF 2.0 has no architectural-limits annex). 🟢

## PDF/X-6p (ISO 15930-9)

Same as PDF/X-6 with an external output profile, exactly as PDF/X-4p relates to PDF/X-4.

## Combined PDF/A + PDF/X validators

PDF/A and PDF/X compose on every embedded-profile PDF/X level (X-1a, X-3, X-4, X-6)
where the version ranges overlap. A combined file carries both a `GTS_PDFA1` and a
`GTS_PDFX` output intent sharing one embedded profile, both `pdfaid` and `pdfxid`
XMP metadata, and the union of both standards' restrictions (via `Validators::prohibits`). 🟢

`ConfigurationBuilder::finish` rejects:
- non-overlapping version ranges (e.g. PDF/A-1b + PDF/X-4) via `NoOverlappingValidatorsRange`. 🟢
- PDF/A + an external-profile PDF/X (X-4p, X-6p) via `IncompatibleOutputIntents`: PDF/A forbids the `DestOutputProfileRef` those levels require. 🟢

| Combination | PDF Version |
|---|---|
| PDF/A-1b + PDF/X-1a or X-3 | 1.4 |
| PDF/A-2b / A-3b + PDF/X-4 | 1.6 |
| PDF/A-4 + PDF/X-6 | 2.0 |

## Validation tooling notes

There is no open-source PDF/X conformance validator (veraPDF covers PDF/A and PDF/UA
only). PDF/X conformance in CI is covered by the integration tests in
`tests/src/validate.rs`, the byte-exact snapshot tests, and a structural-marker smoke
test in `ci.yml`.

CI runs the Arlington (ISO 19005-3) model on every snapshot except the three PDF/X ones
that are not PDF 1.7 / PDF/A-3 documents: `validate_pdf_x4p_full_example` (uses
`DestOutputProfileRef`), `validate_pdf_x6_full_example` (PDF 2.0) and
`validate_pdf_x6p_full_example` (both).

For end-to-end validation against an external PDF/X checker (or a commercial preflight
tool), `cargo run --example pdfx_validation_samples -- <dir>` writes one conformant
sample per level (X-1a through X-6p) using the bundled eciCMYK output profile.
