# Attribution

Every asset Local Sheets includes, bundles, embeds at build time or shows in its documentation, with
its author, source and licence. The clean-room and asset policy is in [AGENTS.md](AGENTS.md);
`cargo xtask assets` fails if a file under `assets/` or `docs/images/` is missing from
this table.

Local Sheets contains **no Microsoft, Adobe, Avid or Autodesk iconography, images, fonts, themes,
templates, table styles or sample files**. Its themes, cell and table styles, chart palettes, sample
workbooks and UI icons are original work of the Local Sheets contributors (drawn or generated in
code), carried over from the GridCraft upstream this repository forks from. Excel feature *names*
(menu labels, function names) are not assets, and no Microsoft screenshot is committed.

The rows below are the work of this repository and its upstream; "Local Sheets contributors" is the
fork's own attribution for assets the rebrand keeps as-is.

## In this repository

| Asset | Author | Source | Licence | Used for |
|---|---|---|---|---|
| `assets/app-icon/localsheets.svg` | Local Sheets contributors | master SVG of the suite icon, rendered by `packaging/icons.sh` (resvg) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | App icon source (green tile, ledger page) |
| `assets/app-icon/localsheets-1024.png` | Local Sheets contributors | rendered from `assets/app-icon/localsheets.svg` by `packaging/icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Master bitmap of the app icon |
| `assets/app-icon/localsheets-macos-512.png` | Local Sheets contributors | rendered from `assets/app-icon/localsheets.svg` by `packaging/icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | macOS icon grid render |
| `assets/app-icon/localsheets.icns` | Local Sheets contributors | generated from the PNG renders by `packaging/icons.sh` (iconutil on macOS) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | macOS and BSD bundle icon |
| `assets/app-icon/localsheets.ico` | Local Sheets contributors | packed from the PNG renders by `cargo xtask ico` (see `packaging/icons.sh`) | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Windows executable icon |
| `assets/app-icon/hicolor/16x16/apps/com.local.localsheets.png` | Local Sheets contributors | rendered from `assets/app-icon/localsheets.svg` by `packaging/icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Linux hicolor render |
| `assets/app-icon/hicolor/24x24/apps/com.local.localsheets.png` | Local Sheets contributors | rendered from `assets/app-icon/localsheets.svg` by `packaging/icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Linux hicolor render |
| `assets/app-icon/hicolor/32x32/apps/com.local.localsheets.png` | Local Sheets contributors | rendered from `assets/app-icon/localsheets.svg` by `packaging/icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Linux hicolor render |
| `assets/app-icon/hicolor/48x48/apps/com.local.localsheets.png` | Local Sheets contributors | rendered from `assets/app-icon/localsheets.svg` by `packaging/icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Linux hicolor render |
| `assets/app-icon/hicolor/64x64/apps/com.local.localsheets.png` | Local Sheets contributors | rendered from `assets/app-icon/localsheets.svg` by `packaging/icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Linux hicolor render |
| `assets/app-icon/hicolor/128x128/apps/com.local.localsheets.png` | Local Sheets contributors | rendered from `assets/app-icon/localsheets.svg` by `packaging/icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Linux hicolor render |
| `assets/app-icon/hicolor/256x256/apps/com.local.localsheets.png` | Local Sheets contributors | rendered from `assets/app-icon/localsheets.svg` by `packaging/icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Linux hicolor render; README icon |
| `assets/app-icon/hicolor/512x512/apps/com.local.localsheets.png` | Local Sheets contributors | rendered from `assets/app-icon/localsheets.svg` by `packaging/icons.sh` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Linux hicolor render |
| `assets/app-icon/hicolor/scalable/apps/com.local.localsheets.svg` | Local Sheets contributors | copy of the master SVG `assets/app-icon/localsheets.svg` | MIT OR Apache-2.0 (`assets/app-icon/LICENSE.txt`) | Scalable app icon for Linux desktops (hicolor theme) |
| `assets/app-icon/README.md` | Local Sheets contributors | this repository | MIT OR Apache-2.0 | How the icon files are produced |
| `assets/app-icon/LICENSE.txt` | (licence text) | this repository | — | Licence of the app icon files |
| `packaging/macos/dmg/background.svg` | @XusBadia | original hand-written SVG, derived from the app icon `assets/app-icon/localsheets.svg` (referenced via `<image>`, not copied); text is outlined glyph paths | MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`) | Finder window background of the macOS DMG; `packaging/macos/dmg/generate.py` renders it to `background.tiff` |
| `packaging/macos/dmg/background.tiff` | @XusBadia | rendered from `packaging/macos/dmg/background.svg` by `packaging/macos/dmg/generate.py` | MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`) | 1x + 2x HiDPI TIFF for the macOS DMG window; includes the app icon (see its row) and LittleCMS's built-in sRGB profile ("No copyright, use freely") |
| `docs/images/hero-sales.png` | GridCraft contributors (upstream) | screenshot of the upstream GridCraft application, carried over from the fork base (v0.16.1); not shown by this README | MIT OR Apache-2.0 | Historical documentation of the fork base |
| `docs/images/budget.png` | GridCraft contributors (upstream) | screenshot of the upstream GridCraft application, carried over from the fork base (v0.16.1); not shown by this README | MIT OR Apache-2.0 | Historical documentation of the fork base |
| `docs/images/grades-formulas.png` | GridCraft contributors (upstream) | screenshot of the upstream GridCraft application, carried over from the fork base (v0.16.1); not shown by this README | MIT OR Apache-2.0 | Historical documentation of the fork base |
| `docs/images/dark-insert.png` | GridCraft contributors (upstream) | screenshot of the upstream GridCraft application, carried over from the fork base (v0.16.1); not shown by this README | MIT OR Apache-2.0 | Historical documentation of the fork base |
| `docs/images/pivot.png` | GridCraft contributors (upstream) | screenshot of the upstream GridCraft application, carried over from the fork base (v0.16.1); not shown by this README | MIT OR Apache-2.0 | Historical documentation of the fork base |
| `docs/images/format-chart.png` | GridCraft contributors (upstream) | screenshot of the upstream GridCraft application, carried over from the fork base (v0.16.1); not shown by this README | MIT OR Apache-2.0 | Historical documentation of the fork base |
| `docs/images/page-break.png` | GridCraft contributors (upstream) | screenshot of the upstream GridCraft application, carried over from the fork base (v0.16.1); not shown by this README | MIT OR Apache-2.0 | Historical documentation of the fork base |

## Bundled through dependencies

| Asset | Author | Source | Licence | Used for |
|---|---|---|---|---|
| Ubuntu Light (`Ubuntu-Light.ttf`, via the `epaint_default_fonts` crate 0.37) | Dalton Maag Ltd for Canonical | https://crates.io/crates/epaint_default_fonts | Ubuntu Font Licence 1.0 (full upstream text in `NOTICE`) | Last-resort text face when no other font is available; UI fallback |
| Hack Regular (`Hack-Regular.ttf`, via the `epaint_default_fonts` crate) | Source Foundry Authors (Hack; DejaVu and Bitstream Vera foundations) | https://crates.io/crates/epaint_default_fonts, [Hack](https://github.com/source-foundry/Hack) | MIT + Bitstream Vera License (full upstream texts in `NOTICE`) | egui's bundled monospace face; last-resort symbol fallback |
| Noto Emoji (`NotoEmoji-Regular.ttf`, via egui's `monochrome_emoji_fonts` feature) | The Noto Project Authors | https://crates.io/crates/epaint_default_fonts, [Noto Emoji](https://fonts.google.com/noto/specimen/Noto+Emoji) | SIL Open Font License 1.1 (full upstream text in `NOTICE`) | Monochrome emoji fallback |
| `epaint_default_fonts::EGUI_ICONS`, `epaint_default_fonts::EMOJI_ICON` (dependency font inputs) | John Slegers | [egui 6d518fc7](https://github.com/storytold/egui/tree/6d518fc799681571963842f95b55a6b68431b15a/crates/epaint_default_fonts/fonts), [emoji-icon-font](https://github.com/jslegers/emoji-icon-font) | MIT (Copyright (c) 2014 John Slegers; full upstream text in `NOTICE`) | egui-icons.ttf is the five-glyph subset; emoji-icon-font.ttf is the full monochrome fallback. Dependency assets unmodified here; no font file copied into this repository. |

Dependency fonts above are embedded by `epaint_default_fonts`; no font file is copied into this
repository.

## Embedded at build time from craft-fonts (`CRAFT_FONTS_DIR`)

Builds that set `CRAFT_FONTS_DIR` (every official release) embed the fonts listed in
[craft-fonts](https://github.com/storytold/craft-fonts) `fonts/manifest.txt` (per-file authors,
sources, SHA-256 and licences in its
[ATTRIBUTION.md](https://github.com/storytold/craft-fonts/blob/main/ATTRIBUTION.md)), minus the
families barred by the repository asset policy in
[`crates/ui-egui/src/craft_fonts_policy.rs`](crates/ui-egui/src/craft_fonts_policy.rs). No font file
is copied into this repository.

| Families | Licence | Notes |
|---|---|---|
| BIZ UDPGothic Regular, BIZ UDPGothic Bold | OFL-1.1 | Japanese text (required face; in the web bundle) |
| NanumGothic Regular | OFL-1.1 | Korean text (required face; in the web bundle) |
| Noto Sans Symbols Regular | OFL-1.1 | Interface symbol fallback (required face; in the web bundle) |
| Noto Sans Symbols 2 Regular | OFL-1.1 | Interface symbol fallback (required face; in the web bundle) |
| The other Latin and script faces listed in craft-fonts' `fonts/manifest.txt` (for example Inter, Carlito, Caladea, Liberation, DejaVu, Roboto, Noto Sans, Noto Sans Arabic, Noto Sans Hebrew, Noto Sans Thai, Noto Sans Georgian, Shippori Mincho, BIZ UDMincho) | OFL-1.1 (per-file in craft-fonts) | Document and interface text; embedded natively only |

The Adobe, Source Han/Source Sans/Source Serif and Noto Sans/Serif CJK families in craft-fonts are
never embedded: the asset policy bars their prefixes. The web (wasm32) build embeds only the four
required faces above.

### Optional native Chinese fallback

Native builds also read craft-fonts' `fonts/simplified-chinese-manifest.txt` when present. This
opt-in never enters the wasm font allowlist, and is only required when the build sets
`CRAFT_FONTS_REQUIRED=1`.

| Family | Author | Source | Licence | Used for |
|---|---|---|---|---|
| LXGW WenKai Lite Regular | LXGW; original Klee Project Authors / Fontworks | https://github.com/lxgw/LxgwWenKai-Lite | OFL-1.1, with the LXGW Reserved Font Names | Native Simplified Chinese glyph fallback after the interface faces; font bytes and the full licence stay in craft-fonts |

## Metrics and tables typed in from public sources

These are numbers transcribed from published tables: no font or data file is copied into this
repository, and none is redistributed.

| Asset | Author | Source | Licence | Used for |
|---|---|---|---|---|
| Core 14 AFM metric numbers in `crates/pdf/src/symbol.rs` (Symbol: character codes and advance widths) and `crates/pdf/src/metrics.rs` (Helvetica, Times and Courier advance widths) | Adobe Systems Incorporated | Adobe's Core 14 AFM font metrics files (`Symbol.afm` 001.008 and the Helvetica, Times and Courier AFMs) | Adobe's Core 14 metrics notice: the files may be used, copied and distributed for any purpose and without charge, provided the notice is kept (quoted in `crates/pdf/src/metrics.rs`) | PDF export embeds no font: Symbol and the Times faces are base-14 fonts every PDF reader supplies, and text outside WinAnsiEncoding is set in a predefined, non-embedded Adobe-GB1 CID font |
| Unicode values of the Symbol encoding in `crates/pdf/src/symbol.rs` (the code-to-character map; numbers only) | Unicode, Inc. | `symbol.txt`, "Adobe Symbol Encoding to Unicode", table version 1.0 (2011-07-12), at unicode.org/Public/MAPPINGS/VENDORS/ADOBE/ | Unicode, Inc.'s notice in the file: "Unicode, Inc. hereby grants the right to freely use the information supplied in this file in the creation of products supporting the Unicode Standard, and to make copies of this file in any form for internal or external distribution as long as this notice remains attached." The file is provided as-is, without warranties | Separate from Adobe's Core 14 notice (previous row): this table supplies the Unicode values |

Generated-in-code art is original and has no file to list: the UI icon set
(`crates/ui-egui/src/icons.rs`), themes (`crates/engine/src/cmd/view.rs`), cell styles
(`crates/engine/src/cmd/format.rs`), table styles (`crates/engine/src/tables.rs`), chart palettes
and the sample workbooks (`crates/engine/src/sample.rs`).
