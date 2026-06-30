`krilla-generic-cmyk-v2.icc` is a compact synthetic CMYK output profile used by
krilla's examples and tests.

It is intentionally small so that snapshot PDFs and example outputs stay easy to
inspect and share. It is a CMYK ICC v2.4 `prtr` profile with Lab PCS and linked
intent tags, generated with LittleCMS from compact synthetic `A2B`/`B2A` CLUTs.

It is not a press characterization profile and should not be used as a real
production printing condition. For production PDF/X output, callers should
provide the actual press/output ICC profile that matches their workflow.

## Licence and provenance

`krilla-generic-cmyk-v2.icc` is an original work: it was generated from
synthetic CLUT data using Little CMS (lcms2) and is not measured from any
device, nor derived from any third-party, vendor, or press characterisation
profile.

It is licensed under the same terms as the krilla crate, MIT OR Apache-2.0
(see `LICENSE_MIT` and `LICENSE_APACHE` at the repository root).

Little CMS (<https://littlecms.com>, MIT licence) was used only as an offline
generation tool; its licence covers the library, not the profile data produced
with it, and lcms2 is neither bundled nor redistributed by krilla, so no lcms2
licence obligation attaches to this file.
