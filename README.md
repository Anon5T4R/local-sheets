<p align="center">
  <img src="assets/app-icon/hicolor/256x256/apps/com.local.localsheets.png" width="128" height="128" alt="Local Sheets icon: a spreadsheet page on a green tile">
</p>

<h1 align="center">Local Sheets</h1>

<p align="center">
  <b>Spreadsheets and data; an open-source, modern spreadsheet application, built in pure Rust.</b>
</p>

<p align="center">
  Build workbooks with formulas, tables, charts, pivot tables and conditional formatting,
  open and save .xlsx workbooks, and print or export them to PDF. Native on Windows, Linux,
  macOS and BSD, and in the browser. Every button is also a command that the CLI and AI agents
  (through MCP) can drive.
</p>


<br>



<p align="center">
  <a href="#what-works-today">What works</a> ·
  <a href="#getting-started">Getting started</a> ·
  <a href="#agents-cli-and-mcp">Agents, CLI and MCP</a> ·
  <a href="#roadmap">Roadmap</a> ·
  <a href="#downloads">Downloads</a> ·
  <a href="#the-local-suite">The Local suite</a> ·
  <a href="#license-and-credits">License</a>
</p>

## What works today

Local Sheets is in alpha (see the [roadmap](ROADMAP.md)). Today it can:

- **Write and edit**: ribbon, formula bar, Name Box, sheet tabs, keyboard shortcuts, fill handle,
  point-mode formula entry with coloured references, Format Cells, Paste Special, Find and Replace,
  Flash Fill, drag-and-drop and in-cell editing.
- **Calculate**: a dependency-graph engine with dynamic arrays and spilling, 500+ worksheet
  functions plus LET, LAMBDA, MAP, REDUCE, SCAN, BYROW, BYCOL and MAKEARRAY, structured table
  references, iterative calculation and a comprehensive number-format language.
- **Format**: cell styles, themes, borders, conditional formatting (rules, data bars, colour
  scales, icon sets), tables with styles, and per-cell pictures.
- **Analyse**: sort and AutoFilter, data validation, outlining and subtotals, pivot tables with
  field lists and show-values-as, Goal Seek, Solver and the Data Analysis tools, plus what-if
  scenarios.
- **Chart**: seventeen chart types with a Chart Design tab and a Format pane; selecting a chart
  outlines its source data.
- **Exchange files**: .xlsx is the native format, with styles, themes, formulas, tables,
  conditional formats, validation, comments, hyperlinks, charts, pictures, sparklines, pivot
  tables and print settings round-tripping. CSV and TSV too; legacy .xls, XLSB and ODS worksheet
  data import their values; password-protected workbooks are decrypted.
- **Print and export**: page setup, Page Break Preview, printing to the system print dialog and a
  self-contained PDF export.
- **Automate**: every ribbon button, menu item, dialog, keystroke and mouse gesture is a command
  with undo, all scriptable from the CLI, a JSON control channel and an MCP server for AI agents.
- **Run everywhere**: one Rust codebase for desktop and the web; no subscription, no licence
  server, no telemetry.

The honest picture, area by area, is in [ROADMAP.md](ROADMAP.md),
[docs/target-app-parity.md](docs/target-app-parity.md),
[docs/gaps.md](docs/gaps.md) and the generated
[docs/parity-checklist.md](docs/parity-checklist.md).

## Getting started

```sh
git clone https://github.com/Anon5T4R/local-sheets
cd local-sheets
cargo build --locked
cargo run --release -p localsheets -- --sample sales        # the desktop app with the sample workbook
cargo run --release -p localsheets -- book.xlsx             # open a workbook
cargo run --release -p localsheets -- --sample sales --control 7979   # + JSON control channel
cargo xtask ci                                              # fmt, clippy, tests, assets, layering, wasm
```

The built-in samples are `sales` (quarterly sales dashboard), `budget` (household budget) and
`grades` (gradebook with formulas). Under File › Options › Interface language the interface also
speaks Spanish, with Spanish formula names, decimal commas and semicolon arguments; workbooks,
clipboard exchange and programmatic APIs keep canonical English syntax. See
[docs/formula-input.md](docs/formula-input.md).

Fonts come from [craft-fonts](https://github.com/storytold/craft-fonts): clone it next to this repo
(`../craft-fonts`) and local builds pick it up automatically; without it Local Sheets uses your
system fonts.

Printing goes through the shared `craft-print` crate from `libs/craft-libs` (the Windows spooler, or
CUPS `lp` elsewhere); PDF export uses the self-contained writer in
[`crates/pdf`](crates/pdf/README.md), which has no dependencies and embeds no font files.

### Web

```sh
cd apps/localsheets-web && trunk build --release     # → dist/web (serve it with any static server)
cd apps/localsheets-web && trunk serve --release     # http://127.0.0.1:8771  (?sample=sales opens a sample)
```

You need [trunk](https://trunk-rs.github.io/trunk/) and the `wasm32-unknown-unknown` target. Open
uses the browser's file picker (dropping files works too) and Save downloads the workbook.

### Logs

The desktop app writes its `log` records to standard error and to `logs/localsheets.log` in the
settings directory, beside its other saved state (Linux `~/.config/localsheets/logs/`, or
`$XDG_CONFIG_HOME/localsheets/logs/`; macOS `~/Library/Application Support/Local Sheets/logs/`;
Windows `%APPDATA%\Local Sheets\logs\`). A start from a desktop menu has no terminal, so this file
is what to attach to a bug report: import problems, recovery saves and graphics-backend fallbacks
land there. Each launch moves the previous log to `localsheets.1.log` (and that one to
`localsheets.2.log`), so the log of a run that crashed survives the next start. The file stops
growing at 16 MiB. Runs with `LOCALSHEETS_NO_PREFS` set (no saved settings) log to standard error
only.

| Variable | Effect |
|---|---|
| `RUST_LOG` | Log levels for standard error and the log file. Default: `info` for Local Sheets' own crates, `warn` for everything else. env_logger-style directives replace that, e.g. `RUST_LOG=debug`, `RUST_LOG=warn,localsheets_engine=debug` or `RUST_LOG=info,wgpu_core=warn`; a directive ending in `*` covers every target starting with it (`localsheets*=debug`). |

The web app logs to the browser console instead.

## Agents, CLI and MCP

Every action in Local Sheets is a command with a stable id, so people, scripts and AI agents use the
same verbs:

```sh
localsheets-cli commands --search chart                     # every command and its params
localsheets-cli eval '=XLOOKUP("b",{"a","b"},{1,2})'        # → 2
localsheets-cli run --sample budget --cmd 'home.bold={"range":"B4:F4"}' --out budget.xlsx
localsheets-cli cat budget.xlsx --range B4:F15              # aligned text table
localsheets-cli mcp                                         # MCP server over stdio (headless)
localsheets-cli mcp --connect 7979                          # MCP bridged to a running window
```

The MCP server offers tools such as `read_range`, `write_range`, `set_cell`, `evaluate_formula`,
`format_range`, `insert_chart`, `create_table`, `sort_range`, `filter` and `execute_command`
(which reaches every one of Local Sheets' commands). The desktop app's control channel adds real
pointer and keyboard input, dialogs and screenshots. See [`docs/mcp.md`](docs/mcp.md),
[`docs/cli.md`](docs/cli.md) and [`docs/control-protocol.md`](docs/control-protocol.md).

## Architecture

| Layer | Crate | Job |
|---|---|---|
| L0 | `localsheets-core`, `localsheets-numfmt`, `localsheets-pdf` | addresses, values, errors, dates, input parsing; number-format codes; PDF 1.7 writer |
| L1 | `localsheets-formula`, `localsheets-functions`, `localsheets-math` | lexer, parser, printer, reference adjustment; worksheet functions; equation layout |
| L2 | `localsheets-model`, `localsheets-calc`, `localsheets-xlsx` | workbook, sheets, copy-on-write cells, styles, tables, charts; dependency graph and evaluator; XLSX, CSV, XLSB and ODS data import |
| L3 | `localsheets-chart` | toolkit-free chart layout and rendering |
| L5 | `localsheets-engine` | session, commands, history, clipboard, fill, sort, filter, file I/O |
| L6 | `localsheets-ui-egui`, `localsheets-mcp` | spreadsheet UI and control channel; MCP server |
| L7 | apps | `localsheets` (desktop), `localsheets-cli`, `localsheets-web` |

`cargo xtask ci` runs formatting, clippy, the tests, the asset-attribution check, the layering
check, the wasm build and the generated parity and keytip checks. Contributor and agent
instructions: [AGENTS.md](AGENTS.md).

## Roadmap

Local Sheets is in **alpha**. It covers 89% of the standard ribbon commands in our catalog
([`docs/parity-checklist.md`](docs/parity-checklist.md)) and 513 standard worksheet functions
([`docs/function-parity.md`](docs/function-parity.md)); this is a parity inventory, not a count of
working implementations. Weighted by what spreadsheet users rely on, we estimate it is about 50%
ready for real work. Next up: XLSX fidelity on real-world files, very large workbooks and
multi-threaded recalc, chart depth, text in every script, and .xls import. Stage, numbers and
estimates are in [ROADMAP.md](ROADMAP.md); the ranked work list is
[`docs/gaps.md`](docs/gaps.md).

## Downloads

**Download Local Sheets** from GitHub: the [latest release](https://github.com/Anon5T4R/local-sheets/releases/latest)
has every build listed below, and [all releases](https://github.com/Anon5T4R/local-sheets/releases)
has earlier versions and their notes. `<ver>` in the file names is the version number, and
`SHA256SUMS.txt` lists a checksum for every file.

### Windows

| Build | Installer (all users) | Installer (just me) | Portable |
|---|---|---|---|
| x64 (64-bit Intel/AMD) | `localsheets-<ver>-windows-x64.msi` | `localsheets-<ver>-windows-user-x64.msi` | `localsheets-<ver>-windows-x64-portable.zip` |
| arm64 (Snapdragon and other ARM PCs) | `localsheets-<ver>-windows-arm64.msi` | `localsheets-<ver>-windows-user-arm64.msi` | `localsheets-<ver>-windows-arm64-portable.zip` |
| x86 (32-bit) | `localsheets-<ver>-windows-x86.msi` | `localsheets-<ver>-windows-user-x86.msi` | `localsheets-<ver>-windows-x86-portable.zip` |

The all-users installer puts Local Sheets in `Program Files\Local Sheets` and needs administrator
rights. The just-me installer needs none: it installs into `%LOCALAPPDATA%\Programs\Local Sheets`
for your account only. Choose one: installing both gives two Local Sheets entries in Settings › Apps
and two in the Start menu. The portable ZIP carries `localsheets.exe` and `localsheets-cli.exe` and
needs no installation. Installers and executables are code-signed.

**If the app doesn't open on Windows:** the desktop app initializes only DirectX 12 by default.
Letting wgpu also create an OpenGL instance can crash some graphics drivers (AMD's
`atio6axx.dll`) before the window appears, so the app would flash in Task Manager and quit.
`WGPU_BACKEND` overrides the default for troubleshooting (for example `dx12` or `vulkan`). In
PowerShell, from the folder containing the executable:

```powershell
$env:WGPU_BACKEND = "vulkan"
& .\localsheets.exe
Remove-Item Env:WGPU_BACKEND                     # restore the default for later launches
```

An explicit `gl` override can bring the driver crash back on affected systems. The macOS, Linux and
web backend defaults are unchanged.

DirectX 12 shaders are compiled with FXC, which is part of Windows, so no `dxcompiler.dll` or
`dxil.dll` is needed. `$env:WGPU_DX12_COMPILER = "dxc"` (or `"auto"`) selects wgpu's DXC handling
again for troubleshooting.

### Linux

| Format | File | Notes |
|---|---|---|
| AppImage | `localsheets-<ver>-linux-x86_64.AppImage` | Runs anywhere; unpack and run |
| Debian/Ubuntu | `localsheets-<ver>-linux-x86_64.deb` | |
| Fedora/RHEL/openSUSE | `localsheets-<ver>-linux-x86_64.rpm` | |
| Tarball | `localsheets-<ver>-linux-x86_64.tar.gz` | Unpack anywhere |

**Runtime libraries.** The window-system, GPU and file-dialog libraries Local Sheets needs are
opened when the app starts (`dlopen` through `x11-dl`, `xkbcommon-dl`, `ash`, `glutin` and `rfd`)
instead of being linked in, so `ldd` shows none of them: an install that satisfies the linked
dependencies can still fail at start-up with a loader error and no hint of what to install. The
`.deb` and `.rpm` declare the set in `packaging/linux/nfpm.yaml`; a tarball install or a musl system
has to provide it itself:

```sh
apk add libx11 libxcursor libxrandr libxi libxkbcommon xkeyboard-config \
        wayland-libs-client \
        vulkan-loader mesa-vulkan-swrast \
        mesa-gl mesa-egl mesa-dri-gallium \
        xdg-desktop-portal
```

Alpine Linux `v3.22` names above; Void's musl variant uses the same `/lib/ld-musl-x86_64.so.1`
loader and the same libraries as `libX11 libXcursor libXrandr libXi libxkbcommon xkeyboard-config
wayland vulkan-loader mesa xdg-desktop-portal`. There is no musl build among the release artifacts
yet — this is what a build from source on musl needs at run time.

- `mesa-vulkan-swrast` is the software Vulkan driver: it runs anywhere, VMs and containers
  included. On a real GPU install your vendor's driver instead (`mesa-vulkan-intel`,
  `mesa-vulkan-ati`, `mesa-vulkan-nouveau`, …).
- Without Vulkan the app falls back to CPU rendering through Mesa's GL drivers. It works, but
  slowly — a driver limit, not a Local Sheets bug.
- `xdg-desktop-portal` (with `xdg-desktop-portal-gtk` or `-kde`; `-wlr` has no file chooser, so
  on wlroots compositors install `zenity`) is what the Open and Save dialogs talk to;
  `xkeyboard-config` holds the XKB keyboard data `libxkbcommon` reads.
- fontconfig and freetype are not needed: Local Sheets reads `/usr/share/fonts` itself.

Building from source on Linux needs `pkg-config libxkbcommon-dev libwayland-dev libx11-dev
libxcursor-dev libxrandr-dev libxi-dev libgtk-3-dev libxcb1-dev`.

### macOS, BSD and the web

There is no published macOS or BSD artifact yet: both build from source with `cargo build
--locked` (the DMG and FreeBSD packaging in `packaging/` is kept for when there are). The web build
is `trunk build --release` in `apps/localsheets-web`, hosted on any static server.

## The Local suite

Local Sheets is one of the **Local** apps: free, open-source, local-first tools, each written from
scratch in Rust and each able to stand on its own — no account, no telemetry, no server.

<br>

## License and credits

Local Sheets is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Copyright (c) 2026 the Local Sheets contributors. Required notices are in [NOTICE](NOTICE).

Bundled fonts, icons, images and other assets keep their own open licenses; each one is listed
with its author, source and license in [ATTRIBUTION.md](ATTRIBUTION.md).

Local Sheets is an independent fork of [GridCraft](https://github.com/storytold/gridcraft),
maintained by Anon5T4R in the Local suite. "GridCraft" and "ArtCraft" are names of the upstream
project and its team; this repository and its packages use only the Local Sheets name.
`UPSTREAM-SYNC.md` documents how changes are taken from the upstream and what stays local.

<p align="center">
  <sub>Suíte Local — apps locais, sem conta, sem telemetria.</sub>
</p>
