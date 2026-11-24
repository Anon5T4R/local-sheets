# Upstream sync (storytold/gridcraft)

The Local suite's port/rebrand: `main` is ours, the upstream is a source of candidate changes and
nothing enters without review. General rules in `docs/planos/licoes-de-porta.md` (dev-notes).

## Base

- Upstream: `storytold/gridcraft`, tag **v0.16.1** = commit **e581bc95** (`Release 0.16.1`,
  2026-11-04). The tree was copied without history in the port's first commit (`4032f89`, phases
  A+B).
- Checked in the 2026-11-21 sync: the imported tree is not exactly the diff of `e581bc95` — two
  commits from the `main` of Nov 3 already came in it (`065f05db` MODE by index/FREQUENCY by binary
  search and `db05400e` each dependent listed once), with code and tests. They are not reappliable;
  the base for the next syncs stays `e581bc95`.
- Fork: `Styx-Studios/Local-Sheets`, own version on the `0.2.x` line (a new numbering avoids a false
  auto-update to the upstream).
- Nothing is read from `C:\Dev\storytold` in build, test or release; the folder is only a local
  reference for the `sync-upstream` skill.
- `docs/UPSTREAM-MAP.toml` records the paths that differ, what can be applied by script and what
  requires review.

## Vendored (outside the normal sync)

| Path | Origin | Rev |
|---|---|---|
| `libs/craft-libs` | `storytold/craft-libs` | `c5a1cbf` (with `vendor/`) |
| `libs/craft-libs/crates/update` | **`Styx-Studios/local-libs`** (the suite's single source) | checked by `scripts/check-mirror.sh` |
| `libs/egui` | `storytold/egui` | `6d518fc` |
| `libs/winit` | `storytold/winit` | `a517b38` |
| `libs/winprint.rs` | `storytold/winprint.rs` | `417ac3c` |

`craft-update` is a **conscious permanent fork** and stays outside the sync: bugfixes go straight
into the source (`local-libs`) and are replicated with `sync.ps1 -Mode push` (hash guard; the copy
here is a mirror). Never edit `libs/craft-libs/crates/update` by hand.

## Rebrand divergences (what is ours)

- Identity: crates `localsheets-*`, binaries `localsheets`/`localsheets-cli`, app id, ProgIds and
  the MSI's UpgradeCode, registry keys and configuration folder of our own.
- XLSX stays the native format — the fork introduces no format of its own, so files and tools from
  the upstream stay compatible.
- Own CI: no `storytold` org workflows; release by tag `vX.Y.Z` on Actions.
- Packaging in the suite's mold (per-machine and per-user MSI, AppImage/deb/rpm/tar, PKGBUILD).
- The `craft-fonts` fonts go through the asset policy filter
  (`crates/ui-egui/src/craft_fonts_policy.rs`).
- The AI tab (BYOK) is ours: vendored `libs/local-ai` + crate `localsheets-ai`, key in the OS
  keychain.
- MCP/CLI: server `crates/mcp` (stdio + headless/`Remote` backends) and the shared crate
  `craft-mcp` on `--connect`.

## Sync log

The base stays `e581bc95`: only chosen points enter (skill `sync-upstream`), not a bump.

| Date | Target (gridcraft) | Taken | Relevant discards |
|---|---|---|---|
| 2026-11-21 | `origin/main` of 2026-11-21 (`11a5711a`); commits from 2026-11-03 to 2026-11-11 | `aa4951fd` → `304690e` (WORKDAY, PIVOTBY, SORT and the D functions stop rescanning the input); `bbe08254` → `f8ca170` (TEXTJOIN builds the result inside the text limit); `7cb00883`+`9ab297ce` → `47045bf` (structural edit rewrites only the formulas it changes, with the `adjust_oracle` and `structural_edit_rewrite` oracles); `35e98156` → `d6f0f32` (a single work budget for all the calls of a hoisted function); `67a0b7f3`+`bc162524` → `293e380` (rasterization, splines and category labels with a cap) | `065f05db` and `db05400e` were already in the base; from `bc162524`, the `needs_sanitizing` fast path (we always sanitize and the levels already have a cap), the `BarRun` patch (we don't have bar merging) and the moving-average one (we sum the window directly, without block sums) |

No item asked for a crate or API we don't have; the only prerequisite was `Formula::with_expr`
(from `b6bbdcd5`, which we don't have), brought along in item 3's commit.
