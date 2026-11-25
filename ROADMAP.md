# ROADMAP — Local Sheets

Fork of gridcraft (gridcraft v0.16.1), rebranded for the Local suite. Pure Rust + egui, desktop.
**XLSX is the native format** — the fork invents no format of its own, so files and tools from the
upstream and from Excel stay compatible. How it is built: `docs/architecture.md`; decisions and the
why: `docs/decisions.md`.

**Stage: alpha.** It opens, edits and saves real-world spreadsheets; what is missing below is the
path to "ready for real work".

## What is missing (by area, in no order of difficulty)

- **XLSX fidelity.** The round-trip is tested with our own files; a corpus of third-party files and
  a real Excel are still to go through it (PivotTables, charts, conditional formatting, validation,
  dynamic arrays, tables): a repair prompt or silent loss is what costs the most. Unmodeled content
  (slicers, timelines, the data model, `extLst`) passes intact through the pass-through, but has
  never been checked in Excel.
- **Large spreadsheets.** No streaming read, and recalculation on a single thread.
- **Charts.** Shallow model: axis scale, label options, error bars and per-point formatting.
- **Non-Latin text.** No shaping (Arabic, Devanagari, Thai) and no bidi; the PDF export uses only
  the 14 standard fonts.
- **`.xls` and protected files.** `.xls` (BIFF) is only partly read and never written; encrypted
  `.xlsx` opens with the password, but is saved without it.
- **PivotTables.** Slicers, PivotCharts and calculated fields exist; calculated items do not. PivotTables
  were never checked in Excel.
- **Keyboard.** Ribbon keytips are still partial, with fewer shortcuts than Excel.
- **Localization.** The interface offers 19 languages, but strings are missing in all of them —
  from 78 in Japanese to more than 800 in Turkish; `cargo xtask i18n-report` lists what is missing
  per language.

## Formats

| Format | Read | Write |
| --- | --- | --- |
| `.xlsx` (native) | yes, with styles, themes, formulas, tables, conditional formatting, validation, comments, links, charts, images, sparklines, pivots and print setup | yes |
| `.ods` | yes: data, numbers, text, booleans, dates, repeated cells/rows and merges (formulas become the last saved result) | no |
| `.xlsb` | yes: names, visibility, numbers, text, booleans, errors and merges (formulas become the last saved result) | no |
| `.xls` (BIFF8) | partial: values, formulas, number formats, merges, sizes, names and cell formatting; exact theme, charts, images, comments, conditional formatting and validation no | no |
| `.csv`/`.tsv`/`.txt` | yes (including `csvAsText`) | yes |
| `.xlsx` with password | yes (Agile and Standard) | without password |

Known import limits: `docs/ods-import.md`, `docs/xlsb-import.md`; pictures in cells:
`docs/cell-pictures.md`.

## References

`docs/architecture.md` · `docs/decisions.md` · `docs/cli.md` · `docs/control-protocol.md` ·
`docs/mcp.md` · `docs/macros.md` · `docs/releasing.md` · `docs/UPSTREAM-SYNC.md`
