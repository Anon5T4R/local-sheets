# Local Sheets app icon

A suite tile: a spreadsheet page on the Local Sheets green gradient, with a column-header band, a
row-number gutter, one selected cell with its fill handle, and the suite's "Taylor" signature along
the bottom. All of it is original geometry written by hand for this repository, deliberately not
modelled on any vendor's spreadsheet logo (no letters, no "X", no overlapping panels).

The signature is outlined paths, not text: Segoe Script does not exist on CI Linux or on users'
machines, so turning it into paths keeps the master deterministic and the SVG self-contained. The
DMG background reuses this file through an `<image>` link, so nothing here is duplicated.

## Palette

| Colour | Hex | Used for |
|---|---|---|
| Ink | `#0b0b0c` | Line work (the DMG background's rules and labels) |
| Paper | `#efe9dc` | The DMG background's field |
| Tile gradient | `#34d399` → `#047857` | The full-bleed field, light at the top-left |
| Header | `#10b981` | The column-header band |
| Header corner | `#047857` | The select-all corner cell |
| Grid and selection | `#a7f3d0` | Grid lines and the selected cell's fill |
| Row gutter | `#d1fae5` | The row-number column |
| Sheet | `#ffffff` → `#ecfdf5` | The page |
| Shadow | `#064e3b` at 35% | The page's drop shadow |

The app colour proposed for the rest of the packaging (metainfo, documentation) is plain
`#1f9d55`, close to the middle of this gradient. Changing it is a packaging decision, not an icon
one: this file is the source of truth for the artwork only.

## Geometry

A 1024 × 1024 viewBox with a full-bleed rounded tile (`rx=224`). Windows and Linux use it as is; the
macOS renders (`.icns`, `localsheets-macos-512.png`) sit on Apple's grid, an 824/1024 body with a
transparent margin, which `packaging/icons.sh` obtains by widening the viewBox.

## Files

| File | Use |
|---|---|
| `localsheets.svg` | Canonical artwork; also `hicolor/scalable` |
| `localsheets-1024.png` | 1024 px render of the full tile |
| `localsheets-macos-512.png` | Runtime Dock icon on macOS (for `apps/localsheets` to embed) |
| `localsheets.icns` | macOS bundle icon (`CFBundleIconFile`) |
| `localsheets.ico` | Windows exe icon, 16–256 px (embedded by `apps/localsheets/build.rs`) |
| `hicolor/<size>/apps/com.local.localsheets.png` | Linux icon theme, 16–512 px; the 256 px one is also the runtime window icon on Windows and Linux |
| `hicolor/scalable/apps/com.local.localsheets.svg` | Linux scalable icon |
| `LICENSE.txt` | MIT OR Apache-2.0 |

The app ID is `com.local.localsheets`. It's used for the hicolor icon names, the Wayland app
ID and `packaging/linux/com.local.localsheets.desktop`.

## Regenerate

Edit `localsheets.svg`, then run `packaging/icons.sh`. It needs `resvg`, packs the `.ico` with
`cargo xtask ico`, and writes the `.icns` with `iconutil` where that exists (macOS). Without
`iconutil` the committed `.icns` stands; the one in this repository was packed from the same macOS
renders as PNG-payload chunks (`icp4`, `icp5`, `ic11`–`ic14`, `ic07`–`ic10`), so it carries exactly
those renders.
